//! References from a feature to the rest of the design: the sketches it is built from (by id),
//! the features it names (patterned and mirrored features and the construction plane a plane is
//! built on, by id since #1134; construction axes and points, still by name) and the bodies it
//! works on (by name). The walk (`crate::walk`) is behind
//! `input_sketches`, `remap_refs` and `remap_bodies`.

use std::collections::BTreeMap;

use crate::walk::Walk;
use crate::{BodyName, FeatureId, FeatureKind, FeatureName, SketchId};

/// A reference a feature holds: `S` is `&SketchId` or `&mut SketchId`, and so on. `Feature`
/// carries a feature id (pattern and mirror seeds, construction planes); `FeatureName` carries
/// a feature name, for the references that are still keyed by name (construction axes and
/// points).
pub(crate) enum Ref<S, F, N, B> {
    Sketch(S),
    Feature(F),
    FeatureName(N),
    Body(B),
}

impl FeatureKind {
    /// Sketches the feature is built from (profiles, paths, points): deleting one of them
    /// deletes the feature.
    pub fn input_sketches(&self) -> Vec<u64> {
        let mut v = Vec::new();
        self.walk_refs(&mut |r| {
            if let Ref::Sketch(id) = r {
                v.push(id.0);
            }
        });
        v
    }

    /// Whether the feature refers to the feature `id` (by id: a patterned or mirrored feature,
    /// the construction plane a plane is built on) or names it (`name`: a construction axis or
    /// point it is built from). Used to refuse a feature built on itself (#3567).
    pub fn names_feature(&self, id: u64, name: &str) -> bool {
        let mut found = false;
        self.walk_refs(&mut |r| match r {
            Ref::Feature(x) => found |= x.0 == id,
            Ref::FeatureName(n) => found |= n.as_ref() == name,
            _ => {}
        });
        found
    }

    /// Point references to sketches and features (both by id) at their copies, and references
    /// that are still keyed by name at a feature's new name (`timeline.rename`).
    pub fn remap_refs(&mut self, ids: &BTreeMap<u64, u64>, names: &BTreeMap<String, String>) {
        self.walk_refs_mut(&mut |r| match r {
            Ref::Sketch(id) => {
                if let Some(n) = ids.get(&id.0) {
                    *id = SketchId(*n);
                }
            }
            Ref::Feature(id) => {
                if let Some(n) = ids.get(&id.0) {
                    *id = FeatureId(*n);
                }
            }
            Ref::FeatureName(name) => {
                if let Some(n) = names.get(name.as_ref()) {
                    *name = FeatureName(n.clone());
                }
            }
            Ref::Body(_) => {}
        });
    }

    /// Point references to bodies (by name) at their copies.
    pub fn remap_bodies(&mut self, bodies: &BTreeMap<String, String>) {
        self.walk_refs_mut(&mut |r| {
            if let Ref::Body(name) = r
                && let Some(n) = bodies.get(name.as_ref())
            {
                *name = BodyName(n.clone());
            }
        });
    }
}

impl crate::Document {
    /// Rename feature `old` to `new` where the design names it outside the features' own
    /// references (`remap_refs`): sketch projections of a construction plane (#4353). The
    /// features are the rename command's to change; feature references inside features are keyed
    /// by id since #1134 and follow a rename on their own.
    pub fn rename_feature_keys(&mut self, old: &str, new: &str) {
        let names = BTreeMap::from([(old.to_string(), new.to_string())]);
        let (ids, bodies) = (BTreeMap::new(), BTreeMap::new());
        for f in &mut self.features {
            if let FeatureKind::Sketch { sketch, .. } = &mut f.kind {
                for link in &mut sketch.links {
                    crate::clipboard::remap_link(&mut link.source, &ids, &names, &bodies, 0);
                }
            }
        }
    }
}


/// An explicit dependency problem found without evaluating geometry. The feature id and
/// target id remain stable across timeline reordering, suppression and save/reload.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct DependencyIssue {
    /// Feature whose reference is affected.
    pub feature: u64,
    /// Referenced feature, when one can be identified.
    pub target: Option<u64>,
    /// Machine-readable diagnosis (missing_sketch, future_sketch, cycle, etc.).
    pub code: &'static str,
    pub message: String,
    /// Whether the feature cannot be safely evaluated from the current timeline.
    pub blocking: bool,
}

/// Dependencies explicitly described by feature IDs, named construction entities and sketch
/// projection links. Implicit body/face picks are still validated by the geometric evaluator:
/// a name on a body is not proof that a particular feature produced that body.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct FeatureDependencyGraph {
    /// Feature id -> direct source feature ids (sorted, deduplicated).
    pub upstream: BTreeMap<u64, Vec<u64>>,
    /// Missing, suppressed, forward and cyclic references. Soft diagnoses do not break
    /// documents that historically allowed forward pattern seeds.
    pub issues: Vec<DependencyIssue>,
    /// Active directed cycles, each ending in its starting feature id.
    pub cycles: Vec<Vec<u64>>,
}

impl FeatureDependencyGraph {
    /// Blocking diagnosis for a feature, if any. The evaluator uses this *before* taking a
    /// previous result or a cached one, to prevent an invalid dependency from being concealed.
    pub fn blocking_error(&self, feature: u64) -> Option<&str> {
        self.issues.iter().find(|i| i.feature == feature && i.blocking).map(|i| i.message.as_str())
    }

    /// All explicit downstream dependants, in id order. Iterative: a 10,000-feature history
    /// cannot overflow the thread stack, including a malformed cyclic history.
    pub fn dependents(&self, feature: u64) -> Vec<u64> {
        use std::collections::BTreeSet;
        let mut reverse: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for (child, parents) in &self.upstream {
            for parent in parents {
                reverse.entry(*parent).or_default().push(*child);
            }
        }
        let mut found = BTreeSet::new();
        let mut todo = vec![feature];
        while let Some(parent) = todo.pop() {
            for child in reverse.get(&parent).into_iter().flatten() {
                if *child != feature && found.insert(*child) {
                    todo.push(*child);
                }
            }
        }
        found.into_iter().collect()
    }
}

/// Capture a sketch link's cross-sketch references, including nested intersections.
/// A bounded iterative walk does not recurse into attacker-controlled saved link trees.
fn projected_sketches(
    source: &solidcraft_sketch::LinkSource,
    refs: &mut Vec<(u64, bool, &'static str)>,
) -> bool {
    use solidcraft_sketch::LinkSource as L;
    let mut stack = vec![source];
    let mut visited = 0;
    while let Some(source) = stack.pop() {
        visited += 1;
        if visited > 65_536 {
            return false;
        }
        match source {
            L::SketchCurve { sketch, .. } | L::SketchPoint { sketch, .. } | L::OnSurface { sketch, .. } => {
                refs.push((*sketch, true, "projected sketch"));
            }
            L::Intersection { a, b } => {
                stack.push(a);
                stack.push(b);
            }
            L::Edge { .. } | L::Face { .. } | L::Body { .. } | L::Vertex { .. }
            | L::Origin | L::Axis { .. } | L::SketchAxis { .. } | L::Plane { .. }
            | L::Iso { .. } | L::Spun { .. } | L::Text { .. } => {}
        }
    }
    true
}

/// Find cycles without recursion. A back-edge identifies the actual involved feature IDs,
/// rather than reporting a generic "maximum pattern depth" after replay has already begun.
fn dependency_cycles(edges: &BTreeMap<u64, Vec<u64>>) -> Vec<Vec<u64>> {
    use std::collections::BTreeSet;
    let mut color = BTreeMap::<u64, u8>::new();
    let mut found = BTreeSet::<Vec<u64>>::new();
    for start in edges.keys() {
        if color.contains_key(start) {
            continue;
        }
        color.insert(*start, 1);
        let mut stack = vec![(*start, 0usize)];
        while let Some((id, next)) = stack.last().copied() {
            let neighbours = edges.get(&id).map(Vec::as_slice).unwrap_or_default();
            if next >= neighbours.len() {
                color.insert(id, 2);
                stack.pop();
                continue;
            }
            if let Some(tail) = stack.last_mut() {
                tail.1 += 1;
            }
            let target = neighbours[next];
            match color.get(&target).copied() {
                Some(1) => {
                    if let Some(index) = stack.iter().position(|(node, _)| *node == target) {
                        let mut cycle: Vec<u64> = stack[index..].iter().map(|(node, _)| *node).collect();
                        cycle.push(target);
                        found.insert(cycle);
                    }
                }
                Some(2) => {}
                _ if edges.contains_key(&target) => {
                    color.insert(target, 1);
                    stack.push((target, 0));
                }
                _ => {}
            }
        }
    }
    found.into_iter().collect()
}

impl crate::Document {
    /// Read-only structural diagnosis. No history changes, generated geometry, new IDs or
    /// persisted fields; safe to call before/after a dimension edit, suppression or reordering.
    /// Pattern/mirror seeds are permitted to refer forward for historical compatibility, but
    /// cycles and dangling IDs are not. Body/face dependencies require an evaluated ModelState.
    pub fn feature_dependency_graph(&self) -> FeatureDependencyGraph {
        use std::collections::BTreeSet;

        let mut graph = FeatureDependencyGraph::default();
        let mut positions = BTreeMap::<u64, usize>::new();
        let mut names = BTreeMap::<String, Vec<u64>>::new();
        for (index, feature) in self.features.iter().enumerate() {
            positions.entry(feature.id).or_insert(index);
            if matches!(feature.kind, FeatureKind::ConstructionAxis { .. } | FeatureKind::ConstructionPoint { .. }) {
                names.entry(feature.name.to_lowercase()).or_default().push(feature.id);
            }
            graph.upstream.entry(feature.id).or_default();
        }

        for (index, feature) in self.features.iter().enumerate() {
            // ref type: (id, must be an earlier sketch, where the reference came from).
            let mut refs = Vec::<(u64, bool, &'static str)>::new();
            let mut named_refs = Vec::<String>::new();
            feature.kind.walk_refs(&mut |reference| match reference {
                Ref::Sketch(id) => refs.push((id.0, true, "sketch")),
                Ref::Feature(id) => refs.push((id.0, false, "feature")),
                Ref::FeatureName(name) => named_refs.push(name.as_ref().to_string()),
                Ref::Body(_) => {},
            });
            if let FeatureKind::Sketch { sketch, plane } = &feature.kind {
                for link in &sketch.links {
                    if !projected_sketches(&link.source, &mut refs) {
                        graph.issues.push(DependencyIssue {
                            feature: feature.id, target: None, code: "projection_depth",
                            message: "sketch projection link nesting exceeds the safe limit".into(),
                            blocking: true,
                        });
                        break;
                    }
                }
                if let crate::PlaneRef::Face { anchor: Some(anchor), .. } = plane {
                    refs.push((anchor.target.origin_feature, false, "certified face origin"));
                }
            }
            for name in named_refs {
                match names.get(&name.to_lowercase()) {
                    Some(ids) if ids.len() == 1 => refs.push((ids[0], false, "construction entity")),
                    Some(ids) => graph.issues.push(DependencyIssue {
                        feature: feature.id, target: None, code: "ambiguous_construction",
                        message: format!("construction reference '{name}' matches {} features: {ids:?}", ids.len()),
                        blocking: false,
                    }),
                    None => graph.issues.push(DependencyIssue {
                        feature: feature.id, target: None, code: "missing_construction",
                        message: format!("construction reference '{name}' does not identify a construction feature"),
                        blocking: false,
                    }),
                }
            }
            let mut direct = BTreeSet::new();
            for (target, sketch_ref, source) in refs {
                let Some(&upstream) = positions.get(&target) else {
                    graph.issues.push(DependencyIssue {
                        feature: feature.id, target: Some(target),
                        code: if sketch_ref { "missing_sketch" } else { "missing_feature" },
                        message: format!("feature '{}' (F{}): {source} F{target} does not exist", feature.name, feature.id),
                        blocking: true,
                    });
                    continue;
                };
                direct.insert(target);
                let input = &self.features[upstream];
                if target == feature.id {
                    // The cycle detector reports the complete path below.
                    continue;
                }
                if sketch_ref && !matches!(input.kind, FeatureKind::Sketch { .. }) {
                    graph.issues.push(DependencyIssue {
                        feature: feature.id, target: Some(target), code: "not_a_sketch",
                        message: format!("feature '{}' (F{}): {source} F{target} is not a sketch", feature.name, feature.id),
                        blocking: true,
                    });
                }
                if upstream > index {
                    graph.issues.push(DependencyIssue {
                        feature: feature.id, target: Some(target),
                        code: if sketch_ref { "future_sketch" } else { "forward_reference" },
                        message: format!("feature '{}' (F{}): {source} F{target} comes after its consumer in the timeline", feature.name, feature.id),
                        // Forward pattern/mirror seeds have long been supported.
                        blocking: sketch_ref || source == "certified face origin",
                    });
                }
                if input.suppressed {
                    graph.issues.push(DependencyIssue {
                        feature: feature.id, target: Some(target), code: "suppressed_source",
                        message: format!("feature '{}' (F{}): {source} F{target} is suppressed", feature.name, feature.id),
                        // Suppressed pattern seeds intentionally copy nothing; sketch geometry
                        // and certified face attachments, however, cannot be resolved.
                        blocking: sketch_ref || source == "certified face origin",
                    });
                }
            }
            if let Some(entry) = graph.upstream.get_mut(&feature.id) {
                entry.extend(direct);
            }
        }

        // A suppressed feature cannot initiate active evaluation or a dependency cycle.
        // A consumer of a suppressed sketch still receives the explicit error above.
        let active: BTreeMap<u64, Vec<u64>> = self.features.iter().filter(|f| !f.suppressed).map(|f| {
            let parents = graph.upstream.get(&f.id).into_iter().flatten().copied().filter(|id| {
                positions.get(id).is_some_and(|index| !self.features[*index].suppressed)
            }).collect();
            (f.id, parents)
        }).collect();
        graph.cycles = dependency_cycles(&active);
        let mut reported = BTreeSet::new();
        for cycle in &graph.cycles {
            let summary = cycle.iter().take(16).map(|id| format!("F{id}")).collect::<Vec<_>>().join(" -> ");
            let tail = if cycle.len() > 16 { " -> ..." } else { "" };
            for &feature in cycle.iter().take(cycle.len().saturating_sub(1)) {
                if reported.insert(feature) {
                    graph.issues.push(DependencyIssue {
                        feature, target: Some(feature), code: "cycle",
                        message: format!("cyclic feature dependency: {summary}{tail}"),
                        blocking: true,
                    });
                }
            }
        }
        graph
    }
}
