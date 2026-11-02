//! Read-only, sample-based quality diagnostics of rational tensor-product NURBS surfaces.
//!
//! Positions are in mm; principal and mean curvature in mm^-1, Gaussian in mm^-2.
//! Derivatives are analytic on a knot span (tensor-product homogeneous basis derivatives,
//! followed by the quotient rule). At a knot lacking C2 smoothness the second derivative
//! is ambiguous: fail instead of guessing an average. A sampled G2 result is NOT a
//! global continuity guarantee or a Class A certification.
//!
//! The surface constructor lives in bspline; this module does not alter NURBS geometry,
//! kernels or document formats. Normal orientation follows r_u x r_v.

use serde::Serialize;

use crate::bspline::{HPoint, NurbsSurface, Side, SplineError, SplineResult, find_span, local_basis};
use crate::Vec3;

pub const MAX_QUALITY_GRID: usize = 129;
pub const MAX_JOIN_STATIONS: usize = 257;
pub const METHOD: &str = "analytic homogeneous NURBS basis derivatives (orders 0,1,2); sampled, not a global bound";

fn invalid<T>(message: impl Into<String>) -> SplineResult<T> {
    Err(SplineError::Invalid(message.into()))
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct SurfaceDerivatives {
    pub position: Vec3,
    pub du: Vec3,
    pub dv: Vec3,
    pub duu: Vec3,
    pub duv: Vec3,
    pub dvv: Vec3,
}

/// Evaluate only the active p+1 basis functions, so a large, sparse control net
/// does not cost O(number_of_controls) for every displayed sample.
fn basis(surface: &NurbsSurface, axis: usize, t: f64) -> SplineResult<Vec<(usize, [f64; 3])>> {
    let domain = surface.domain()[axis];
    if !t.is_finite() || !(domain[0]..=domain[1]).contains(&t) {
        return invalid("quality sample lies outside the finite NURBS domain");
    }
    let p = surface.degree()[axis];
    let knots = &surface.knots()[axis];
    let n = surface.count()[axis];
    // At an interior knot, C^(p-m) is the guaranteed order. It must be >= C2;
    // choosing the right derivative at a C0/C1 seam would invent a G2 result.
    if t > domain[0] && t < domain[1] {
        let multiplicity = knots.iter().filter(|&&x| x == t).count();
        if multiplicity > 0 && p.saturating_sub(multiplicity) < 2 {
            return invalid("second derivative is not guaranteed at an interior knot of multiplicity greater than degree - 2");
        }
    }
    let span = find_span(knots, p, n, t).ok_or_else(|| SplineError::Invalid("cannot locate NURBS knot span".into()))?;
    let side = if t == domain[1] { Side::Left } else { Side::Right };
    let mut out = Vec::with_capacity(p + 1);
    for i in span - p..=span {
        let local = knots.get(i..i + p + 2).ok_or_else(|| SplineError::Invalid("invalid local knot support".into()))?;
        let d = local_basis(local, t, side, 2);
        out.push((i, [d[0], d[1], d[2]]));
    }
    Ok(out)
}

pub fn derivatives(surface: &NurbsSurface, u: f64, v: f64) -> SplineResult<SurfaceDerivatives> {
    let bu = basis(surface, 0, u)?;
    let bv = basis(surface, 1, v)?;
    // A_ij = sum N_u^(i) N_v^(j) (w * P, w).
    let mut a = [HPoint::ZERO; 6]; // 00,10,01,20,11,02
    for (j, dv) in &bv {
        for (i, du) in &bu {
            let control = surface.point(*i, *j);
            for (slot, coeff) in [
                du[0] * dv[0],
                du[1] * dv[0],
                du[0] * dv[1],
                du[2] * dv[0],
                du[1] * dv[1],
                du[0] * dv[2],
            ]
            .into_iter()
            .enumerate()
            {
                a[slot] += control * coeff;
            }
        }
    }
    if a.iter().any(|h| !h.is_finite()) || !(a[0].w > f64::MIN_POSITIVE) {
        return invalid("unstable homogeneous derivatives or vanishing rational denominator");
    }
    let w = a[0].w;
    let position = a[0].xyz() / w;
    let du = (a[1].xyz() - position * a[1].w) / w;
    let dv = (a[2].xyz() - position * a[2].w) / w;
    let duu = (a[3].xyz() - du * (2.0 * a[1].w) - position * a[3].w) / w;
    let duv = (a[4].xyz() - du * a[2].w - dv * a[1].w - position * a[4].w) / w;
    let dvv = (a[5].xyz() - dv * (2.0 * a[2].w) - position * a[5].w) / w;
    let result = SurfaceDerivatives { position, du, dv, duu, duv, dvv };
    if [position, du, dv, duu, duv, dvv].iter().all(|p| p.is_finite()) {
        Ok(result)
    } else {
        invalid("nonfinite rational derivatives")
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct SurfaceCurvature {
    pub position: Vec3,
    pub normal: Vec3,
    pub du: Vec3,
    pub dv: Vec3,
    pub duu: Vec3,
    pub duv: Vec3,
    pub dvv: Vec3,
    pub k_min_per_mm: f64,
    pub k_max_per_mm: f64,
    pub mean_per_mm: f64,
    pub gaussian_per_mm2: f64,
    pub max_abs_principal_per_mm: f64,
}

impl SurfaceCurvature {
    /// Second fundamental form II(a,b), evaluated in physical unit tangent
    /// directions, with the sample's own oriented normal. A vector slightly off
    /// the tangent plane is projected into the plane by the metric solve.
    fn second_form(&self, a: Vec3, b: Vec3) -> Option<f64> {
        let (e, f, g) = (self.du.dot(self.du), self.du.dot(self.dv), self.dv.dot(self.dv));
        let det = e * g - f * f;
        if !(det.is_finite() && det > 0.0) {
            return None;
        }
        let coefficients = |d: Vec3| {
            let u = (g * d.dot(self.du) - f * d.dot(self.dv)) / det;
            let v = (e * d.dot(self.dv) - f * d.dot(self.du)) / det;
            (u, v)
        };
        let (au, av) = coefficients(a);
        let (bu, bv) = coefficients(b);
        let result = self.normal.dot(self.duu) * au * bu
            + self.normal.dot(self.duv) * (au * bv + av * bu)
            + self.normal.dot(self.dvv) * av * bv;
        result.is_finite().then_some(result)
    }
}

/// Local signed principal curvatures: eigenvalues of I^-1 II.
/// G0/G1/G2 across patches is a separate calculation; a smooth colour map
/// cannot establish any of those conditions.
pub fn curvature(surface: &NurbsSurface, u: f64, v: f64) -> SplineResult<SurfaceCurvature> {
    let d = derivatives(surface, u, v)?;
    let e1 = d.du.dot(d.du);
    let f1 = d.du.dot(d.dv);
    let g1 = d.dv.dot(d.dv);
    let det = e1 * g1 - f1 * f1;
    if !(e1 > 0.0 && g1 > 0.0 && det.is_finite() && det > 1e-12 * e1 * g1) {
        return invalid("degenerate or ill-conditioned NURBS surface metric");
    }
    let normal = d.du.cross(d.dv).normalized().ok_or_else(|| SplineError::Invalid("surface normal is singular".into()))?;
    let e2 = normal.dot(d.duu);
    let f2 = normal.dot(d.duv);
    let g2 = normal.dot(d.dvv);
    let mean = (e2 * g1 - 2.0 * f2 * f1 + g2 * e1) / (2.0 * det);
    let gaussian = (e2 * g2 - f2 * f2) / det;
    let discriminant = mean * mean - gaussian;
    let roundoff = 1e-12 * (1.0 + mean.abs() * mean.abs() + gaussian.abs());
    if !(mean.is_finite() && gaussian.is_finite() && discriminant >= -roundoff) {
        return invalid("nonfinite or inconsistent shape-operator eigenvalues");
    }
    let delta = discriminant.max(0.0).sqrt();
    let k_min = mean - delta;
    let k_max = mean + delta;
    if !k_min.is_finite() || !k_max.is_finite() {
        return invalid("principal curvature overflow");
    }
    Ok(SurfaceCurvature {
        position: d.position,
        normal,
        du: d.du,
        dv: d.dv,
        duu: d.duu,
        duv: d.duv,
        dvv: d.dvv,
        k_min_per_mm: k_min,
        k_max_per_mm: k_max,
        mean_per_mm: mean,
        gaussian_per_mm2: gaussian,
        max_abs_principal_per_mm: k_min.abs().max(k_max.abs()),
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct QualityGrid {
    pub u_samples: usize,
    pub v_samples: usize,
    pub samples: Vec<SurfaceCurvature>,
    pub max_abs_mean_gradient_per_mm2: f64,
    pub max_neighbor_normal_angle_deg: f64,
    pub min_neighbor_spacing_mm: f64,
    pub max_neighbor_spacing_mm: f64,
    pub method: &'static str,
    pub limitation: &'static str,
}

/// Sample a rectangular *parameter* grid, not an adaptive surface-area grid.
/// Adjacent mean-curvature differences per chord length are a waviness *indicator*,
/// NOT a bound on small-scale ripples between samples. Every sample is checked;
/// a singular or non-C2 point gives an error rather than a false smooth patch.
pub fn sample_grid(surface: &NurbsSurface, u_count: usize, v_count: usize) -> SplineResult<QualityGrid> {
    if !(3..=MAX_QUALITY_GRID).contains(&u_count) || !(3..=MAX_QUALITY_GRID).contains(&v_count) {
        return invalid(format!("quality grid axes must contain 3..={MAX_QUALITY_GRID} stations"));
    }
    let [u_domain, v_domain] = surface.domain();
    let mut samples = Vec::with_capacity(u_count * v_count);
    for j in 0..v_count {
        let v = v_domain[0] + (v_domain[1] - v_domain[0]) * j as f64 / (v_count - 1) as f64;
        for i in 0..u_count {
            let u = u_domain[0] + (u_domain[1] - u_domain[0]) * i as f64 / (u_count - 1) as f64;
            samples.push(curvature(surface, u, v)?);
        }
    }
    let mut gradient = 0.0_f64;
    let mut angle = 0.0_f64;
    let mut min_spacing = f64::INFINITY;
    let mut max_spacing = 0.0_f64;
    for j in 0..v_count {
        for i in 0..u_count {
            let index = j * u_count + i;
            for neighbor in [if i + 1 < u_count { Some(index + 1) } else { None }, if j + 1 < v_count { Some(index + u_count) } else { None }]
                .into_iter()
                .flatten()
            {
                let (a, b) = (&samples[index], &samples[neighbor]);
                let dist = a.position.dist(b.position);
                if !(dist.is_finite() && dist > 0.0) {
                    return invalid("sample grid has collapsed neighboring points");
                }
                gradient = gradient.max((a.mean_per_mm - b.mean_per_mm).abs() / dist);
                angle = angle.max(a.normal.dot(b.normal).clamp(-1.0, 1.0).acos().to_degrees());
                min_spacing = min_spacing.min(dist);
                max_spacing = max_spacing.max(dist);
            }
        }
    }
    Ok(QualityGrid {
        u_samples: u_count,
        v_samples: v_count,
        samples,
        max_abs_mean_gradient_per_mm2: gradient,
        max_neighbor_normal_angle_deg: angle,
        min_neighbor_spacing_mm: min_spacing,
        max_neighbor_spacing_mm: max_spacing,
        method: METHOD,
        limitation: "parameter grid and neighbor chords; curvature change and micro-waviness between stations can be missed",
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchSide {
    UMin,
    UMax,
    VMin,
    VMax,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct JoinTolerances {
    pub gap_mm: f64,
    pub angle_deg: f64,
    pub curvature_per_mm: f64,
}

impl JoinTolerances {
    fn validate(self) -> SplineResult<()> {
        if [self.gap_mm, self.angle_deg, self.curvature_per_mm].iter().any(|x| !x.is_finite() || *x < 0.0 || *x >= 1e9) {
            return invalid("join tolerances must be finite and nonnegative");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SampledJoinOrder {
    Discontinuous,
    G0,
    G1,
    G2,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct JoinPeak {
    pub value: f64,
    pub edge_fraction: f64,
    pub at: Vec3,
}

impl JoinPeak {
    fn zero() -> Self {
        Self { value: 0.0, edge_fraction: 0.0, at: Vec3::ZERO }
    }
    fn keep(&mut self, value: f64, fraction: f64, at: Vec3) {
        if value > self.value {
            *self = Self { value, edge_fraction: fraction, at };
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PatchJoinReport {
    pub samples: usize,
    pub gap_mm: JoinPeak,
    pub angle_deg: JoinPeak,
    /// Frobenius mismatch of the two second fundamental forms in a shared
    /// orthonormal (edge tangent, across-edge) frame, mm^-1.
    pub shape_operator_jump_per_mm: JoinPeak,
    pub tolerances: JoinTolerances,
    pub sampled_order: SampledJoinOrder,
    pub method: &'static str,
    pub higher_order: &'static str,
    pub limitation: &'static str,
}

fn on_boundary(surface: &NurbsSurface, side: PatchSide, fraction: f64) -> SplineResult<(SurfaceCurvature, Vec3)> {
    let [u, v] = surface.domain();
    let interp = |d: [f64; 2]| d[0] + (d[1] - d[0]) * fraction;
    let (a, b, along_u) = match side {
        PatchSide::UMin => (u[0], interp(v), false),
        PatchSide::UMax => (u[1], interp(v), false),
        PatchSide::VMin => (interp(u), v[0], true),
        PatchSide::VMax => (interp(u), v[1], true),
    };
    let sample = curvature(surface, a, b)?;
    let tangent = if along_u { sample.du } else { sample.dv };
    let tangent = tangent.normalized().ok_or_else(|| SplineError::Invalid("undefined boundary tangent".into()))?;
    Ok((sample, tangent))
}

/// Match two explicitly corresponding, oriented boundary parametrisations.
/// reverse_second reverses the *sampling order*, never changes geometry.
/// Reparametrised coincident curves require a separate correspondence solver;
/// equal fractions alone do not find closest points. G3 is not evaluated.
pub fn measure_patch_join(
    a: &NurbsSurface,
    edge_a: PatchSide,
    b: &NurbsSurface,
    edge_b: PatchSide,
    reverse_second: bool,
    stations: usize,
    tolerances: JoinTolerances,
) -> SplineResult<PatchJoinReport> {
    tolerances.validate()?;
    if !(3..=MAX_JOIN_STATIONS).contains(&stations) {
        return invalid(format!("join stations must be 3..={MAX_JOIN_STATIONS}"));
    }
    let mut gap = JoinPeak::zero();
    let mut angle = JoinPeak::zero();
    let mut jump = JoinPeak::zero();
    for k in 0..stations {
        let fraction = k as f64 / (stations - 1) as f64;
        let (sa, tangent) = on_boundary(a, edge_a, fraction)?;
        let (sb, _) = on_boundary(b, edge_b, if reverse_second { 1.0 - fraction } else { fraction })?;
        let dot = sa.normal.dot(sb.normal);
        let normal_alignment = if dot < 0.0 { -1.0 } else { 1.0 };
        let across = sa.normal.cross(tangent).normalized().ok_or_else(|| SplineError::Invalid("undefined across-edge section".into()))?;
        let form = |s: &SurfaceCurvature, x: Vec3, y: Vec3| {
            s.second_form(x, y).ok_or_else(|| SplineError::Invalid("nonfinite boundary second fundamental form".into()))
        };
        let delta_tt = form(&sa, tangent, tangent)? - normal_alignment * form(&sb, tangent, tangent)?;
        let delta_tq = form(&sa, tangent, across)? - normal_alignment * form(&sb, tangent, across)?;
        let delta_qq = form(&sa, across, across)? - normal_alignment * form(&sb, across, across)?;
        let mismatch = (delta_tt * delta_tt + 2.0 * delta_tq * delta_tq + delta_qq * delta_qq).sqrt();
        if !mismatch.is_finite() {
            return invalid("curvature tensor mismatch overflows");
        }
        gap.keep(sa.position.dist(sb.position), fraction, sa.position);
        angle.keep(dot.abs().clamp(0.0, 1.0).acos().to_degrees(), fraction, sa.position);
        jump.keep(mismatch, fraction, sa.position);
    }
    let order = if gap.value > tolerances.gap_mm {
        SampledJoinOrder::Discontinuous
    } else if angle.value > tolerances.angle_deg {
        SampledJoinOrder::G0
    } else if jump.value > tolerances.curvature_per_mm {
        SampledJoinOrder::G1
    } else {
        SampledJoinOrder::G2
    };
    Ok(PatchJoinReport {
        samples: stations,
        gap_mm: gap,
        angle_deg: angle,
        shape_operator_jump_per_mm: jump,
        tolerances,
        sampled_order: order,
        method: METHOD,
        higher_order: "G3 not evaluated: matching third-order geometric jets requires additional well-defined correspondence",
        limitation: "discrete corresponding edge stations; results are local diagnostic evidence, not a whole-edge Class A certificate",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_1_SQRT_2;

    fn planar(x0: f64, x1: f64, tilt: f64) -> NurbsSurface {
        let point = |x: f64, y: f64| Vec3::new(x, y, tilt * (x - x0));
        NurbsSurface::new(
            [1, 1],
            [vec![0.0, 0.0, 1.0, 1.0], vec![0.0, 0.0, 1.0, 1.0]],
            &[point(x0, 0.0), point(x1, 0.0), point(x0, 1.0), point(x1, 1.0)],
            &[1.0; 4],
        )
        .unwrap()
    }

    fn cylinder(z0: f64, z1: f64) -> NurbsSurface {
        let r = 4.0;
        let row = |z: f64| [Vec3::new(r, 0.0, z), Vec3::new(r, r, z), Vec3::new(0.0, r, z)];
        let points = [row(z0), row(z1)].concat();
        NurbsSurface::new(
            [2, 1],
            [vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0], vec![0.0, 0.0, 1.0, 1.0]],
            &points,
            &[1.0, FRAC_1_SQRT_2, 1.0, 1.0, FRAC_1_SQRT_2, 1.0],
        )
        .unwrap()
    }

    fn quadratic_join() -> NurbsSurface {
        // x = 1 + u, z = u^2; tangent at u=0 is coplanar with the first surface,
        // but normal curvature across that seam is exactly 2 mm^-1.
        let pts = [
            Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.5, 0.0, 0.0), Vec3::new(2.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.5, 1.0, 0.0), Vec3::new(2.0, 1.0, 1.0),
        ];
        NurbsSurface::new(
            [2, 1],
            [vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0], vec![0.0, 0.0, 1.0, 1.0]],
            &pts,
            &[1.0; 6],
        )
        .unwrap()
    }

    fn tolerances() -> JoinTolerances {
        JoinTolerances { gap_mm: 1e-7, angle_deg: 0.05, curvature_per_mm: 1e-6 }
    }

    #[test]
    fn planar_patch_is_exactly_flat_and_has_zero_principal_curvature() {
        let plane = planar(0.0, 2.0, 0.0);
        for [u, v] in [[0.0, 0.0], [0.25, 0.75], [1.0, 1.0]] {
            let k = curvature(&plane, u, v).unwrap();
            assert!(k.k_min_per_mm.abs() < 1e-12 && k.k_max_per_mm.abs() < 1e-12, "{k:?}");
            assert!(k.mean_per_mm.abs() < 1e-12 && k.gaussian_per_mm2.abs() < 1e-12);
            assert!((k.normal - Vec3::Z).len() < 1e-12);
        }
        let scan = sample_grid(&plane, 5, 7).unwrap();
        assert_eq!(scan.samples.len(), 35);
        assert!(scan.max_abs_mean_gradient_per_mm2 < 1e-12);
        assert!(scan.max_neighbor_normal_angle_deg < 1e-8);
    }

    #[test]
    fn rational_quarter_cylinder_has_analytic_principal_curvature() {
        let cylinder = cylinder(0.0, 10.0);
        for [u, v] in [[0.0, 0.0], [0.25, 0.75], [0.5, 0.5], [1.0, 1.0]] {
            let k = curvature(&cylinder, u, v).unwrap();
            assert!(k.gaussian_per_mm2.abs() < 1e-11, "{k:?}");
            assert!((k.max_abs_principal_per_mm - 0.25).abs() < 1e-10, "{k:?}");
            assert!((k.mean_per_mm.abs() - 0.125).abs() < 1e-10, "{k:?}");
        }
        let grid = sample_grid(&cylinder, 9, 5).unwrap();
        assert!(grid.max_abs_mean_gradient_per_mm2 < 1e-9, "{grid:?}");
        assert!(grid.min_neighbor_spacing_mm > 0.0);
    }

    #[test]
    fn two_planes_and_adjacent_cylinder_patches_have_sampled_g2() {
        let a = planar(0.0, 1.0, 0.0);
        let b = planar(1.0, 2.0, 0.0);
        let plane = measure_patch_join(&a, PatchSide::UMax, &b, PatchSide::UMin, false, 33, tolerances()).unwrap();
        assert_eq!(plane.sampled_order, SampledJoinOrder::G2, "{plane:?}");
        assert!(plane.gap_mm.value < 1e-12 && plane.angle_deg.value < 1e-6);
        let a = cylinder(0.0, 10.0);
        let b = cylinder(10.0, 20.0);
        let rounded = measure_patch_join(&a, PatchSide::VMax, &b, PatchSide::VMin, false, 33, tolerances()).unwrap();
        assert_eq!(rounded.sampled_order, SampledJoinOrder::G2, "{rounded:?}");
        assert!(rounded.shape_operator_jump_per_mm.value < 1e-9);
    }

    #[test]
    fn mismatched_normal_and_curvature_are_not_called_smooth() {
        let a = planar(0.0, 1.0, 0.0);
        let tilted = planar(1.0, 2.0, 0.01);
        let r = measure_patch_join(&a, PatchSide::UMax, &tilted, PatchSide::UMin, false, 33, tolerances()).unwrap();
        assert_eq!(r.sampled_order, SampledJoinOrder::G0, "{r:?}");
        assert!(r.angle_deg.value > 0.5 && r.angle_deg.value < 0.6);
        let curved = quadratic_join();
        let r = measure_patch_join(&a, PatchSide::UMax, &curved, PatchSide::UMin, false, 33, tolerances()).unwrap();
        assert_eq!(r.sampled_order, SampledJoinOrder::G1, "{r:?}");
        assert!((r.shape_operator_jump_per_mm.value - 2.0).abs() < 1e-9, "{r:?}");
        let gap = planar(1.02, 2.02, 0.0);
        let r = measure_patch_join(&a, PatchSide::UMax, &gap, PatchSide::UMin, false, 33, tolerances()).unwrap();
        assert_eq!(r.sampled_order, SampledJoinOrder::Discontinuous);
        assert!((r.gap_mm.value - 0.02).abs() < 1e-12);
    }

    #[test]
    fn singular_metric_and_non_c2_knots_are_explicit_errors() {
        let singular = NurbsSurface::new(
            [1, 1],
            [vec![0.0, 0.0, 1.0, 1.0], vec![0.0, 0.0, 1.0, 1.0]],
            &[Vec3::ZERO; 4],
            &[1.0; 4],
        )
        .unwrap();
        assert!(curvature(&singular, 0.5, 0.5).is_err());
        // Quadratic with a simple interior knot: only C1 is guaranteed at u=0.5.
        let mut controls = Vec::new();
        for y in [0.0, 1.0] {
            for x in [0.0, 0.3, 0.7, 1.0] {
                controls.push(Vec3::new(x, y, x * x));
            }
        }
        let piecewise = NurbsSurface::new(
            [2, 1],
            [vec![0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0], vec![0.0, 0.0, 1.0, 1.0]],
            &controls,
            &[1.0; 8],
        )
        .unwrap();
        assert!(curvature(&piecewise, 0.5, 0.5).is_err(), "ambiguous left/right second derivative");
        assert!(curvature(&piecewise, 0.4, 0.5).is_ok());
        assert!(measure_patch_join(&piecewise, PatchSide::VMin, &piecewise, PatchSide::VMin, false, 5, tolerances()).is_err());
        assert!(sample_grid(&piecewise, 5, 5).is_err());
    }

    #[test]
    fn invalid_sampling_and_tolerances_are_refused() {
        let plane = planar(0.0, 1.0, 0.0);
        assert!(sample_grid(&plane, 130, 3).is_err());
        assert!(curvature(&plane, f64::NAN, 0.5).is_err());
        assert!(measure_patch_join(&plane, PatchSide::UMax, &plane, PatchSide::UMin, false, 2, tolerances()).is_err());
        assert!(measure_patch_join(&plane, PatchSide::UMax, &plane, PatchSide::UMin, false, 4, JoinTolerances { gap_mm: f64::NAN, ..tolerances() }).is_err());
    }
}
