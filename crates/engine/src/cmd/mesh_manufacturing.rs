//! Read-only, conservative print-preflight checks for a B-rep tessellation or an
//! imported mesh. This is a screening report, not a slicer/printer certification.
//! Never silently mark an unexamined self-intersection or wall as safe.

use serde_json::{Value, json};
use solidcraft_geom::mesh_distance::triangle_distance;
use solidcraft_geom::{Aabb3, Vec3};

use crate::params::bad;
use crate::{Result, Session};

const MAX_INTERSECTION_TRIANGLES: usize = 8_000;
const MAX_INTERSECTION_CANDIDATES: usize = 500_000;
const MAX_INTERSECTION_EXAMPLES: usize = 12;
const MAX_THICKNESS_TRIANGLES: usize = 20_000;
const MAX_THICKNESS_SAMPLES: usize = 96;

fn number(p: &Value, cmd: &str, name: &str, default: f64, lo: f64, hi: f64) -> Result<f64> {
    match p.get(name) {
        None => Ok(default),
        Some(Value::Number(n)) if n.as_f64().is_some_and(|v| v.is_finite() && (lo..=hi).contains(&v)) => {
            Ok(n.as_f64().unwrap_or(default))
        }
        _ => Err(bad(cmd, format!("`{name}` must be a finite number from {lo} to {hi}"))),
    }
}

fn bed_size(p: &Value, cmd: &str) -> Result<Option<[f64; 3]>> {
    match p.get("bed_size_mm") {
        None => Ok(None),
        Some(Value::Array(a)) if a.len() == 3 => {
            let dims = a.iter().map(Value::as_f64).collect::<Option<Vec<_>>>()
                .ok_or_else(|| bad(cmd, "`bed_size_mm` must be [width, depth, height] in mm"))?;
            if dims.iter().any(|x| !x.is_finite() || !(0.0..=1e6).contains(x) || *x == 0.0) {
                return Err(bad(cmd, "`bed_size_mm` must have three finite, positive dimensions at most 1000000 mm"));
            }
            Ok(Some([dims[0], dims[1], dims[2]]))
        }
        _ => Err(bad(cmd, "`bed_size_mm` must be [width, depth, height] in mm")),
    }
}

/// A conservative triangle-pair screen: excludes common-vertex neighbours, sorts
/// AABBs along X and uses exact triangle distances for candidates. The cap is
/// reported, not disguised as a successful global check. Contact/overlap of
/// coplanar non-neighbours is included as a potential intersection, not a proof
/// of physical solid overlap.
fn intersection_screen(positions: &[Vec3], triangles: &[[u32; 3]], tolerance_mm: f64) -> Value {
    if triangles.len() > MAX_INTERSECTION_TRIANGLES {
        return json!({"complete": false, "reason": "mesh exceeds self-intersection triangle budget",
            "triangle_limit": MAX_INTERSECTION_TRIANGLES, "candidate_limit": MAX_INTERSECTION_CANDIDATES,
            "potential_pairs": null});
    }
    let mut valid = Vec::with_capacity(triangles.len());
    for (i, ids) in triangles.iter().enumerate() {
        let (Some(&a), Some(&b), Some(&c)) = (positions.get(ids[0] as usize), positions.get(ids[1] as usize), positions.get(ids[2] as usize)) else {
            return json!({"complete": false, "reason": "invalid triangle index"});
        };
        if !(a.is_finite() && b.is_finite() && c.is_finite()) || (b - a).cross(c - a).len2() <= 0.0 {
            return json!({"complete": false, "reason": "invalid or degenerate triangle"});
        }
        valid.push((i, *ids, [a, b, c], Aabb3::from_iter([a, b, c])));
    }
    let mut order: Vec<usize> = (0..valid.len()).collect();
    order.sort_by(|&a, &b| valid[a].3.min.x.total_cmp(&valid[b].3.min.x));
    let (mut pairs, mut hits, mut examples) = (0usize, 0usize, Vec::new());
    for (pos, &i) in order.iter().enumerate() {
        let (ia, aa, a, ab) = valid[i];
        for &j in order.iter().skip(pos + 1) {
            let (ib, bb, b, xb) = valid[j];
            if xb.min.x > ab.max.x + tolerance_mm {
                break;
            }
            if ab.max.y + tolerance_mm < xb.min.y || xb.max.y + tolerance_mm < ab.min.y
                || ab.max.z + tolerance_mm < xb.min.z || xb.max.z + tolerance_mm < ab.min.z
            {
                continue;
            }
            // Neighbouring triangles naturally meet, and are not self-intersections.
            if aa.iter().any(|v| bb.contains(v)) {
                continue;
            }
            if pairs >= MAX_INTERSECTION_CANDIDATES {
                return json!({"complete": false, "reason": "self-intersection candidate budget exhausted",
                    "candidate_limit": MAX_INTERSECTION_CANDIDATES,
                    "pairs_tested": pairs, "potential_pairs": hits, "examples": examples});
            }
            pairs += 1;
            let (distance, ..) = triangle_distance(&a, &b);
            if distance.is_finite() && distance <= tolerance_mm {
                hits += 1;
                if examples.len() < MAX_INTERSECTION_EXAMPLES {
                    examples.push([ia, ib]);
                }
            }
        }
    }
    json!({"complete": true, "method": "AABB_sweep_triangle_distance_excluding_shared_vertex",
        "pairs_tested": pairs, "potential_pairs": hits, "examples": examples,
        "certified_global_nonintersection": false,
        "limitations": "Pairs sharing vertices are excluded; numerical and coplanar degeneracies are not certified."})
}

/// Distance along an inward-pointing ray to a triangle. The ray origin lies
/// on the source face, so exclude t near zero. No culling by winding: opposite
/// wall normals need not be perfectly antiparallel on curved surfaces.
fn ray_hit(origin: Vec3, direction: Vec3, tri: [Vec3; 3], epsilon: f64) -> Option<f64> {
    let [a, b, c] = tri;
    let e1 = b - a;
    let e2 = c - a;
    let h = direction.cross(e2);
    let determinant = e1.dot(h);
    if determinant.abs() <= 1e-14 * e1.len().max(1.0) * e2.len().max(1.0) {
        return None;
    }
    let inverse = 1.0 / determinant;
    let s = origin - a;
    let u = inverse * s.dot(h);
    if !(-1e-12..=1.0 + 1e-12).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = inverse * direction.dot(q);
    if v < -1e-12 || u + v > 1.0 + 1e-12 {
        return None;
    }
    let t = inverse * e2.dot(q);
    (t.is_finite() && t > epsilon).then_some(t)
}

/// A sampled inward-ray feature thickness. This is NOT a minimum wall
/// certificate: sparse samples, cavities, topology and orientation can
/// produce either missed thin regions or an unrelated opposite wall.
fn wall_screen(positions: &[Vec3], triangles: &[[u32; 3]], nozzle_mm: f64, size_mm: f64) -> Value {
    if triangles.len() > MAX_THICKNESS_TRIANGLES {
        return json!({"complete": false, "reason": "wall sampling triangle budget exceeded",
            "sample_limit": MAX_THICKNESS_SAMPLES, "minimum_sampled_mm": null});
    }
    let epsilon = (size_mm.max(1.0) * 1e-8).min(nozzle_mm * 0.001);
    let stride = triangles.len().div_ceil(MAX_THICKNESS_SAMPLES).max(1);
    let (mut samples, mut hits, mut shortest) = (0usize, 0usize, f64::INFINITY);
    for i in (0..triangles.len()).step_by(stride) {
        let Some(ids) = triangles.get(i) else { continue };
        let (Some(&a), Some(&b), Some(&c)) = (positions.get(ids[0] as usize), positions.get(ids[1] as usize), positions.get(ids[2] as usize)) else { continue };
        let Some(normal) = (b - a).cross(c - a).normalized() else { continue };
        samples += 1;
        let centre = (a + b + c) * (1.0 / 3.0);
        let inward = -normal;
        let mut local = f64::INFINITY;
        for (j, other) in triangles.iter().enumerate() {
            if j == i || ids.iter().any(|v| other.contains(v)) { continue; }
            let Some((&p, &q, &r)) = positions.get(other[0] as usize)
                .zip(positions.get(other[1] as usize))
                .zip(positions.get(other[2] as usize))
                .map(|((a, b), c)| (a, b, c)) else { continue };
            if let Some(distance) = ray_hit(centre, inward, [p, q, r], epsilon) {
                local = local.min(distance);
            }
        }
        if local.is_finite() {
            hits += 1;
            shortest = shortest.min(local);
        }
    }
    json!({
        "complete": false, "method": "sampled_inward_ray_to_next_face",
        "sampled_faces": samples, "rays_with_hits": hits,
        "minimum_sampled_mm": shortest.is_finite().then_some(shortest),
        "below_nozzle_diameter": shortest.is_finite() && shortest < nozzle_mm,
        "nozzle_diameter_mm": nozzle_mm,
        "certified_minimum_wall_thickness": false,
        "limitations": "Not all faces sampled; ray distances are candidate local features, not a guaranteed minimum thickness."
    })
}

fn overhang_screen(positions: &[Vec3], triangles: &[[u32; 3]], bounds: Aabb3, overhang_deg: f64) -> Value {
    let extent = bounds.size();
    let tolerance = bounds.diagonal().max(1.0) * 1e-7;
    let limit = -overhang_deg.to_radians().sin();
    let (mut overhang_area, mut bed_contact_area, mut triangles_checked) = (0.0, 0.0, 0usize);
    for ids in triangles {
        let (Some(&a), Some(&b), Some(&c)) = (positions.get(ids[0] as usize), positions.get(ids[1] as usize), positions.get(ids[2] as usize)) else { continue };
        if !(a.is_finite() && b.is_finite() && c.is_finite()) { continue; }
        let cross = (b - a).cross(c - a);
        let area = cross.len() * 0.5;
        if !(area > 0.0 && area.is_finite()) { continue; }
        let nz = cross.z / cross.len();
        triangles_checked += 1;
        let at_bottom = [a.z, b.z, c.z].iter().all(|z| (*z - bounds.min.z).abs() <= tolerance);
        if at_bottom && nz < -0.95 {
            bed_contact_area += area;
        } else if nz < limit {
            overhang_area += area;
        }
    }
    json!({
        "method": "downward_triangle_normal_screen",
        "overhang_angle_from_vertical_deg": overhang_deg,
        "possible_support_area_mm2": overhang_area,
        "horizontal_bed_contact_area_mm2": bed_contact_area,
        "has_flat_bed_contact": bed_contact_area > 0.0,
        "height_mm": extent.z,
        "z_translation_to_bed_mm": -bounds.min.z,
        "triangles_checked": triangles_checked,
        "support_guarantee": false,
        "limitations": "No bridging, cavities, toolpath or physical support analysis; faces already on the lowest plane are excluded."
    })
}

pub(super) fn check(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "mesh.manufacturing_check";
    let nozzle = number(p, cmd, "nozzle_diameter_mm", 0.4, 0.01, 10.0)?;
    let angle = number(p, cmd, "overhang_deg", 45.0, 0.0, 89.0)?;
    let bed = bed_size(p, cmd)?;
    let name = super::target(s, p, cmd)?;
    let state = s.world_state();
    let source = state.body(&name).ok_or_else(|| bad(cmd, format!("no body `{name}`")))?;
    let size = source.body.size();
    if !size.is_finite() || size <= 0.0 {
        return Err(bad(cmd, "body has invalid overall size"));
    }
    // The default matches the actual STL/3MF export weld tolerance. A
    // B-rep's adjacent faces often have separate, coincident vertex indices.
    let weld = number(p, cmd, "weld_tolerance_mm", (size * 1e-6).max(1e-7), 0.0, 1000.0)?;
    let chord = number(p, cmd, "chord_mm", (size * 5e-5).max(1e-4), 1e-6, 1000.0)?;
    let tess = if source.body.is_mesh() {
        None
    } else {
        Some(source.body.tessellate_with(chord, true).map_err(|e| bad(cmd, format!("mesh generation failed: {e}")))?)
    };
    let (positions, triangles) = match &tess {
        Some(m) => (m.positions.as_slice(), m.triangles.as_slice()),
        None => source.body.triangle_mesh().ok_or_else(|| bad(cmd, "mesh body has no triangles"))?,
    };
    if positions.len() > 6_000_000 || triangles.len() > 2_000_000 {
        return Err(bad(cmd, "mesh exceeds preflight resource limit (6M vertices or 2M triangles)"));
    }
    if triangles.is_empty() {
        return Err(bad(cmd, "body contains no mesh triangles"));
    }
    // Diagnose the original before welding. Otherwise a malformed facet that
    // the exporter silently drops could appear as a successfully repaired mesh.
    let source_topology = super::analyze(positions, triangles, 0.0);
    let malformed_input = ["invalid_index_triangles", "nonfinite_vertices",
        "degenerate_triangles", "duplicate_faces"]
        .iter().any(|key| source_topology[*key].as_u64().unwrap_or(0) > 0);
    let (print_positions, print_triangles) = if malformed_input {
        (Vec::new(), Vec::new())
    } else {
        solidcraft_io::weld(positions, triangles, weld.max(1e-12))
    };
    let topology = if malformed_input {
        json!({"closed": false, "error": "source has invalid or duplicate facets; refusing to discard them"})
    } else {
        super::analyze(&print_positions, &print_triangles, 0.0)
    };
    let bounds: Aabb3 = positions.iter().copied().collect();
    if bounds.is_empty() || !bounds.min.is_finite() || !bounds.max.is_finite() {
        return Err(bad(cmd, "mesh vertices have invalid bounds"));
    }
    let dimensions = bounds.size();
    let topology_ok = !malformed_input && topology["closed"] == true;
    let intersections = if topology_ok {
        intersection_screen(&print_positions, &print_triangles, (bounds.diagonal() * 1e-10).max(1e-10))
    } else {
        json!({"complete": false, "reason": "mesh topology is invalid or inward; repair before intersection screening"})
    };
    let wall = if topology_ok && intersections["potential_pairs"].as_u64() == Some(0) && intersections["complete"] == true {
        wall_screen(&print_positions, &print_triangles, nozzle, size)
    } else {
        json!({"complete": false, "reason": "valid nonintersecting closed mesh required for wall sampling",
            "minimum_sampled_mm": null})
    };
    let orientation = overhang_screen(&print_positions, &print_triangles, bounds, angle);
    let fit = bed.map(|dims| {
        json!({
            "bed_mm": dims, "fits_current_xy_orientation":
                dimensions.x <= dims[0] && dimensions.y <= dims[1] && dimensions.z <= dims[2],
            "placement_method": "XY translation allowed; rotation not considered",
        })
    });
    let self_intersections = intersections["potential_pairs"].as_u64().unwrap_or(0);
    let checked_intersections = intersections["complete"] == true;
    let thin_warning = wall["below_nozzle_diameter"] == true;
    let possible_support = orientation["possible_support_area_mm2"].as_f64().unwrap_or(0.0) > 0.0;
    let fails_bed = fit.as_ref().is_some_and(|v| v["fits_current_xy_orientation"] == false);
    let verdict = if !topology_ok || self_intersections > 0 { "invalid_mesh" }
        else if !checked_intersections { "inspection_incomplete" }
        else if thin_warning || possible_support || fails_bed || orientation["has_flat_bed_contact"] == false { "requires_review" }
        else { "passed_limited_screening" };
    Ok(json!({
        "body": name,
        "source": if source.body.is_mesh() { "imported_mesh" } else { "brep_tessellation" },
        "units": "millimeter",
        "chord_target_mm": if tess.is_some() { Some(chord) } else { None },
        "weld_tolerance_mm": weld,
        "bounds": {"min": bounds.min, "max": bounds.max},
        "size_mm": [dimensions.x, dimensions.y, dimensions.z],
        "source_topology": source_topology,
        "topology": topology,
        "candidate_mesh_method": "same_tolerance_weld_as_STL_3MF_export",
        "candidate_mesh_vertices": print_positions.len(),
        "candidate_mesh_triangles": print_triangles.len(),
        "potential_self_intersections": intersections,
        "sampled_wall_thickness": wall,
        "orientation_and_support": orientation,
        "bed_fit": fit,
        "verdict": verdict,
        "manufacturing_certified": false,
        "unchecked": [
            "minimum_wall_thickness_certification",
            "complete_self_intersection_certificate",
            "printer_firmware_and_gcode_compatibility",
            "material_specific_slicer_settings",
            "inter_part_clearance",
            "slicer_path_and_support_validation",
        ],
    }))
}
