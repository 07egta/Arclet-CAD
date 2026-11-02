//! Approximate inertia about the centre of mass, integrated over a closed oriented
//! triangle mesh using signed tetrahedra. B-rep curves are tessellated first: this is
//! NOT an exact NURBS mass-property integral and has no certified error bound.
//! Reject inconsistent meshes instead of reporting plausible-looking tensors.

use serde_json::{Value, json};
use solidcraft_geom::{Mesh, Vec3};
use solidcraft_kernel::Body;

/// A volume in mm^3, a centre in mm, and material density in g/cm^3.
pub(super) fn measure(body: &Body, volume_mm3: f64, centre_mm: Vec3, density_g_cm3: f64) -> Value {
    match calculate(body, volume_mm3, centre_mm, density_g_cm3) {
        Ok(v) => v,
        Err(error) => json!({"error": error}),
    }
}

fn calculate(body: &Body, expected_volume: f64, expected_centre: Vec3, density_g_cm3: f64) -> std::result::Result<Value, String> {
    if body.is_mesh() && !body.is_closed_mesh() || !body.is_mesh() && body.is_surface() {
        return Err("inertia requires a closed, oriented volume".into());
    }
    if !(expected_volume.is_finite() && expected_volume > 0.0 && expected_centre.is_finite()
        && density_g_cm3.is_finite() && density_g_cm3 >= 0.0)
    {
        return Err("invalid volume, centroid or density".into());
    }
    // The full validity diagnostic also checks for failed face tessellation. It does not
    // certify that the B-rep is free from all self-intersections.
    let issues = body.validity();
    if !issues.is_empty() {
        return Err(format!("geometry validity check failed: {}", issues.join("; ")));
    }
    let size = body.size();
    if !(size.is_finite() && size > 0.0) {
        return Err("invalid body size".into());
    }
    let chord_mm = (size * 5e-5).max(1e-4);
    let mesh = body.tessellate_with(chord_mm, true).map_err(|e| format!("inertia tessellation failed: {e}"))?;
    tensor_from_mesh(&mesh, expected_volume, expected_centre, density_g_cm3, chord_mm)
}

fn tensor_from_mesh(
    mesh: &Mesh,
    expected_volume: f64,
    expected_centre: Vec3,
    density_g_cm3: f64,
    chord_mm: f64,
) -> std::result::Result<Value, String> {
    if mesh.triangles.is_empty() {
        return Err("the inertia mesh has no triangles".into());
    }
    let bounds = mesh.bounds();
    if bounds.is_empty() || !bounds.center().is_finite() {
        return Err("the inertia mesh bounds are invalid".into());
    }
    let reference = bounds.center();
    let mut volume = 0.0;
    let mut first = [0.0; 3];
    let mut second = [[0.0; 3]; 3];

    // An oriented triangle (a,b,c) bounds an oriented tetrahedron (0,a,b,c).
    // Its signed volume is det(a,b,c)/6. Integrating barycentric coordinates:
    // first_i = dV * (a_i+b_i+c_i)/4,
    // second_ij = dV * ((a_i+b_i+c_i)(a_j+b_j+c_j)+sum p_i*p_j)/20.
    for triangle in &mesh.triangles {
        let [a, b, c] = mesh.tri(triangle).ok_or_else(|| "the inertia mesh has an invalid triangle index".to_string())?;
        if !(a.is_finite() && b.is_finite() && c.is_finite()) {
            return Err("the inertia mesh contains a non-finite triangle".into());
        }
        let (a, b, c) = (a - reference, b - reference, c - reference);
        let v = a.dot(b.cross(c)) / 6.0;
        if !v.is_finite() {
            return Err("the inertia volume overflowed".into());
        }
        volume += v;
        let a: [f64; 3] = a.into();
        let b: [f64; 3] = b.into();
        let c: [f64; 3] = c.into();
        let sum = [a[0] + b[0] + c[0], a[1] + b[1] + c[1], a[2] + b[2] + c[2]];
        for (i, moment) in first.iter_mut().enumerate() {
            *moment += v * sum[i] * 0.25;
        }
        for (i, row) in second.iter_mut().enumerate() {
            for (j, term) in row.iter_mut().enumerate() {
                *term += v * (sum[i] * sum[j] + a[i] * a[j] + b[i] * b[j] + c[i] * c[j]) / 20.0;
            }
        }
    }

    if !(volume.is_finite() && volume > 0.0 && first.iter().all(|x| x.is_finite())
        && second.iter().flatten().all(|x| x.is_finite()))
    {
        return Err("the inertia mesh has invalid signed mass moments".into());
    }
    if (volume - expected_volume).abs() > 0.005 * expected_volume {
        return Err(format!(
            "inertia mesh volume disagrees with measured volume ({volume:.6} versus {expected_volume:.6} mm^3)"
        ));
    }
    let centre = reference + Vec3::new(first[0] / volume, first[1] / volume, first[2] / volume);
    if !(centre.is_finite() && centre.dist(expected_centre) <= bounds.diagonal().max(1.0) * 0.002) {
        return Err("inertia mesh centroid disagrees with measured centroid".into());
    }

    // The central second-moment tensor subtracts the parallel-axis term; the
    // volume-inertia tensor is tr(S)I - S. Density is g/mm^3 = (g/cm^3)/1000.
    let mut central = [[0.0; 3]; 3];
    for (i, row) in central.iter_mut().enumerate() {
        for (j, term) in row.iter_mut().enumerate() {
            *term = second[i][j] - first[i] * first[j] / volume;
        }
    }
    let trace = central[0][0] + central[1][1] + central[2][2];
    let density = density_g_cm3 / 1000.0;
    let mut tensor = [[0.0; 3]; 3];
    for (i, row) in tensor.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = density * (if i == j { trace } else { 0.0 } - central[i][j]);
            if !value.is_finite() {
                return Err("the inertia tensor has a non-finite component".into());
            }
        }
    }
    let tolerance = trace.abs().max(1.0) * density * 1e-8;
    if tensor.iter().enumerate().any(|(i, row)| row[i] < -tolerance) {
        return Err("the inertia tensor has a significantly negative principal-axis moment".into());
    }
    Ok(json!({
        "method": "signed_tetrahedra_fine_tessellation",
        "about": "centre_of_mass",
        "origin_mm": centre,
        "mass_g": volume * density,
        "volume_mm3": volume,
        "tensor_g_mm2": tensor,
        "mesh_chord_target_mm": chord_mm,
        "error_bound_g_mm2": null,
        "certified": false,
        "not_checked": ["self_intersection", "exact_nurbs_moments"],
    }))
}
