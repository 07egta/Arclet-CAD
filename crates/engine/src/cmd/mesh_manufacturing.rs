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
    // The default matches MeshQuality::Standard in the STL/3MF exporter.
    // An explicit chord_mm is an analysis override, not a saved export option.
    let chord = number(p, cmd, "chord_mm", (size * 2e-4).max(1e-3), 1e-6, 1000.0)?;
    let tess = if source.body.is_mesh() {
        None
    } else {
        Some(source.body.tessellate(chord).map_err(|e| bad(cmd, format!("mesh generation failed: {e}")))?)
    };
    let (positions, triangles) = match &tess {
        Some(m) => (m.positions.as_slice(), m.triangles.as_slice()),
        None => source.body.triangle_mesh().ok_or_else(|| bad(cmd, "mesh body has no triangles"))?,
    };
    // The existing 3MF weld also checks T-junctions between open edges; on an
    // adversarial mesh that step can be superlinear. Bound the read-only GUI
    // command independently of the exporter's much larger format limits.
    if positions.len() > 180_000 || triangles.len() > 60_000 {
        return Err(bad(cmd, "mesh exceeds print-preflight resource limit (180000 vertices or 60000 triangles); use external mesh validation"));
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
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    if malformed_input {
        blockers.push("source mesh has invalid/duplicate triangles");
    }
    if !topology_ok {
        blockers.push("export candidate is not a closed, outward, two-manifold solid");
    }
    if self_intersections > 0 {
        blockers.push("potential self-intersecting triangle pairs were found");
    }
    if !checked_intersections {
        warnings.push("self-intersection screening was incomplete");
    }
    if thin_warning {
        warnings.push("a sampled inward wall distance is below the nozzle diameter");
    }
    if possible_support {
        warnings.push("downward-facing surfaces may require supports");
    }
    if orientation["has_flat_bed_contact"] == false {
        warnings.push("no horizontal underside was found on the lowest mesh plane");
    }
    if fails_bed {
        warnings.push("bounding box exceeds specified build volume in the current orientation");
    }
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
        "candidate_mesh_method": "standard_STL_3MF_tessellation_and_weld_when_defaults_used",
        "candidate_mesh_vertices": print_positions.len(),
        "candidate_mesh_triangles": print_triangles.len(),
        "potential_self_intersections": intersections,
        "sampled_wall_thickness": wall,
        "orientation_and_support": orientation,
        "bed_fit": fit,
        "verdict": verdict,
        "blockers": blockers,
        "warnings": warnings,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn box_mesh(x: f64, y: f64, z: f64) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0), Vec3::new(x, 0.0, 0.0),
            Vec3::new(x, y, 0.0), Vec3::new(0.0, y, 0.0),
            Vec3::new(0.0, 0.0, z), Vec3::new(x, 0.0, z),
            Vec3::new(x, y, z), Vec3::new(0.0, y, z),
        ];
        let triangles = vec![
            [0, 2, 1], [0, 3, 2], [4, 5, 6], [4, 6, 7],
            [0, 1, 5], [0, 5, 4], [3, 6, 2], [3, 7, 6],
            [0, 4, 7], [0, 7, 3], [1, 2, 6], [1, 6, 5],
        ];
        (positions, triangles)
    }

    /// An open-top, closed-volume thin-walled vase: both wall surfaces, the
    /// rim, a solid bottom and an inner floor, with outward winding.
    fn thin_vase(radius_mm: f64, wall_mm: f64, bottom_mm: f64, height_mm: f64)
        -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let n = 48u32;
        let mut positions = Vec::new();
        for (radius, z) in [
            (radius_mm, 0.0), (radius_mm, height_mm),
            (radius_mm - wall_mm, bottom_mm), (radius_mm - wall_mm, height_mm),
        ] {
            for k in 0..n {
                let angle = std::f64::consts::TAU * f64::from(k) / f64::from(n);
                positions.push(Vec3::new(radius * angle.cos(), radius * angle.sin(), z));
            }
        }
        let bottom_centre = positions.len() as u32;
        positions.push(Vec3::new(0.0, 0.0, 0.0));
        let inner_centre = positions.len() as u32;
        positions.push(Vec3::new(0.0, 0.0, bottom_mm));
        let mut triangles = Vec::new();
        for i in 0..n {
            let j = (i + 1) % n;
            let (ob, on) = (i, j);
            let (ot, tn) = (i + n, j + n);
            let (ib, bn) = (i + 2 * n, j + 2 * n);
            let (it, itn) = (i + 3 * n, j + 3 * n);
            // exterior cylinder, interior cylinder, upper lip, underside,
            // and interior floor of the vase.
            triangles.extend([
                [ob, on, tn], [ob, tn, ot],
                [ib, itn, bn], [ib, it, itn],
                [ot, tn, itn], [ot, itn, it],
                [bottom_centre, on, ob],
                [inner_centre, ib, bn],
            ]);
        }
        (positions, triangles)
    }

    /// A cylindrical through-hole plate, built directly as correctly
    /// oriented surface triangles rather than trusting a boolean result.
    /// This also exercises a multiply connected manifold cross-section.
    fn annular_plate(outer: f64, inner: f64, height: f64, segments: u32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let mut positions = Vec::new();
        for (radius, z) in [(outer, 0.0), (outer, height), (inner, 0.0), (inner, height)] {
            for i in 0..segments {
                let angle = std::f64::consts::TAU * f64::from(i) / f64::from(segments);
                positions.push(Vec3::new(radius * angle.cos(), radius * angle.sin(), z));
            }
        }
        let mut triangles = Vec::new();
        for i in 0..segments {
            let j = (i + 1) % segments;
            let (ob, on) = (i, j);
            let (ot, tn) = (i + segments, j + segments);
            let (ib, bn) = (i + 2 * segments, j + 2 * segments);
            let (it, itn) = (i + 3 * segments, j + 3 * segments);
            triangles.extend([
                [ob, on, tn], [ob, tn, ot], // outside +radial
                [ib, itn, bn], [ib, it, itn], // bore -radial
                [ot, tn, itn], [ot, itn, it], // annular top +Z
                [ob, ib, bn], [ob, bn, on], // annular bottom -Z
            ]);
        }
        (positions, triangles)
    }

    /// Cap a simple cylinder at both ends with outward winding.
    fn cylinder(radius: f64, height: f64, segments: u32) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let mut positions = Vec::new();
        for z in [0.0, height] {
            for i in 0..segments {
                let angle = std::f64::consts::TAU * f64::from(i) / f64::from(segments);
                positions.push(Vec3::new(radius * angle.cos(), radius * angle.sin(), z));
            }
        }
        let bottom = positions.len() as u32;
        positions.push(Vec3::new(0.0, 0.0, 0.0));
        let top = positions.len() as u32;
        positions.push(Vec3::new(0.0, 0.0, height));
        let mut triangles = Vec::new();
        for i in 0..segments {
            let j = (i + 1) % segments;
            triangles.extend([
                [i, j, j + segments], [i, j + segments, i + segments],
                [bottom, j, i], [top, i + segments, j + segments],
            ]);
        }
        (positions, triangles)
    }

    #[test]
    fn circular_through_hole_plate_keeps_closed_bore_and_expected_volume() {
        let (positions, triangles) = annular_plate(10.0, 3.0, 4.0, 96);
        let topology = super::super::analyze(&positions, &triangles, 0.0);
        assert_eq!(topology["closed"], true, "{topology}");
        assert_eq!(topology["naked_edges"], 0);
        assert_eq!(topology["non_manifold_edges"], 0);
        let expected = std::f64::consts::PI * (10.0f64.powi(2) - 3.0f64.powi(2)) * 4.0;
        let measured = topology["signed_volume_mm3"].as_f64().unwrap();
        assert!((measured - expected).abs() / expected < 0.001, "{topology}");
        let intersections = intersection_screen(&positions, &triangles, 1e-9);
        assert_eq!(intersections["potential_pairs"], 0, "{intersections}");
        assert_eq!(intersections["complete"], true, "{intersections}");
        let beds: Aabb3 = positions.iter().copied().collect();
        let layout = overhang_screen(&positions, &triangles, beds, 45.0);
        let contact = layout["horizontal_bed_contact_area_mm2"].as_f64().unwrap_or(0.0);
        assert!((contact - std::f64::consts::PI * 91.0).abs() < 0.3, "{layout}");
    }

    #[test]
    fn closed_cylinder_volume_and_normal_diagnostics_have_analytic_reference() {
        let (positions, triangles) = cylinder(5.0, 20.0, 96);
        let topology = super::super::analyze(&positions, &triangles, 0.0);
        assert_eq!(topology["closed"], true, "{topology}");
        let expected = std::f64::consts::PI * 25.0 * 20.0;
        let measured = topology["signed_volume_mm3"].as_f64().unwrap();
        assert!((measured - expected).abs() / expected < 0.001, "{topology}");
        let intersections = intersection_screen(&positions, &triangles, 1e-9);
        assert_eq!(intersections["potential_pairs"], 0, "{intersections}");
    }

    #[test]
    fn cube_preflight_has_analytic_20mm_bounds_and_bed_contact() {
        let (positions, triangles) = box_mesh(20.0, 20.0, 20.0);
        let topo = super::super::analyze(&positions, &triangles, 0.0);
        assert_eq!(topo["closed"], true, "{topo}");
        let self_check = intersection_screen(&positions, &triangles, 1e-9);
        assert_eq!(self_check["complete"], true, "{self_check}");
        assert_eq!(self_check["potential_pairs"], 0, "{self_check}");
        let bounds: Aabb3 = positions.iter().copied().collect();
        let support = overhang_screen(&positions, &triangles, bounds, 45.0);
        assert!((support["horizontal_bed_contact_area_mm2"].as_f64().unwrap() - 400.0).abs() < 1e-8);
        assert_eq!(support["has_flat_bed_contact"], true);
        assert_eq!(support["possible_support_area_mm2"], 0.0);
        let thickness = wall_screen(&positions, &triangles, 0.4, 20.0);
        assert_eq!(thickness["below_nozzle_diameter"], false, "{thickness}");
        let nearest = thickness["minimum_sampled_mm"].as_f64().unwrap_or(f64::NAN);
        assert!((nearest - 20.0).abs() < 1e-8, "{thickness}");
    }

    #[test]
    fn crossed_nonadjacent_triangles_are_detected_without_shared_vertices() {
        let points = vec![
            Vec3::new(-3.0, -3.0, 0.0), Vec3::new(3.0, -3.0, 0.0), Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(-3.0, 0.0, -1.0), Vec3::new(3.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 3.0),
        ];
        let report = intersection_screen(&points, &[[0, 1, 2], [3, 4, 5]], 1e-9);
        assert_eq!(report["complete"], true);
        assert_eq!(report["potential_pairs"], 1, "{report}");
        assert_eq!(report["examples"][0], json!([0, 1]));
    }

    #[test]
    fn extremely_large_self_intersection_workload_is_not_marked_clear() {
        let positions = [
            Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0),
        ];
        let triangles = vec![[0, 1, 2]; MAX_INTERSECTION_TRIANGLES + 1];
        let report = intersection_screen(&positions, &triangles, 1e-9);
        assert_eq!(report["complete"], false);
        assert_eq!(report["potential_pairs"], Value::Null);
    }

    #[test]
    fn sealed_hollow_case_has_outer_and_inner_shells_and_correct_net_volume() {
        let (mut vertices, mut faces) = box_mesh(20.0, 20.0, 20.0);
        let (inner, mut inner_faces) = box_mesh(16.0, 16.0, 16.0);
        let base = vertices.len() as u32;
        vertices.extend(inner.into_iter().map(|p| p + Vec3::new(2.0, 2.0, 2.0)));
        for face in &mut inner_faces {
            for index in face.iter_mut() {
                *index += base;
            }
            // The void's faces must point out of the material and into
            // the trapped cavity; their signed volume is negative.
            face.swap(1, 2);
        }
        faces.extend(inner_faces);
        let topology = super::super::analyze(&vertices, &faces, 0.0);
        assert_eq!(topology["closed"], true, "{topology}");
        assert_eq!(topology["naked_edges"], 0);
        assert_eq!(topology["non_manifold_vertices"], 0);
        let volume_mm3 = topology["signed_volume_mm3"].as_f64().unwrap();
        assert!((volume_mm3 - (20.0_f64.powi(3) - 16.0_f64.powi(3))).abs() < 1e-8);
        let intersections = intersection_screen(&vertices, &faces, 1e-9);
        assert_eq!(intersections["complete"], true, "{intersections}");
        assert_eq!(intersections["potential_pairs"], 0, "{intersections}");
        let sampled = wall_screen(&vertices, &faces, 0.4, 20.0);
        assert_eq!(sampled["certified_minimum_wall_thickness"], false);
        assert_eq!(sampled["below_nozzle_diameter"], false, "{sampled}");
    }

    #[test]
    fn a_0_2mm_wall_is_only_a_warning_not_a_printability_certificate() {
        let (vertices, faces) = thin_vase(10.0, 0.2, 0.2, 20.0);
        let topology = super::super::analyze(&vertices, &faces, 0.0);
        assert_eq!(topology["closed"], true, "{topology}");
        let bounds: Aabb3 = vertices.iter().copied().collect();
        assert!((bounds.size().z - 20.0).abs() < 1e-12);
        let wall = wall_screen(&vertices, &faces, 0.4, 20.0);
        assert_eq!(wall["certified_minimum_wall_thickness"], false);
        assert_eq!(wall["below_nozzle_diameter"], true, "{wall}");
        let distance = wall["minimum_sampled_mm"].as_f64().unwrap_or(f64::NAN);
        assert!((distance - 0.2).abs() < 0.03, "{wall}");
        let support = overhang_screen(&vertices, &faces, bounds, 45.0);
        assert!(support["horizontal_bed_contact_area_mm2"].as_f64().unwrap_or(0.0) > 300.0);
    }

    #[test]
    fn independent_session_command_reports_invalid_threshold_and_bed_size() {
        let mut s = Session::default();
        s.execute("solid.box", &json!({
            "length": 20, "width": 20, "height": 20, "corner": [0, 0, 0],
            "body_name": "Cube20",
        })).unwrap();
        let result = s.execute("mesh.manufacturing_check", &json!({
            "body": "Cube20", "bed_size_mm": [180, 180, 180],
            "nozzle_diameter_mm": 0.4,
        })).unwrap();
        assert_eq!(result["units"], "millimeter");
        assert_eq!(result["source"], "brep_tessellation");
        assert_eq!(result["topology"]["closed"], true, "{result}");
        // Independently construct the actual default STL/3MF export mesh:
        // candidate vertex and triangle counts must match, not just the box.
        let state = s.world_state();
        let solid = &state.body("Cube20").unwrap().body;
        let export_mesh = solidcraft_io::export_mesh(solid).unwrap();
        let eps = (solid.size() * 1e-6).max(1e-7);
        let (vertices, triangles) = solidcraft_io::weld(&export_mesh.positions, &export_mesh.triangles, eps);
        assert_eq!(result["candidate_mesh_vertices"], vertices.len(), "{result}");
        assert_eq!(result["candidate_mesh_triangles"], triangles.len(), "{result}");
        assert_eq!(result["bed_fit"]["fits_current_xy_orientation"], true);
        assert_eq!(result["manufacturing_certified"], false);
        for bad in [json!(-0.2), json!(0.0), json!("0.4"), Value::Null, json!(100.0)] {
            assert!(s.execute("mesh.manufacturing_check", &json!({
                "body": "Cube20", "nozzle_diameter_mm": bad,
            })).is_err());
        }
        for bad in [json!([0, 180, 180]), json!([180, 180]), json!("180x180x180")] {
            assert!(s.execute("mesh.manufacturing_check", &json!({
                "body": "Cube20", "bed_size_mm": bad,
            })).is_err());
        }
    }
}
