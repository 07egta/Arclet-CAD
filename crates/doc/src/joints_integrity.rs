//! Non-mutating integrity checks for existing assembly and joint data.
//!
//! Intended for commands, importers, inspectors and interference clients before
//! applying a solution. Legacy document deserialization and the default joint
//! solver keep their previous behavior; corrupt data is never silently repaired.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{DocError, Document, Result};

use super::MAX_JOINTS;

/// Validate stored component placements, joint references and motion couplings.
///
/// A failure means the graph cannot safely be driven as a single mechanism;
/// it does not alter the document or delete any legacy component data.
pub fn validate(doc: &Document) -> Result<()> {
    doc.validate_assembly_tree()?;
    let assembly = &doc.assembly;
    if assembly.joints.len() > MAX_JOINTS {
        return Err(DocError::Invalid("too many joints to solve safely".into()));
    }
    let occurrences: BTreeSet<u64> =
        std::iter::once(0).chain(doc.occurrences.iter().map(|occurrence| occurrence.id)).collect();
    let mut joints = BTreeMap::new();
    for joint in &assembly.joints {
        if joints.insert(joint.id, joint).is_some() {
            return Err(DocError::Invalid(format!("duplicate joint identifier {}", joint.id)));
        }
        for origin in [&joint.a, &joint.b] {
            if !occurrences.contains(&origin.occurrence) {
                return Err(DocError::Unknown(format!(
                    "joint '{}' refers to missing occurrence {}", joint.name, origin.occurrence
                )));
            }
            if !origin.snap.is_finite() {
                return Err(DocError::Invalid(format!("joint '{}' has a non-finite snap", joint.name)));
            }
        }
        if joint.a.occurrence == joint.b.occurrence {
            return Err(DocError::Invalid(format!("joint '{}' connects an occurrence to itself", joint.name)));
        }
        let n = joint.kind.dofs().len();
        if joint.values.len() > n || joint.limits.len() > n || joint.exprs.values.len() > n
            || !joint.values.iter().all(|value| value.is_finite())
            || !joint.offset.is_finite() || !joint.angle.is_finite()
            || joint.limits.iter().flatten().any(|(low, high)| !low.is_finite() || !high.is_finite())
        {
            return Err(DocError::Invalid(format!("joint '{}' has invalid values or limits", joint.name)));
        }
    }

    // Links form a directed graph between individual joint parameters.
    // Reject duplicate drivers and cycles instead of retaining an arbitrary
    // value after the solver's bounded fixed-point passes.
    type Port = (u64, usize);
    let mut driven = BTreeSet::<Port>::new();
    let mut adjacency = BTreeMap::<Port, Vec<Port>>::new();
    let mut indegree = BTreeMap::<Port, usize>::new();
    for link in &assembly.links {
        let a = (link.a, link.ia);
        let b = (link.b, link.ib);
        if joints.get(&link.a).is_none_or(|j| link.ia >= j.kind.dofs().len())
            || joints.get(&link.b).is_none_or(|j| link.ib >= j.kind.dofs().len())
        {
            return Err(DocError::Invalid(format!("motion link {:?} -> {:?} refers to a missing joint parameter", a, b)));
        }
        if !link.ratio.is_finite() || !link.offset.is_finite() || link.lift.iter().any(|v| !v.is_finite()) {
            return Err(DocError::Invalid(format!("motion link {:?} -> {:?} has non-finite values", a, b)));
        }
        if !driven.insert(b) {
            return Err(DocError::Invalid(format!("motion parameter {:?} has multiple drivers", b)));
        }
        indegree.entry(a).or_insert(0);
        *indegree.entry(b).or_insert(0) += 1;
        adjacency.entry(a).or_default().push(b);
    }
    let mut ready: VecDeque<Port> = indegree.iter().filter_map(|(port, n)| (*n == 0).then_some(*port)).collect();
    let mut visited = 0usize;
    while let Some(port) = ready.pop_front() {
        visited += 1;
        for child in adjacency.get(&port).into_iter().flatten() {
            if let Some(n) = indegree.get_mut(child) {
                *n -= 1;
                if *n == 0 {
                    ready.push_back(*child);
                }
            }
        }
    }
    if visited != indegree.len() {
        return Err(DocError::Invalid("motion links contain a cyclic dependency".into()));
    }

    for study in &assembly.studies {
        study.validate(assembly)?;
    }
    Ok(())
}

impl Document {
    /// Strict opt-in mechanism integrity check; performs no evaluation or mutation.
    pub fn validate_mechanism(&self) -> Result<()> {
        validate(self)
    }
}
