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
        .params("body?: named solid or mesh; weld_tolerance_mm?: number (default 0); chord_mm?: B-rep tessellation tolerance (default size*0.00005, min 0.0001); nozzle_diameter_mm?: number (default 0.4); overhang_deg?: number (default 45); bed_size_mm?: [width,depth,height] (optional). Reports manifold/orientation/duplicate/invalid checks, bounded potential self-intersections, bounds, volume, bed fit, overhang screening, build-plate contact and thin-feature warning. No wall-thickness certification, no printer compatibility guarantee."),
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
    let topology_closed =
        !triangles.is_empty() && degenerate_triangles == 0 && nonfinite_vertices == 0
        && duplicate_faces == 0 && naked_edges == 0 && non_manifold_edges == 0;
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
