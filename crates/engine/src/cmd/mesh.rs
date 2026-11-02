//! Parametric reconstruction and inspection of imported mechanical mesh bodies.

use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};
use solidcraft_doc::FeatureKind;
use solidcraft_doc::expr::Kind;
use solidcraft_geom::{Aabb3, Vec3};

#[path = "mesh_manufacturing.rs"]
mod manufacturing;

use super::{
    CommandSpec,
    features::{add_feature, check_expr},
};
use crate::{
    Result, Sel, Session,
    params::{bad, expr, str_},
};

/// The largest welding distance `mesh.check` takes (mm).
const MAX_CHECK_TOLERANCE: f64 = 1e6;
/// More than this many faces: vertex-link topology is not certified.
const MAX_VERTEX_LINK_TRIANGLES: usize = 500_000;

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("mesh.convert", "Convert Mesh", convert)
        .icon("import")
        .params("body?: mesh body name (default: the selected mesh body); tolerance?: length expression (default 0.05 mm): reconstruct a closed constant-section mechanical mesh as editable planar and cylindrical faces, with a parametric timeline feature in the mesh body's owning component (the active component must be that component or an ancestor; otherwise activate the named component first); the active component is unchanged; name?"),
    CommandSpec::new("mesh.check", "Check Mesh", check)
        .icon("inspect")
        .noundo()
        .params("body?: imported mesh (default selected); tolerance?: length (default 0 mm, exact-position welding). Reports invalid indices, nonfinite vertices, zero-area/collapsed triangles, repeated faces, naked/nonmanifold edges, winding conflicts, boundary loops and signed volume. closed is true only for a valid outward-oriented closed mesh; this does not prove absence of self-intersections."),
    CommandSpec::new("mesh.manufacturing_check", "Manufacturing Mesh Check", manufacturing::check)
        .icon("inspect")
        .noundo()
        .params("body?: named solid or mesh; weld_tolerance_mm?: number (default export weld: max(body_size*1e-6, 1e-7) mm); chord_mm?: B-rep tessellation tolerance (default standard STL/3MF: max(size*0.0002, 0.001) mm); nozzle_diameter_mm?: number (default 0.4); overhang_deg?: number (default 45); bed_size_mm?: [width,depth,height] (optional). Reports manifold/orientation/duplicate/invalid checks, bounded potential self-intersections, bounds, volume, bed fit, overhang screening, build-plate contact and thin-feature warning. No wall-thickness certification, no printer compatibility guarantee."),
];

/// The mesh body to inspect: `body`, else the one selected body.
fn target(s: &Session, p: &Value, cmd: &str) -> Result<String> {
    if let Some(name) = str_(p, "body").filter(|b| !b.trim().is_empty()) {
        return Ok(name.to_owned());
    }
    let selected: Vec<&str> = s.selection.iter().filter_map(|x| if let Sel::Body { name } = x { Some(name.as_str()) } else { None }).collect();
    match selected.as_slice() {
        [name] => Ok((*name).to_owned()),
        [] => Err(bad(cmd, "select a mesh body")),
        _ => Err(bad(cmd, "select one mesh body")),
    }
}

fn convert(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "mesh.convert";
    let state = s.model.state();
    let body = target(s, p, cmd)?;
    let source = state.body(&body).ok_or_else(|| bad(cmd, format!("no body `{body}`")))?;
    if !source.body.is_mesh() {
        return Err(bad(cmd, "select a mesh body"));
    }
    let owner = s.doc.model_body_component(source);
    if !s.doc.component_within(owner, s.active_component) {
        let name = s.doc.components.iter().find(|c| c.id == owner).map(|c| c.name.as_str()).unwrap_or(s.doc.name.as_str());
        return Err(bad(cmd, format!("activate component '{name}' first")));
    }
    let tolerance = expr(p, "tolerance").unwrap_or_else(|| "0.05 mm".into());
    check_expr(s, &tolerance, Kind::Length, cmd, "tolerance")?;
    // Conversion edits the component definition, independent of its occurrence placement.
    let active_component = std::mem::replace(&mut s.active_component, owner);
    let active_occurrence_path = s.active_occurrence_path.take();
    let result = add_feature(s, p, FeatureKind::MeshConvert { body: body.into(), tolerance });
    s.active_component = active_component;
    s.active_occurrence_path = active_occurrence_path;
    result
}

fn check(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "mesh.check";
    let tolerance = match p.get("tolerance") {
        None | Some(Value::Null) => "0.0 mm".into(),
        Some(_) => expr(p, "tolerance").ok_or_else(|| bad(cmd, "`tolerance` must be a length or an expression"))?,
    };
    let tolerance = s.doc.eval(&tolerance, Kind::Length).map_err(|e| bad(cmd, format!("tolerance: {e}")))?;
    if !(tolerance.is_finite() && (0.0..=MAX_CHECK_TOLERANCE).contains(&tolerance)) {
        return Err(bad(cmd, format!("`tolerance` must be a length from 0 to {MAX_CHECK_TOLERANCE} mm")));
    }
    let state = s.model.state();
    let body = target(s, p, cmd)?;
    let source = state.body(&body).ok_or_else(|| bad(cmd, format!("no body `{body}`")))?;
    if !source.body.is_mesh() {
        return Err(bad(cmd, format!("`{body}` is not a mesh body: mesh.check reports imported triangle meshes")));
    }
    let (positions, triangles) = source.body.triangle_mesh().ok_or_else(|| bad(cmd, format!("`{body}` has no triangles")))?;
    let mut report = analyze(positions, triangles, tolerance);
    if let Some(fields) = report.as_object_mut() {
        fields.insert("body".into(), Value::String(body));
        fields.insert("tolerance_mm".into(), json!(tolerance));
    }
    Ok(report)
}


/// Mesh topology classification is deliberately independent of the display
/// tessellation and material settings: callers can refuse invalid export meshes.
/// The integrity report of a triangle mesh. Pure, read-only, and bounded by the two slices: a
/// triangle that names a vertex that is not there is counted as degenerate and kept out of the
/// topology, so no index is ever used unchecked.
pub(crate) fn analyze(positions: &[Vec3], triangles: &[[u32; 3]], tolerance: f64) -> Value {
    let (welded, unique, duplicate_vertices) = weld_vertices(positions, tolerance);
    let nonfinite_vertices = positions.iter().filter(|p| !p.is_finite()).count();
    let bounds: Aabb3 = positions.iter().copied().collect();
    let origin = bounds.center();

    // Undirected incidence and net winding on each edge. A count of two alone is
    // insufficient: two triangles can traverse a shared edge in the same direction.
    let mut edges: HashMap<(usize, usize), (u32, i32)> = HashMap::new();
    let mut faces: HashSet<[usize; 3]> = HashSet::new();
    let mut valid_faces = Vec::new();
    let (mut degenerate_triangles, mut invalid_index_triangles, mut nonfinite_triangles) = (0usize, 0usize, 0usize);
    let (mut zero_area_triangles, mut collapsed_after_weld, mut duplicate_faces) = (0usize, 0usize, 0usize);
    let (mut area_mm2, mut six_volume_mm3) = (0.0, 0.0);

    for triangle in triangles {
        let [a, b, c] = *triangle;
        let (Some(&pa), Some(&pb), Some(&pc)) =
            (positions.get(a as usize), positions.get(b as usize), positions.get(c as usize))
        else {
            invalid_index_triangles += 1;
            degenerate_triangles += 1;
            continue;
        };
        if !(pa.is_finite() && pb.is_finite() && pc.is_finite()) {
            nonfinite_triangles += 1;
            degenerate_triangles += 1;
            continue;
        }
        if a == b || b == c || c == a || flat(pa, pb, pc) {
            zero_area_triangles += 1;
            degenerate_triangles += 1;
            continue;
        }
        let (Some(&wa), Some(&wb), Some(&wc)) =
            (welded.get(a as usize), welded.get(b as usize), welded.get(c as usize))
        else {
            degenerate_triangles += 1;
            continue;
        };
        if wa == wb || wb == wc || wc == wa {
            // A nonzero input triangle collapsed under the requested weld radius.
            collapsed_after_weld += 1;
            degenerate_triangles += 1;
            continue;
        }
        let mut ordered = [wa, wb, wc];
        ordered.sort_unstable();
        if !faces.insert(ordered) {
            duplicate_faces += 1;
        }
        if triangles.len() <= MAX_VERTEX_LINK_TRIANGLES {
            valid_faces.push([wa, wb, wc]);
        }
        for (u, v) in [(wa, wb), (wb, wc), (wc, wa)] {
            let (key, dir) = if u < v { ((u, v), 1) } else { ((v, u), -1) };
            let entry = edges.entry(key).or_insert((0, 0));
            entry.0 = entry.0.saturating_add(1);
            entry.1 += dir;
        }
        let cross = (pb - pa).cross(pc - pa);
        area_mm2 += cross.len() * 0.5;
        six_volume_mm3 += (pa - origin).dot((pb - origin).cross(pc - origin));
    }
    let (mut naked_edges, mut non_manifold_edges, mut winding_conflicts) = (0usize, 0usize, 0usize);
    let mut incidence: HashMap<(usize, usize), u32> = HashMap::with_capacity(edges.len());
    for (edge, &(count, winding)) in &edges {
        incidence.insert(*edge, count);
        match count {
            1 => naked_edges += 1,
            2 if winding != 0 => winding_conflicts += 1,
            2 => {}
            _ => non_manifold_edges += 1,
        }
    }
    let non_manifold_vertices = (triangles.len() <= MAX_VERTEX_LINK_TRIANGLES)
        .then(|| count_non_manifold_vertices(&valid_faces));
    let connected_shells = (triangles.len() <= MAX_VERTEX_LINK_TRIANGLES)
        .then(|| count_connected_shells(&valid_faces));
    let topology_closed =
        !triangles.is_empty() && degenerate_triangles == 0 && nonfinite_vertices == 0
        && duplicate_faces == 0 && naked_edges == 0 && non_manifold_edges == 0
        && non_manifold_vertices == Some(0);
    let consistently_oriented = topology_closed && winding_conflicts == 0;
    // Relative to a nearby origin to avoid catastrophic cancellation when a
    // small printable part lies far from the world origin.
    let signed_volume = if consistently_oriented && six_volume_mm3.is_finite() {
        Some(six_volume_mm3 / 6.0)
    } else {
        None
    };
    let outward = signed_volume.is_some_and(|v| v > 0.0);
    let closed = consistently_oriented && outward && area_mm2.is_finite();
    json!({
        "vertices": positions.len(),
        "triangles": triangles.len(),
        "degenerate_triangles": degenerate_triangles,
        "invalid_index_triangles": invalid_index_triangles,
        "nonfinite_vertices": nonfinite_vertices,
        "nonfinite_triangles": nonfinite_triangles,
        "zero_area_triangles": zero_area_triangles,
        "collapsed_after_weld": collapsed_after_weld,
        "duplicate_vertices": duplicate_vertices,
        "duplicate_faces": duplicate_faces,
        "naked_edges": naked_edges,
        "non_manifold_edges": non_manifold_edges,
        "non_manifold_vertices": non_manifold_vertices,
        "vertex_manifold_complete": non_manifold_vertices.is_some(),
        "connected_shells": connected_shells,
        "shell_nesting_checked": false,
        "winding_conflicts": winding_conflicts,
        "boundary_loops": boundary_loops(&incidence, unique),
        "topologically_closed": topology_closed,
        "consistently_oriented": consistently_oriented,
        "outward_oriented": outward,
        "signed_volume_mm3": signed_volume,
        "surface_area_mm2": area_mm2.is_finite().then_some(area_mm2),
        "closed": closed,
    })
}

/// Connected components by triangle vertices, after optional welding. Distinct
/// nested surfaces (a hollow sealed case) remain distinct shells. Their relative
/// containment/orientation is not proved just by a positive total signed volume.
fn count_connected_shells(triangles: &[[usize; 3]]) -> usize {
    let mut id: HashMap<usize, usize> = HashMap::new();
    let mut parents = Vec::new();
    for tri in triangles {
        let mut ids = [0usize; 3];
        for (slot, vertex) in ids.iter_mut().zip(tri) {
            *slot = *id.entry(*vertex).or_insert_with(|| {
                let n = parents.len();
                parents.push(n);
                n
            });
        }
        for &vertex in ids.iter().skip(1) {
            let a = root(&mut parents, ids[0]);
            let b = root(&mut parents, vertex);
            if let Some(parent) = parents.get_mut(b) {
                *parent = a;
            }
        }
    }
    let mut representatives = HashSet::new();
    for i in 0..parents.len() {
        representatives.insert(root(&mut parents, i));
    }
    representatives.len()
}

/// The link of every vertex in an orientable closed triangle surface is one
/// single cycle. Two otherwise valid closed shells touching at only one welded
/// vertex have no non-manifold *edges*, but the shared vertex has two disconnected
/// cycles and cannot be treated as a printable two-manifold solid.
///
/// Also counts open or branching vertex fans (the edge diagnostics report
/// their separate root causes). This is linear in the number of triangle
/// corners, with a local connectivity search per vertex.
fn count_non_manifold_vertices(triangles: &[[usize; 3]]) -> usize {
    // Allocate only for vertices actually used by triangles. Imported meshes
    // can contain many unreferenced positions with very large welded IDs.
    let mut links: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
    for &[a, b, c] in triangles {
        links.entry(a).or_default().push((b, c));
        links.entry(b).or_default().push((c, a));
        links.entry(c).or_default().push((a, b));
    }
    let mut invalid = 0usize;
    for wedges in links.into_values() {
        let mut neighbours: HashMap<usize, Vec<usize>> = HashMap::new();
        for (a, b) in wedges {
            neighbours.entry(a).or_default().push(b);
            neighbours.entry(b).or_default().push(a);
        }
        if neighbours.values().any(|v| v.len() != 2) {
            invalid += 1;
            continue;
        }
        let Some(first) = neighbours.keys().next().copied() else { continue };
        let mut visited = HashSet::new();
        let mut pending = vec![first];
        while let Some(v) = pending.pop() {
            if visited.insert(v) {
                pending.extend(neighbours.get(&v).into_iter().flatten().copied());
            }
        }
        if visited.len() != neighbours.len() {
            invalid += 1;
        }
    }
    invalid
}

/// A triangle with no area: three collinear points (or three at the same place), by the same
/// relative test the importer uses to drop them.
fn flat(a: Vec3, b: Vec3, c: Vec3) -> bool {
    let (ab, ac, bc) = (b - a, c - a, c - b);
    let longest = ab.len2().max(ac.len2()).max(bc.len2());
    ab.cross(ac).len() <= 1e-12 * longest
}

/// A weld id per vertex (equal ids: one vertex within the tolerance), the number of distinct
/// positions, and how many vertices repeat an earlier one. A tolerance of 0 keys positions by
/// their exact bits, so only the vertex indices decide the topology then.
fn weld_vertices(positions: &[Vec3], tolerance: f64) -> (Vec<usize>, usize, usize) {
    let mut welded = vec![0usize; positions.len()];
    if !(tolerance.is_finite() && tolerance > 0.0) {
        let mut seen: HashMap<[u64; 3], usize> = HashMap::new();
        let mut unique = 0usize;
        let mut duplicates = 0usize;
        for (i, p) in positions.iter().enumerate() {
            let key = [(p.x + 0.0).to_bits(), (p.y + 0.0).to_bits(), (p.z + 0.0).to_bits()];
            match seen.get(&key) {
                Some(&id) => {
                    welded[i] = id;
                    duplicates += 1;
                }
                None => {
                    seen.insert(key, unique);
                    welded[i] = unique;
                    unique += 1;
                }
            }
        }
        return (welded, unique, duplicates);
    }
    let tolerance2 = tolerance * tolerance;
    // One representative list per cell. With the cell as wide as the tolerance, every
    // representative in a cell is more than the tolerance from the others, so a cell holds only
    // a few however many vertices there are: the work stays linear in the mesh size.
    let mut cells: HashMap<(i64, i64, i64), Vec<(usize, Vec3)>> = HashMap::new();
    let mut unique = 0usize;
    let mut duplicates = 0usize;
    for (i, &p) in positions.iter().enumerate() {
        let found = cell_key(p, tolerance).and_then(|(cx, cy, cz)| {
            let mut found = None;
            'search: for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let key = (cx.saturating_add(dx), cy.saturating_add(dy), cz.saturating_add(dz));
                        for &(id, q) in cells.get(&key).into_iter().flatten() {
                            if (q - p).len2() <= tolerance2 {
                                found = Some(id);
                                break 'search;
                            }
                        }
                    }
                }
            }
            found
        });
        if let Some(id) = found {
            welded[i] = id;
            duplicates += 1;
        } else {
            welded[i] = unique;
            if let Some(key) = cell_key(p, tolerance) {
                cells.entry(key).or_default().push((unique, p));
            }
            unique += 1;
        }
    }
    (welded, unique, duplicates)
}

/// The grid cell a point falls in, at a cell size of `cell`; `None` when a coordinate is too
/// large for the grid (such a vertex is then never welded, which is safe).
fn cell_key(p: Vec3, cell: f64) -> Option<(i64, i64, i64)> {
    fn one(v: f64, cell: f64) -> Option<i64> {
        let c = (v / cell).floor();
        (c.is_finite() && (-9.0e18..=9.0e18).contains(&c)).then_some(c as i64)
    }
    Some((one(p.x, cell)?, one(p.y, cell)?, one(p.z, cell)?))
}

/// The connected pieces of the naked edges: each is one boundary loop of the mesh.
fn boundary_loops(edges: &HashMap<(usize, usize), u32>, vertices: usize) -> usize {
    let mut parent: Vec<usize> = (0..vertices).collect();
    let mut naked = false;
    for (&(a, b), &n) in edges {
        if n != 1 {
            continue;
        }
        naked = true;
        let ra = root(&mut parent, a);
        let rb = root(&mut parent, b);
        if let Some(slot) = parent.get_mut(rb) {
            *slot = ra;
        }
    }
    if !naked {
        return 0;
    }
    let mut roots: HashSet<usize> = HashSet::new();
    for (&(a, b), &n) in edges {
        if n == 1 {
            roots.insert(root(&mut parent, a));
            roots.insert(root(&mut parent, b));
        }
    }
    roots.len()
}

/// A disjoint-set root, without recursion (a boundary of a huge mesh can be one long chain).
fn root(parent: &mut [usize], mut x: usize) -> usize {
    loop {
        let Some(&p) = parent.get(x) else { return x };
        if p == x {
            return x;
        }
        let up = parent.get(p).copied().unwrap_or(p);
        if let Some(slot) = parent.get_mut(x) {
            *slot = up;
        }
        x = up;
    }
}

#[cfg(test)]
mod manufacturing_quality_tests {
    use super::*;
    use solidcraft_geom::{Aabb3, Mesh};

    /// Indexed, outward-oriented 20 mm cube (12 triangles, 8 vertices).
    fn box_mesh(width: f64, depth: f64, height: f64) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(width, 0.0, 0.0),
            Vec3::new(width, depth, 0.0),
            Vec3::new(0.0, depth, 0.0),
            Vec3::new(0.0, 0.0, height),
            Vec3::new(width, 0.0, height),
            Vec3::new(width, depth, height),
            Vec3::new(0.0, depth, height),
        ];
        let faces = vec![
            [0, 2, 1], [0, 3, 2], // bottom -Z
            [4, 5, 6], [4, 6, 7], // top +Z
            [0, 1, 5], [0, 5, 4], // front -Y
            [3, 6, 2], [3, 7, 6], // back +Y
            [0, 4, 7], [0, 7, 3], // left -X
            [1, 2, 6], [1, 6, 5], // right +X
        ];
        (points, faces)
    }

    fn mm(report: &Value, field: &str, expected: f64, absolute_tolerance: f64) {
        let got = report[field].as_f64().unwrap_or(f64::NAN);
        assert!((got - expected).abs() <= absolute_tolerance,
            "{field}: {got} against {expected} ± {absolute_tolerance}: {report}");
    }

    #[test]
    fn exact_cube_is_closed_outward_and_has_correct_volume_area_mm() {
        let (positions, triangles) = box_mesh(20.0, 20.0, 20.0);
        let r = analyze(&positions, &triangles, 0.0);
        assert_eq!(r["closed"], true, "{r}");
        assert_eq!(r["topologically_closed"], true);
        assert_eq!(r["consistently_oriented"], true);
        assert_eq!(r["duplicate_faces"], 0);
        assert_eq!(r["winding_conflicts"], 0);
        mm(&r, "signed_volume_mm3", 8_000.0, 1e-8);
        mm(&r, "surface_area_mm2", 2_400.0, 1e-8);
    }

    #[test]
    fn reversed_triangles_are_not_closed_even_with_two_incident_faces_per_edge() {
        let (positions, mut triangles) = box_mesh(20.0, 20.0, 20.0);
        triangles[0].swap(1, 2);
        let r = analyze(&positions, &triangles, 0.0);
        assert_eq!(r["topologically_closed"], true, "{r}");
        assert_eq!(r["consistently_oriented"], false, "{r}");
        assert!(r["winding_conflicts"].as_u64().unwrap_or(0) > 0);
        assert_eq!(r["closed"], false);
        for t in &mut triangles {
            t.swap(1, 2);
        }
        // The original reversed face becomes right-handed again, while
        // all other faces point inward; the winding conflict remains.
        assert_eq!(analyze(&positions, &triangles, 0.0)["closed"], false);
        let (positions, mut all_inward) = box_mesh(20.0, 20.0, 20.0);
        for t in &mut all_inward { t.swap(1, 2); }
        let inner = analyze(&positions, &all_inward, 0.0);
        assert_eq!(inner["consistently_oriented"], true);
        assert_eq!(inner["outward_oriented"], false);
        mm(&inner, "signed_volume_mm3", -8_000.0, 1e-8);
        assert_eq!(inner["closed"], false);
    }

    #[test]
    fn missing_facets_duplicates_and_nonmanifold_edges_fail_closed() {
        let (positions, mut triangles) = box_mesh(20.0, 20.0, 20.0);
        triangles.pop();
        triangles.pop();
        let hole = analyze(&positions, &triangles, 0.0);
        assert_eq!(hole["closed"], false);
        assert_eq!(hole["naked_edges"], 4);
        assert_eq!(hole["boundary_loops"], 1);
        let (_, mut triangles) = box_mesh(20.0, 20.0, 20.0);
        triangles.push(triangles[0]);
        let duplicate = analyze(&positions, &triangles, 0.0);
        assert_eq!(duplicate["duplicate_faces"], 1);
        assert!(duplicate["non_manifold_edges"].as_u64().unwrap_or(0) >= 3);
        assert_eq!(duplicate["closed"], false);
    }

    #[test]
    fn two_closed_tetrahedra_touching_at_one_vertex_are_nonmanifold() {
        // Both tetrahedra enclose positive volume and every edge has exactly
        // two incident triangles. The sole shared vertex has two disconnected
        // fans; edge-incidence-only closure would incorrectly accept it.
        let vertices = vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
        ];
        let faces = vec![
            [0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3],
            [0, 4, 5], [0, 6, 4], [0, 5, 6], [4, 6, 5],
        ];
        let report = analyze(&vertices, &faces, 0.0);
        assert_eq!(report["naked_edges"], 0, "{report}");
        assert_eq!(report["non_manifold_edges"], 0, "{report}");
        assert_eq!(report["vertex_manifold_complete"], true);
        assert_eq!(report["non_manifold_vertices"], 1, "{report}");
        assert_eq!(report["closed"], false);
        assert_eq!(report["signed_volume_mm3"], Value::Null);
    }

    #[test]
    fn out_of_range_nonfinite_zero_area_and_weld_collapses_are_invalid() {
        let (positions, mut triangles) = box_mesh(20.0, 20.0, 20.0);
        triangles.push([1, 2, 99]);
        let bad_index = analyze(&positions, &triangles, 0.0);
        assert_eq!(bad_index["invalid_index_triangles"], 1);
        assert_eq!(bad_index["closed"], false);
        triangles.pop();
        triangles.push([0, 0, 1]);
        let zero = analyze(&positions, &triangles, 0.0);
        assert_eq!(zero["degenerate_triangles"], 1);
        assert_eq!(zero["closed"], false);
        triangles.pop();
        let mut nonfinite = positions.clone();
        nonfinite.push(Vec3::new(f64::NAN, 0.0, 0.0));
        let invalid = analyze(&nonfinite, &triangles, 0.0);
        assert_eq!(invalid["nonfinite_vertices"], 1);
        assert_eq!(invalid["closed"], false);
        let (thin_positions, thin_triangles) = box_mesh(20.0, 20.0, 0.02);
        let collapsed = analyze(&thin_positions, &thin_triangles, 0.03);
        assert!(collapsed["collapsed_after_weld"].as_u64().unwrap_or(0) > 0, "{collapsed}");
        assert_eq!(collapsed["closed"], false);
    }

    #[test]
    fn duplicate_coordinate_vertices_are_counted_without_faking_a_hole() {
        let (mut positions, mut triangles) = box_mesh(20.0, 20.0, 20.0);
        positions.push(positions[0]);
        triangles[0][0] = 8;
        let report = analyze(&positions, &triangles, 0.0);
        assert_eq!(report["duplicate_vertices"], 1);
        assert_eq!(report["closed"], true, "{report}");
    }

    #[test]
    fn binary_stl_round_trip_preserves_dimensions_and_outward_volume() {
        let (positions, triangles) = box_mesh(20.0, 20.0, 20.0);
        let input = Mesh { positions, triangles, ..Mesh::default() };
        let bytes = solidcraft_io::stl_binary(&[input], "Cube20");
        assert_eq!(bytes.len(), 84 + 12 * 50);
        assert_eq!(u32::from_le_bytes(bytes[80..84].try_into().unwrap()), 12);
        let read = solidcraft_io::read_stl(&bytes).unwrap();
        assert_eq!(read.len(), 12);
        let pos: Vec<Vec3> = read.iter().flat_map(|t| t.iter().map(|p| Vec3::from(p.map(f64::from)))).collect();
        let idx: Vec<[u32; 3]> = (0..read.len() as u32).map(|n| [3*n, 3*n+1, 3*n+2]).collect();
        let bounds: Aabb3 = pos.iter().copied().collect();
        assert_eq!(bounds.size(), Vec3::new(20.0, 20.0, 20.0));
        let report = analyze(&pos, &idx, 0.0);
        assert_eq!(report["closed"], true, "{report}");
        mm(&report, "signed_volume_mm3", 8_000.0, 1e-8);
    }

    #[test]
    fn three_mf_round_trip_preserves_multiple_parts_colors_and_mm() {
        use solidcraft_io::{MeshObject, read_3mf, write_3mf};
        let (pos, tri) = box_mesh(20.0, 20.0, 20.0);
        let first = MeshObject {
            name: "Cube20".to_owned(), positions: pos.clone(), triangles: tri.clone(),
            color: Some([0.2, 0.4, 0.6]), ..Default::default()
        };
        let offset = Vec3::new(30.0, 0.0, 0.0);
        let second = MeshObject {
            name: "SecondPart".to_owned(), positions: pos.iter().map(|p| *p + offset).collect(),
            triangles: tri, ..Default::default()
        };
        let bytes = write_3mf(&[first, second]).unwrap();
        let (objects, warnings) = read_3mf(&bytes).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(objects.len(), 2);
        let cube = objects.iter().find(|o| o.name == "Cube20").unwrap();
        let other = objects.iter().find(|o| o.name == "SecondPart").unwrap();
        let bounds: Aabb3 = cube.positions.iter().copied().collect();
        let far: Aabb3 = other.positions.iter().copied().collect();
        assert!(bounds.size().dist(Vec3::new(20.0, 20.0, 20.0)) < 1e-5);
        assert!(far.min.dist(Vec3::new(30.0, 0.0, 0.0)) < 1e-5);
        let report = analyze(&cube.positions, &cube.triangles, 0.0);
        assert_eq!(report["closed"], true, "{report}");
        mm(&report, "signed_volume_mm3", 8_000.0, 1e-3);
        let color = cube.color.unwrap();
        for (got, want) in color.iter().zip([0.2_f32, 0.4, 0.6]) {
            assert!((*got - want).abs() <= 1.0 / 255.0 + 1e-6);
        }
    }
}
