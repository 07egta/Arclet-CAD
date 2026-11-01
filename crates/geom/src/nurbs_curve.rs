//! Editable rational curves, measured geometric joins, and bounded curve rebuilding.
//!
//! The basis and its derivatives come from [`crate::spline_space::Space`]. Rational derivatives
//! follow by differentiating `A = w C` three times. Join measurements use derivatives with
//! respect to arc length, so changing a regular curve's parameter speed does not change its
//! geometric continuity. These are local curve measurements, not a surface-quality certificate.
//!
//! Rebuilding fixes both endpoints and fits the remaining control points by Householder QR least
//! squares, without forming normal equations. Its deviation report compares sampled points to
//! the other sampled polyline in both directions. It is an estimate, not a Hausdorff bound: a
//! feature between samples can be missed. Long operations take a cancellation callback and
//! enforce shared work and allocation limits before allocating their dense matrices.

use crate::Vec3;
use crate::bspline::{MAX_SPLINE_DEGREE, SplineError};
use crate::spline_space::Space;

/// Most controls accepted in one editable curve.
pub const MAX_CURVE_CONTROLS: usize = 1024;
/// Most independently editable pieces in a curve chain.
pub const MAX_CHAIN_SEGMENTS: usize = 256;
/// A curve chain's aggregate control budget, including all of its pieces.
pub const MAX_CHAIN_CONTROLS: usize = 16_384;
/// Dense fitting is intentionally smaller than the editable-curve limit.
pub const MAX_REBUILD_CONTROLS: usize = 64;
pub const MAX_FIT_SAMPLES: usize = 2049;
pub const MAX_CHECK_SAMPLES: usize = 1025;
/// Conservative work units for QR sweeps, basis evaluation and directed distance pairs.
/// This is an algorithmic budget, not an instruction count or a wall-clock guarantee.
pub const MAX_REBUILD_WORK: usize = 8_000_000;

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum CurveError {
    #[error(transparent)]
    Spline(#[from] SplineError),
    #[error("curve operation cancelled")]
    Cancelled,
}

pub type CurveResult<T> = Result<T, CurveError>;

fn invalid<T>(message: impl Into<String>) -> CurveResult<T> {
    Err(SplineError::Invalid(message.into()).into())
}

fn too_large<T>(message: impl Into<String>) -> CurveResult<T> {
    Err(SplineError::TooLarge(message.into()).into())
}

fn check_cancelled(cancelled: &impl Fn() -> bool) -> CurveResult<()> {
    if cancelled() { Err(CurveError::Cancelled) } else { Ok(()) }
}

/// A clamped, positive-weight rational B-spline. Construction validates its private storage.
/// Control coordinates and weights remain available for an editor; rebuilds return a new curve.
#[derive(Clone, Debug, PartialEq)]
pub struct NurbsCurve {
    space: Space,
    points: Vec<Vec3>,
    weights: Vec<f64>,
}

impl NurbsCurve {
    pub fn new(degree: usize, knots: Vec<f64>, points: Vec<Vec3>, weights: Vec<f64>) -> CurveResult<Self> {
        if points.len() > MAX_CURVE_CONTROLS || weights.len() > MAX_CURVE_CONTROLS || knots.len() > MAX_CURVE_CONTROLS + MAX_SPLINE_DEGREE + 1 {
            return too_large(format!("at most {MAX_CURVE_CONTROLS} curve controls"));
        }
        let space = Space::new(degree, knots)?;
        if points.len() != space.count() || weights.len() != points.len() {
            return invalid("one point and positive weight per basis function are required");
        }
        if points.iter().any(|point| !point.is_finite()) || weights.iter().any(|w| !w.is_finite() || *w <= 0.0) {
            return invalid("curve points must be finite and weights must be finite and positive");
        }
        let [a, b] = space.domain();
        if !(b - a).is_finite() {
            return invalid("the curve's parameter range is too large");
        }
        let max_weight = weights.iter().copied().fold(0.0_f64, f64::max);
        if weights.iter().any(|weight| *weight / max_weight < f64::MIN_POSITIVE) {
            return invalid("the ratio between curve weights is too large");
        }
        Ok(Self { space, points, weights })
    }

    /// A non-rational curve with uniform interior knots on `[0, 1]`.
    pub fn uniform(degree: usize, points: Vec<Vec3>) -> CurveResult<Self> {
        let knots = uniform_knots(degree, points.len())?;
        let weights = vec![1.0; points.len()];
        Self::new(degree, knots, points, weights)
    }

    pub fn degree(&self) -> usize {
        self.space.degree()
    }

    pub fn knots(&self) -> &[f64] {
        self.space.knots()
    }

    pub fn control_points(&self) -> &[Vec3] {
        &self.points
    }

    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    pub fn domain(&self) -> [f64; 2] {
        self.space.domain()
    }

    /// A validated edit, leaving this curve unchanged if the edit is refused.
    pub fn with_control_point(&self, index: usize, point: Vec3) -> CurveResult<Self> {
        if !point.is_finite() || index >= self.points.len() {
            return invalid("a control edit needs a valid index and a finite point");
        }
        let mut edited = self.clone();
        edited.points[index] = point;
        Ok(edited)
    }

    /// Preserve the curve's image and parameter range, reversing its direction explicitly.
    pub fn reversed(&self) -> CurveResult<Self> {
        let [a, b] = self.domain();
        // Pin the repeated domain endpoints exactly. Reflecting every knot with a+(b-k)
        // loses b when the range is much larger than |b|, and can move an ordinary endpoint
        // by one ulp. For interior knots subtract the distance from the nearer endpoint.
        let knots = self
            .knots()
            .iter()
            .rev()
            .map(|k| {
                if *k == a {
                    b
                } else if *k == b {
                    a
                } else if *k - a <= b - *k {
                    b - (*k - a)
                } else {
                    a + (b - *k)
                }
            })
            .collect();
        Self::new(self.degree(), knots, self.points.iter().rev().copied().collect(), self.weights.iter().rev().copied().collect())
    }

    /// The same geometry with its parameter range mapped affinely onto `[0, 1]`. Control points
    /// and weights are unchanged, so geometric (arc-length) quantities are identical, but the raw
    /// derivatives stay representable for extremely wide or narrow parameter ranges.
    fn normalized(&self) -> CurveResult<Self> {
        let [a, b] = self.domain();
        let width = b - a;
        if !(width > 0.0 && width.is_finite()) {
            return invalid("the curve's parameter range must be positive and finite");
        }
        let knots = self.knots().iter().map(|k| ((*k - a) / width).clamp(0.0, 1.0)).collect();
        Self::new(self.degree(), knots, self.points.clone(), self.weights.clone())
    }

    /// Evaluate inside the domain; extrapolation and non-finite parameters are refused.
    pub fn eval(&self, t: f64) -> CurveResult<Vec3> {
        Ok(self.derivatives(t, 0)?[0])
    }

    /// Analytic derivatives through order three. Interior knots use the right-hand limit;
    /// the last endpoint uses the left-hand limit. A repeated knot may have different limits.
    pub fn jet(&self, t: f64) -> CurveResult<CurveJet> {
        let [point, d1, d2, d3] = self.derivatives(t, 3)?;
        Ok(CurveJet { point, d1, d2, d3 })
    }

    fn derivatives(&self, t: f64, order: usize) -> CurveResult<[Vec3; 4]> {
        let [a, b] = self.domain();
        if !t.is_finite() || t < a || t > b {
            return invalid("the curve parameter must be finite and inside its domain");
        }
        let (first, basis) = self.space.basis(t, 0)?;
        let count = basis.len();
        let weights = &self.weights[first..first + count];
        let points = &self.points[first..first + count];
        // A constant local span has exactly zero derivatives, even when extreme weights
        // or a tiny parameter interval would make individual basis derivatives overflow.
        // All controls in the chosen one-sided span are checked, not only nonzero bases.
        if let Some(point) = points.first().copied()
            && points.iter().all(|control| *control == point)
        {
            let mut result = [Vec3::ZERO; 4];
            result[0] = point;
            return Ok(result);
        }
        // First normalize the weight sum, before multiplying any coordinates. Computing x*w
        // and only then dividing by the weight sum loses small but representable endpoints
        // when both x and w are tiny. The normalized coefficients avoid that underflow.
        let max_weight = weights.iter().copied().fold(0.0_f64, f64::max);
        let weight_sum = weights.iter().zip(&basis).map(|(w, value)| (w / max_weight) * value).sum::<f64>();
        if !(weight_sum > 0.0 && weight_sum.is_finite()) {
            return invalid("curve evaluation has no finite positive weight");
        }
        let normalized_weights: Vec<f64> = weights.iter().map(|w| (w / max_weight) / weight_sum).collect();
        if normalized_weights.iter().any(|w| !w.is_finite()) {
            return invalid("curve evaluation needs an unrepresentable weight ratio");
        }
        // Evaluate the homogeneous numerator in a local coordinate system. Differentiating
        // absolute coordinates subtracts enormous, nearly equal values when a small part is
        // far from the origin. Even a constant rational curve then acquires a false tangent.
        // Choose an actually contributing control, so tiny clamped endpoints stay exact.
        let mut origin_index = 0;
        let mut largest_coefficient = -1.0_f64;
        for (index, (w, value)) in normalized_weights.iter().zip(&basis).enumerate() {
            let coefficient = w * value;
            if coefficient > largest_coefficient {
                origin_index = index;
                largest_coefficient = coefficient;
            }
        }
        let origin = points[origin_index];
        // A finite pair such as (-1e308, +1e308) can overflow on subtraction. Halving
        // both first keeps the local difference finite; recombine the position before
        // restoring that factor of two. Ordinary coordinates never take this path.
        let half_scale = points.iter().any(|point| !(*point - origin).is_finite());
        let mut weighted = [Vec3::ZERO; 4];
        let mut weight = [0.0; 4];
        for der in 0..=order {
            let values = if der == 0 { basis.clone() } else { self.space.basis(t, der)?.1 };
            let mut coordinates = [CompensatedSum::default(); 3];
            let mut denominator = CompensatedSum::default();
            for ((point, w), value) in points.iter().zip(&normalized_weights).zip(values) {
                let factor = w * value;
                // A zero basis coefficient contributes nothing, including at a repeated
                // endpoint. Avoid calculating a needless extreme local difference.
                if factor == 0.0 {
                    continue;
                }
                let local = if half_scale { *point * 0.5 - origin * 0.5 } else { *point - origin };
                let contribution = local * factor;
                coordinates[0].add(contribution.x);
                coordinates[1].add(contribution.y);
                coordinates[2].add(contribution.z);
                denominator.add(factor);
            }
            weighted[der] = Vec3::new(coordinates[0].total(), coordinates[1].total(), coordinates[2].total());
            weight[der] = denominator.total();
            if !weighted[der].is_finite() || !weight[der].is_finite() {
                return invalid("curve evaluation overflowed");
            }
        }
        if !(weight[0] > 0.0) {
            return invalid("curve evaluation has no finite positive weight");
        }
        let mut result = [Vec3::ZERO; 4];
        for k in 0..=order {
            let mut numerator = weighted[k];
            for i in 1..=k {
                let binomial = if k == 3 && i < 3 {
                    3.0
                } else if k == 2 && i == 1 {
                    2.0
                } else {
                    1.0
                };
                numerator -= result[k - i] * (binomial * weight[i]);
            }
            // Normalization makes this denominator close to one, not exactly one.
            // Apply the quotient recurrence to the local curve before adding the origin.
            result[k] = numerator / weight[0];
            if !result[k].is_finite() {
                return invalid("curve derivatives overflowed");
            }
        }
        result[0] = if half_scale { (origin * 0.5 + result[0]) * 2.0 } else { origin + result[0] };
        if half_scale {
            for derivative in result.iter_mut().take(order + 1).skip(1) {
                *derivative = *derivative * 2.0;
            }
        }
        if result.iter().take(order + 1).any(|value| !value.is_finite()) {
            return invalid("curve derivatives overflowed while restoring coordinates");
        }
        Ok(result)
    }
}

/// Neumaier compensated accumulation. A few positive and negative derivative terms can
/// differ substantially in size; retaining their low-order parts costs no heap allocation.
#[derive(Clone, Copy, Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let total = self.sum + value;
        self.correction += if self.sum.abs() >= value.abs() { (self.sum - total) + value } else { (value - total) + self.sum };
        self.sum = total;
    }

    fn total(self) -> f64 {
        self.sum + self.correction
    }
}

fn uniform_knots(degree: usize, count: usize) -> CurveResult<Vec<f64>> {
    if degree == 0 || degree > MAX_SPLINE_DEGREE || count <= degree {
        return invalid(format!("a curve needs degree 1..{MAX_SPLINE_DEGREE} and more controls than its degree"));
    }
    if count > MAX_CURVE_CONTROLS {
        return too_large(format!("at most {MAX_CURVE_CONTROLS} curve controls"));
    }
    let spans = count - degree;
    let mut knots = vec![0.0; degree + 1];
    knots.extend((1..spans).map(|i| i as f64 / spans as f64));
    knots.extend(std::iter::repeat_n(1.0, degree + 1));
    Ok(knots)
}

/// Derivatives with respect to the curve's own parameter, with no implied parameter speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveJet {
    pub point: Vec3,
    pub d1: Vec3,
    pub d2: Vec3,
    pub d3: Vec3,
}

/// Geometry invariant under a regular, orientation-preserving change of curve parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcLengthJet {
    pub point: Vec3,
    pub tangent: Vec3,
    /// `d²C/ds²`, in inverse millimetres, including its direction.
    pub curvature: Vec3,
    /// `d³C/ds³`, in inverse square millimetres. This includes curvature direction changes.
    pub curvature_change: Vec3,
}

impl CurveJet {
    /// Convert a regular curve jet to arc length. Zero-speed/cusp jets are unmeasurable;
    /// they are errors, not silently classified as smooth.
    pub fn arc_length(self) -> CurveResult<ArcLengthJet> {
        if [self.point, self.d1, self.d2, self.d3].iter().any(|v| !v.is_finite()) {
            return invalid("curve derivatives must be finite");
        }
        let speed = norm(self.d1);
        if !(speed > 0.0 && speed.is_finite()) {
            return invalid("geometric continuity needs a nonzero finite tangent");
        }
        let tangent = self.d1 / speed;
        let acceleration = (self.d2 / speed) / speed;
        let jerk = ((self.d3 / speed) / speed) / speed;
        let speed_change = tangent.dot(acceleration);
        let curvature = acceleration - tangent * speed_change;
        // a/v² = hT + K, j/v³ = (v''/v³)T + 3hK + dK/ds,
        // and T·dK/ds = -K·K, giving v''/v³ = T·(j/v³) + K·K.
        let curvature_change = jerk - curvature * (3.0 * speed_change) - tangent * (tangent.dot(jerk) + curvature.dot(curvature));
        if [tangent, curvature, curvature_change].iter().any(|v| !v.is_finite()) {
            return invalid("arc-length derivatives are outside the finite numerical range");
        }
        Ok(ArcLengthJet { point: self.point, tangent, curvature, curvature_change })
    }
}

/// The highest measured local curve order satisfying all lower-order tolerances.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GeometricContinuity {
    G0,
    G1,
    G2,
    G3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JoinTolerances {
    pub gap_mm: f64,
    pub tangent_angle_deg: f64,
    pub curvature_per_mm: f64,
    pub curvature_change_per_mm2: f64,
}

impl Default for JoinTolerances {
    fn default() -> Self {
        Self { gap_mm: 1e-6, tangent_angle_deg: 0.01, curvature_per_mm: 1e-6, curvature_change_per_mm2: 1e-6 }
    }
}

impl JoinTolerances {
    /// Check tolerances before preparing geometry or starting an asynchronous job.
    pub fn validate(self) -> CurveResult<()> {
        if [self.gap_mm, self.tangent_angle_deg, self.curvature_per_mm, self.curvature_change_per_mm2].iter().any(|v| !v.is_finite() || *v < 0.0)
            || self.tangent_angle_deg > 180.0
        {
            return invalid("join tolerances must be finite and nonnegative, with angle at most 180 degrees");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JoinReport {
    pub gap_mm: f64,
    pub tangent_angle_deg: f64,
    pub curvature_jump_per_mm: f64,
    pub curvature_change_jump_per_mm2: f64,
    /// `None` means the position tolerance failed. This is a local, tolerance-based result,
    /// not proof of exact equality and not a Class A surface designation.
    pub measured_order: Option<GeometricContinuity>,
}

/// Compare the end of `before` with the start of `after`, in their authored directions.
pub fn measure_join(before: &NurbsCurve, after: &NurbsCurve, tolerances: JoinTolerances) -> CurveResult<JoinReport> {
    tolerances.validate()?;
    // Measure on [0, 1] parameter ranges so the result never depends on how the curves were
    // parameterized (arc-length quantities are invariant under an affine parameter change).
    let a = before.normalized()?.jet(1.0)?.arc_length()?;
    let b = after.normalized()?.jet(0.0)?.arc_length()?;
    let gap_mm = norm(b.point - a.point);
    // atan2 retains precision near both zero and 180 degrees.
    let tangent_angle_deg = norm(a.tangent.cross(b.tangent)).atan2(a.tangent.dot(b.tangent).clamp(-1.0, 1.0)).to_degrees();
    let curvature_jump_per_mm = norm(b.curvature - a.curvature);
    let curvature_change_jump_per_mm2 = norm(b.curvature_change - a.curvature_change);
    if [gap_mm, tangent_angle_deg, curvature_jump_per_mm, curvature_change_jump_per_mm2].iter().any(|x| !x.is_finite()) {
        return invalid("join measurement overflowed");
    }
    let mut measured_order = None;
    for (passes, order) in [
        (gap_mm <= tolerances.gap_mm, GeometricContinuity::G0),
        (tangent_angle_deg <= tolerances.tangent_angle_deg, GeometricContinuity::G1),
        (curvature_jump_per_mm <= tolerances.curvature_per_mm, GeometricContinuity::G2),
        (curvature_change_jump_per_mm2 <= tolerances.curvature_change_per_mm2, GeometricContinuity::G3),
    ] {
        if !passes {
            break;
        }
        measured_order = Some(order);
    }
    Ok(JoinReport { gap_mm, tangent_angle_deg, curvature_jump_per_mm, curvature_change_jump_per_mm2, measured_order })
}

/// An ordered collection of independently editable rational spline pieces. No automatic
/// reordering, reversal, welding or smoothing is performed. Gaps remain observable in reports;
/// `closed` requests measuring the final-to-first join as well, not silently closing a gap.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveChain {
    segments: Vec<NurbsCurve>,
    closed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveChainJoin {
    pub before: usize,
    pub after: usize,
    pub report: JoinReport,
}

impl CurveChain {
    pub fn new(segments: Vec<NurbsCurve>, closed: bool) -> CurveResult<Self> {
        if segments.is_empty() || segments.len() > MAX_CHAIN_SEGMENTS {
            return too_large(format!("a curve chain needs 1..{MAX_CHAIN_SEGMENTS} segments"));
        }
        let controls = segments.iter().map(|curve| curve.control_points().len()).sum::<usize>();
        if controls > MAX_CHAIN_CONTROLS {
            return too_large(format!("a curve chain has at most {MAX_CHAIN_CONTROLS} controls in total"));
        }
        Ok(Self { segments, closed })
    }

    pub fn segments(&self) -> &[NurbsCurve] {
        &self.segments
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn joins(&self, tolerances: JoinTolerances, cancelled: impl Fn() -> bool) -> CurveResult<Vec<CurveChainJoin>> {
        tolerances.validate()?;
        check_cancelled(&cancelled)?;
        let count = self.segments.len() - usize::from(!self.closed);
        let mut joins = Vec::with_capacity(count);
        for before in 0..count {
            check_cancelled(&cancelled)?;
            let after = (before + 1) % self.segments.len();
            let report = measure_join(&self.segments[before], &self.segments[after], tolerances)?;
            joins.push(CurveChainJoin { before, after, report });
        }
        Ok(joins)
    }
}

/// Rebuilding outputs a non-rational uniform-clamped curve on `[0, 1]`. Both endpoint positions
/// are fixed; endpoint tangents and higher derivatives are not constrained by this operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RebuildOptions {
    pub degree: usize,
    pub control_points: usize,
    pub fit_samples: usize,
    pub check_samples: usize,
}

impl Default for RebuildOptions {
    fn default() -> Self {
        Self { degree: 3, control_points: 8, fit_samples: 129, check_samples: 257 }
    }
}

impl RebuildOptions {
    /// Validate counts and the combined work budget before preparing source geometry.
    pub fn validate(self) -> CurveResult<()> {
        if self.degree == 0 || self.degree > MAX_SPLINE_DEGREE || self.control_points <= self.degree {
            return invalid("rebuild degree must be 1..7 with more controls than the degree");
        }
        if self.control_points > MAX_REBUILD_CONTROLS || self.fit_samples > MAX_FIT_SAMPLES {
            return too_large(format!("rebuild supports at most {MAX_REBUILD_CONTROLS} controls and {MAX_FIT_SAMPLES} fit samples"));
        }
        if self.fit_samples < self.control_points {
            return invalid("rebuild needs at least as many fit samples as control points");
        }
        check_sample_count(self.check_samples)?;
        let columns = self.control_points - 2;
        let rows = self.fit_samples - 2;
        // Counts above are small before multiplying. Count matrix reflection and its right-
        // hand side, all directed distances, and every basis evaluation in sampling, assembly
        // and residuals. The input may be a degree-seven rational curve even for a linear
        // rebuild, so source evaluation uses the maximum supported degree here.
        let qr_work = rows * columns * (columns + 3);
        let check_work = 2 * self.check_samples * (self.check_samples - 1);
        let evaluation_cost = |degree: usize| (degree + 1) * (degree * (degree + 1) / 2 + degree + 17);
        let sampled_points = self.fit_samples + self.check_samples;
        let basis_work = sampled_points * evaluation_cost(MAX_SPLINE_DEGREE) + (sampled_points + rows) * evaluation_cost(self.degree);
        if qr_work + check_work + basis_work > MAX_REBUILD_WORK {
            return too_large("the requested rebuild exceeds its combined fitting and deviation work budget");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RebuildReport {
    pub curve: NurbsCurve,
    /// RMS difference at matching normalized parameters of the fit samples. This is not a
    /// closest-point geometric error; the separate deviation estimate measures that differently.
    pub parameter_rms_mm: f64,
    pub deviation: SampledDeviation,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampledDeviation {
    pub source_to_rebuilt_mm: f64,
    pub rebuilt_to_source_mm: f64,
    pub samples_per_curve: usize,
}

impl SampledDeviation {
    /// The larger directed sampled point-to-polyline distance, not a guaranteed error bound.
    pub fn max_mm(self) -> f64 {
        self.source_to_rebuilt_mm.max(self.rebuilt_to_source_mm)
    }
}

pub fn rebuild(source: &NurbsCurve, options: RebuildOptions, cancelled: impl Fn() -> bool) -> CurveResult<RebuildReport> {
    options.validate()?;
    check_cancelled(&cancelled)?;
    let source_points = sample(source, options.fit_samples, &cancelled)?;
    let start = source_points[0];
    let end = source_points[source_points.len() - 1];
    // Center and scale the right-hand side, so an ordinary shape far from the origin does not
    // lose its useful coordinates in the fitting arithmetic.
    let mut scale = 0.0_f64;
    for point in &source_points {
        let p = *point - start;
        if !p.is_finite() {
            return invalid("the source curve spans too large a coordinate range");
        }
        scale = scale.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
    }
    if scale == 0.0 {
        scale = 1.0;
    }
    let target = Space::new(options.degree, uniform_knots(options.degree, options.control_points)?)?;
    let columns = options.control_points - 2;
    let rows = options.fit_samples - 2;
    let mut matrix = vec![0.0; rows * columns];
    let mut rhs = Vec::with_capacity(rows);
    let end_scaled = (end - start) / scale;
    for row in 0..rows {
        check_cancelled(&cancelled)?;
        let t = (row + 1) as f64 / (options.fit_samples - 1) as f64;
        let (first, values) = target.basis(t, 0)?;
        let mut value = (source_points[row + 1] - start) / scale;
        for (offset, coefficient) in values.into_iter().enumerate() {
            let index = first + offset;
            if index == options.control_points - 1 {
                value -= end_scaled * coefficient;
            } else if index > 0 {
                matrix[row * columns + index - 1] = coefficient;
            }
        }
        rhs.push(value);
    }
    let fitted = least_squares(matrix, rhs, rows, columns, &cancelled)?;
    let mut controls = Vec::with_capacity(options.control_points);
    controls.push(start);
    controls.extend(fitted.into_iter().map(|point| start + point * scale));
    controls.push(end);
    let curve = NurbsCurve::uniform(options.degree, controls)?;
    let mut residual_norm = 0.0_f64;
    for (i, point) in source_points.iter().enumerate() {
        check_cancelled(&cancelled)?;
        let t = i as f64 / (options.fit_samples - 1) as f64;
        let distance = norm(curve.eval(t)? - *point);
        residual_norm = residual_norm.hypot(distance);
    }
    let parameter_rms_mm = residual_norm / (options.fit_samples as f64).sqrt();
    if !parameter_rms_mm.is_finite() {
        return invalid("rebuild residual is outside the finite numerical range");
    }
    let deviation = sampled_deviation(source, &curve, options.check_samples, &cancelled)?;
    check_cancelled(&cancelled)?;
    Ok(RebuildReport { curve, parameter_rms_mm, deviation })
}

/// Uniform-parameter samples against the other curve's sampled polyline, in both directions.
/// Finite sampling can miss features; neither direction is a certified Hausdorff bound.
pub fn sampled_deviation(source: &NurbsCurve, rebuilt: &NurbsCurve, samples: usize, cancelled: impl Fn() -> bool) -> CurveResult<SampledDeviation> {
    check_sample_count(samples)?;
    check_cancelled(&cancelled)?;
    let a = sample(source, samples, &cancelled)?;
    let b = sample(rebuilt, samples, &cancelled)?;
    let source_to_rebuilt_mm = directed_distance(&a, &b, &cancelled)?;
    let rebuilt_to_source_mm = directed_distance(&b, &a, &cancelled)?;
    Ok(SampledDeviation { source_to_rebuilt_mm, rebuilt_to_source_mm, samples_per_curve: samples })
}

fn check_sample_count(samples: usize) -> CurveResult<()> {
    if !(2..=MAX_CHECK_SAMPLES).contains(&samples) {
        return too_large(format!("deviation measurement needs 2..{MAX_CHECK_SAMPLES} samples"));
    }
    Ok(())
}

fn sample(curve: &NurbsCurve, count: usize, cancelled: &impl Fn() -> bool) -> CurveResult<Vec<Vec3>> {
    let [a, b] = curve.domain();
    let mut points = Vec::with_capacity(count);
    for i in 0..count {
        check_cancelled(cancelled)?;
        let t = if i + 1 == count { b } else { a + (b - a) * (i as f64 / (count - 1) as f64) };
        points.push(curve.eval(t)?);
    }
    Ok(points)
}

/// In-place QR with normalized Householder reflectors; no normal equations and no n-by-n
/// inverse. The validated caller caps rows, columns, and their combined work before allocation.
fn least_squares(mut matrix: Vec<f64>, mut rhs: Vec<Vec3>, rows: usize, columns: usize, cancelled: &impl Fn() -> bool) -> CurveResult<Vec<Vec3>> {
    for column in 0..columns {
        check_cancelled(cancelled)?;
        let mut reflector: Vec<f64> = (column..rows).map(|row| matrix[row * columns + column]).collect();
        let length = reflector.iter().fold(0.0_f64, |total, x| total.hypot(*x));
        if !length.is_finite() || length <= 1e-12 {
            return invalid("rebuild samples do not determine stable control points");
        }
        let alpha = -length.copysign(reflector[0]);
        reflector[0] -= alpha;
        let length = reflector.iter().fold(0.0_f64, |total, x| total.hypot(*x));
        for value in &mut reflector {
            *value /= length;
        }
        for target in column..columns {
            let dot = reflector.iter().enumerate().map(|(i, v)| v * matrix[(column + i) * columns + target]).sum::<f64>();
            for (i, value) in reflector.iter().enumerate() {
                matrix[(column + i) * columns + target] -= 2.0 * value * dot;
            }
        }
        let dot = reflector.iter().enumerate().fold(Vec3::ZERO, |acc, (i, v)| acc + rhs[column + i] * *v);
        for (i, value) in reflector.iter().enumerate() {
            rhs[column + i] -= dot * (2.0 * value);
        }
        matrix[column * columns + column] = alpha;
    }
    let mut answer = vec![Vec3::ZERO; columns];
    for row in (0..columns).rev() {
        check_cancelled(cancelled)?;
        let mut value = rhs[row];
        for column in row + 1..columns {
            value -= answer[column] * matrix[row * columns + column];
        }
        answer[row] = value / matrix[row * columns + row];
        if !answer[row].is_finite() {
            return invalid("rebuild did not produce finite control points");
        }
    }
    Ok(answer)
}

fn directed_distance(points: &[Vec3], polyline: &[Vec3], cancelled: &impl Fn() -> bool) -> CurveResult<f64> {
    let mut maximum = 0.0_f64;
    for point in points {
        check_cancelled(cancelled)?;
        let mut minimum = f64::INFINITY;
        for segment in polyline.windows(2) {
            let a = segment[0];
            let b = segment[1];
            let ab = b - a;
            let length = norm(ab);
            let nearest = if length == 0.0 {
                a
            } else {
                let unit = ab / length;
                let projection = (*point - a).dot(unit);
                if !length.is_finite() || !projection.is_finite() {
                    return invalid("deviation measurement spans too large a coordinate range");
                }
                a + unit * projection.clamp(0.0, length)
            };
            let distance = norm(*point - nearest);
            if !distance.is_finite() {
                return invalid("deviation measurement overflowed");
            }
            minimum = minimum.min(distance);
        }
        maximum = maximum.max(minimum);
    }
    Ok(maximum)
}

fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn line(a: Vec3, b: Vec3) -> NurbsCurve {
        NurbsCurve::uniform(1, vec![a, b]).unwrap()
    }

    fn quarter_circle() -> NurbsCurve {
        NurbsCurve::new(
            2,
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            vec![Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y],
            vec![1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0],
        )
        .unwrap()
    }

    fn near(a: Vec3, b: Vec3, tolerance: f64) {
        assert!(norm(a - b) <= tolerance, "{a:?} != {b:?}");
    }

    #[test]
    fn rational_circle_has_analytic_third_order_geometry() {
        let circle = quarter_circle();
        for i in 0..=40 {
            let jet = circle.jet(f64::from(i) / 40.0).unwrap().arc_length().unwrap();
            assert!((norm(jet.point) - 1.0).abs() < 1e-13);
            near(jet.curvature, -jet.point, 1e-12);
            near(jet.curvature_change, -jet.tangent, 1e-11);
        }
        let jet = circle.jet(0.0).unwrap();
        near(jet.d1, Vec3::Y * std::f64::consts::SQRT_2, 1e-12);
        near(jet.d2, Vec3::new(-2.0, 2.0 * std::f64::consts::SQRT_2 - 2.0, 0.0), 1e-12);
        let reversed = circle.reversed().unwrap();
        for i in 0..=16 {
            let t = f64::from(i) / 16.0;
            near(circle.eval(t).unwrap(), reversed.eval(1.0 - t).unwrap(), 1e-13);
        }
    }

    #[test]
    fn geometric_order_ignores_positive_nonlinear_parameter_changes() {
        // The same parabola on either side of the origin. On the right, x = 2t+t² and
        // y = (2t+t²)², so both the first and second parameter derivatives change at the join.
        let left = NurbsCurve::uniform(2, vec![Vec3::new(-1.0, 1.0, 0.0), Vec3::new(-0.5, 0.0, 0.0), Vec3::ZERO]).unwrap();
        let right = NurbsCurve::uniform(
            4,
            vec![Vec3::ZERO, Vec3::new(0.5, 0.0, 0.0), Vec3::new(7.0 / 6.0, 2.0 / 3.0, 0.0), Vec3::new(2.0, 3.0, 0.0), Vec3::new(3.0, 9.0, 0.0)],
        )
        .unwrap();
        assert!(norm(left.jet(1.0).unwrap().d1 - right.jet(0.0).unwrap().d1) > 0.5);
        let report = measure_join(&left, &right, JoinTolerances::default()).unwrap();
        assert_eq!(report.measured_order, Some(GeometricContinuity::G3));
        assert!(report.curvature_change_jump_per_mm2 < 1e-12, "{report:?}");
    }

    #[test]
    fn geometric_orders_distinguish_turn_curvature_and_curvature_change() {
        let before = line(-Vec3::X, Vec3::ZERO);
        let turn = line(Vec3::ZERO, Vec3::Y);
        let parabola = NurbsCurve::uniform(2, vec![Vec3::ZERO, Vec3::X * 0.5, Vec3::new(1.0, 1.0, 0.0)]).unwrap();
        let cubic = NurbsCurve::uniform(3, vec![Vec3::ZERO, Vec3::X / 3.0, Vec3::X * (2.0 / 3.0), Vec3::new(1.0, 1.0, 0.0)]).unwrap();
        let straight = line(Vec3::ZERO, Vec3::X * 2.0);
        for (after, order) in [
            (&turn, GeometricContinuity::G0),
            (&parabola, GeometricContinuity::G1),
            (&cubic, GeometricContinuity::G2),
            (&straight, GeometricContinuity::G3),
        ] {
            assert_eq!(measure_join(&before, after, JoinTolerances::default()).unwrap().measured_order, Some(order));
        }
        let gap = line(Vec3::X, Vec3::X * 2.0);
        assert_eq!(measure_join(&before, &gap, JoinTolerances::default()).unwrap().measured_order, None);
        let reversed = straight.reversed().unwrap();
        assert!(measure_join(&straight, &reversed, JoinTolerances::default()).unwrap().tangent_angle_deg > 179.9);
        let point = line(Vec3::ZERO, Vec3::ZERO);
        assert!(measure_join(&before, &point, JoinTolerances::default()).is_err());
    }

    #[test]
    fn curve_chain_preserves_authored_order_and_reports_closure_gap() {
        let pieces = vec![line(Vec3::ZERO, Vec3::X), line(Vec3::X, Vec3::X * 2.0)];
        let open = CurveChain::new(pieces.clone(), false).unwrap();
        let closed = CurveChain::new(pieces, true).unwrap();
        let joins = open.joins(JoinTolerances::default(), || false).unwrap();
        assert_eq!(joins.len(), 1);
        assert_eq!(joins[0].report.measured_order, Some(GeometricContinuity::G3));
        let joins = closed.joins(JoinTolerances::default(), || false).unwrap();
        assert_eq!(joins.len(), 2);
        assert_eq!((joins[1].before, joins[1].after), (1, 0));
        assert_eq!(joins[1].report.gap_mm, 2.0);
        assert_eq!(joins[1].report.measured_order, None);
        assert!(matches!(closed.joins(JoinTolerances::default(), || true), Err(CurveError::Cancelled)));
    }

    #[test]
    fn rebuild_reproduces_cubic_and_keeps_endpoints_exact() {
        let source = NurbsCurve::uniform(
            3,
            vec![Vec3::new(1000.0, 20.0, 3.0), Vec3::new(1010.0, 40.0, 5.0), Vec3::new(1020.0, -10.0, 7.0), Vec3::new(1030.0, 0.0, 9.0)],
        )
        .unwrap();
        let report =
            rebuild(&source, RebuildOptions { control_points: 4, fit_samples: 65, check_samples: 65, ..RebuildOptions::default() }, || false)
                .unwrap();
        assert_eq!(report.curve.eval(0.0).unwrap(), source.eval(0.0).unwrap());
        assert_eq!(report.curve.eval(1.0).unwrap(), source.eval(1.0).unwrap());
        assert!(report.parameter_rms_mm < 1e-10, "{report:?}");
        assert!(report.deviation.max_mm() < 1e-10);
        for (a, b) in report.curve.control_points().iter().zip(source.control_points()) {
            near(*a, *b, 1e-10);
        }
    }

    #[test]
    fn more_controls_improve_rational_curve_fit_without_certifying_error() {
        let source = quarter_circle();
        let options = RebuildOptions { fit_samples: 129, check_samples: 257, ..RebuildOptions::default() };
        let coarse = rebuild(&source, RebuildOptions { control_points: 4, ..options }, || false).unwrap();
        let fine = rebuild(&source, options, || false).unwrap();
        assert!(fine.parameter_rms_mm < coarse.parameter_rms_mm * 0.1, "{} vs {}", fine.parameter_rms_mm, coarse.parameter_rms_mm);
        assert!(fine.deviation.max_mm() < 0.001, "{:?}", fine.deviation);
        assert_eq!(fine.deviation.samples_per_curve, 257);
        assert!(fine.curve.weights().iter().all(|w| *w == 1.0));
        assert_eq!(fine.curve.control_points().first(), source.control_points().first());
        assert_eq!(fine.curve.control_points().last(), source.control_points().last());
    }

    #[test]
    fn two_way_deviation_detects_bulge_and_ignores_line_parameter_speed() {
        let straight = line(Vec3::ZERO, Vec3::X);
        let bulge = NurbsCurve::uniform(2, vec![Vec3::ZERO, Vec3::new(0.5, 1.0, 0.0), Vec3::X]).unwrap();
        let report = sampled_deviation(&straight, &bulge, 65, || false).unwrap();
        assert!(report.source_to_rebuilt_mm > 0.3);
        assert!((report.rebuilt_to_source_mm - 0.5).abs() < 1e-12);
        let rational_line = NurbsCurve::new(1, vec![0.0, 0.0, 1.0, 1.0], vec![Vec3::ZERO, Vec3::X], vec![1.0, 8.0]).unwrap();
        let report = rebuild(&rational_line, RebuildOptions { degree: 1, control_points: 2, fit_samples: 33, check_samples: 33 }, || false).unwrap();
        assert!(report.parameter_rms_mm > 0.1);
        assert!(report.deviation.max_mm() < 1e-12);
    }

    #[test]
    fn reversing_preserves_exact_domains_and_endpoint_geometry() {
        for [a, b] in [[-1e16, 1.0], [-0.1, 0.2], [1e16, 1e16 + 8.0]] {
            let original = NurbsCurve::new(1, vec![a, a, b, b], vec![Vec3::ZERO, Vec3::X], vec![1.0; 2]).unwrap();
            let reversed = original.reversed().unwrap();
            assert_eq!(reversed.domain(), [a, b]);
            assert_eq!(reversed.knots(), original.knots());
            near(reversed.eval(a).unwrap(), original.eval(b).unwrap(), 1e-14);
            near(reversed.eval(b).unwrap(), original.eval(a).unwrap(), 1e-14);
            assert_eq!(reversed.reversed().unwrap(), original);
        }
        let original = NurbsCurve::new(1, vec![-0.1, -0.1, 0.0, 0.2, 0.2], vec![Vec3::ZERO, Vec3::Y, Vec3::X], vec![1.0; 3]).unwrap();
        let reversed = original.reversed().unwrap();
        assert_eq!(reversed.domain(), original.domain());
        for index in 0..=20 {
            let t = -0.1 + 0.3 * f64::from(index) / 20.0;
            near(original.eval(t).unwrap(), reversed.eval(0.1 - t).unwrap(), 1e-14);
        }
    }

    #[test]
    fn tiny_weighted_endpoints_do_not_silently_underflow() {
        let curve = NurbsCurve::new(1, vec![0.0, 0.0, 1.0, 1.0], vec![Vec3::X * 1e-100, Vec3::X], vec![f64::MIN_POSITIVE, 1.0]).unwrap();
        assert_eq!(curve.eval(0.0).unwrap(), curve.control_points()[0]);
        assert_eq!(curve.eval(1.0).unwrap(), curve.control_points()[1]);
        // The position is representable but third derivatives at this endpoint are not.
        assert!(curve.jet(0.0).is_err());
        let curve = NurbsCurve::new(1, vec![0.0, 0.0, 1.0, 1.0], vec![Vec3::X * 1e-230, Vec3::X], vec![1e-100, 1.0]).unwrap();
        assert_eq!(curve.eval(0.0).unwrap(), curve.control_points()[0]);
        assert_eq!(curve.jet(0.0).unwrap().point, curve.eval(0.0).unwrap());
    }

    #[test]
    fn bounded_operations_cancel_before_or_during_their_work() {
        let source = quarter_circle();
        assert!(matches!(rebuild(&source, RebuildOptions::default(), || true), Err(CurveError::Cancelled)));
        let checks = Cell::new(0);
        let result = rebuild(&source, RebuildOptions::default(), || {
            checks.set(checks.get() + 1);
            checks.get() > 20
        });
        assert!(matches!(result, Err(CurveError::Cancelled)));
        let checks = Cell::new(0);
        let result = least_squares(vec![1.0, 0.0, 0.0, 1.0, 1.0, 1.0], vec![Vec3::X; 3], 3, 2, &|| {
            checks.set(checks.get() + 1);
            checks.get() > 1
        });
        assert!(matches!(result, Err(CurveError::Cancelled)));
        let checks = Cell::new(0);
        let result = sampled_deviation(&source, &source, 65, || {
            checks.set(checks.get() + 1);
            checks.get() > 140
        });
        assert!(matches!(result, Err(CurveError::Cancelled)));
    }

    #[test]
    fn a_constant_rational_curve_never_acquires_a_tangent_from_translation() {
        let point = Vec3::new(1e12, -2e12, 3e12);
        let curve = NurbsCurve::new(2, vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0], vec![point; 3], vec![1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0]).unwrap();
        for index in 0..=100 {
            let jet = curve.jet(f64::from(index) / 100.0).unwrap();
            assert_eq!(jet.point, point);
            assert_eq!(jet.d1, Vec3::ZERO);
            assert_eq!(jet.d2, Vec3::ZERO);
            assert_eq!(jet.d3, Vec3::ZERO);
            assert!(jet.arc_length().is_err(), "a constant curve is not a regular G3 join");
        }
    }

    #[test]
    fn rational_derivatives_are_translation_invariant_at_large_world_coordinates() {
        let circle = quarter_circle();
        for offset in [Vec3::new(1e9, 1e9, 1e9), Vec3::new(1e12, -1e12, 1e12), Vec3::new(1e14, 1e14, 1e14)] {
            let moved = NurbsCurve::new(
                circle.degree(),
                circle.knots().to_vec(),
                circle.control_points().iter().map(|point| *point + offset).collect(),
                circle.weights().to_vec(),
            )
            .unwrap();
            for index in 0..=40 {
                let parameter = f64::from(index) / 40.0;
                let reference = circle.jet(parameter).unwrap();
                let measured = moved.jet(parameter).unwrap();
                near(measured.d1, reference.d1, 1e-12);
                near(measured.d2, reference.d2, 1e-12);
                near(measured.d3, reference.d3, 1e-11);
                near(measured.arc_length().unwrap().curvature, reference.arc_length().unwrap().curvature, 1e-12);
                near(measured.arc_length().unwrap().curvature_change, reference.arc_length().unwrap().curvature_change, 1e-11);
            }
        }
    }

    #[test]
    fn common_weight_scaling_preserves_the_rational_jet() {
        let circle = quarter_circle();
        for scale in [1e-200, 1e200] {
            let scaled = NurbsCurve::new(
                circle.degree(),
                circle.knots().to_vec(),
                circle.control_points().to_vec(),
                circle.weights().iter().map(|weight| *weight * scale).collect(),
            )
            .unwrap();
            for index in 0..=40 {
                let parameter = f64::from(index) / 40.0;
                let a = circle.jet(parameter).unwrap();
                let b = scaled.jet(parameter).unwrap();
                near(a.point, b.point, 1e-13);
                near(a.d1, b.d1, 1e-12);
                near(a.d2, b.d2, 1e-12);
                near(a.d3, b.d3, 1e-11);
            }
        }
    }

    #[test]
    fn clamped_tiny_endpoints_are_exact_in_both_directions() {
        let curve = NurbsCurve::new(1, vec![0.0, 0.0, 1.0, 1.0], vec![Vec3::X * 1e-230, Vec3::X], vec![1e-100, 1.0]).unwrap();
        let reversed = curve.reversed().unwrap();
        assert_eq!(curve.eval(0.0).unwrap(), Vec3::X * 1e-230);
        assert_eq!(reversed.eval(1.0).unwrap(), Vec3::X * 1e-230);
        assert_eq!(curve.eval(1.0).unwrap(), Vec3::X);
        assert_eq!(reversed.eval(0.0).unwrap(), Vec3::X);
    }

    #[test]
    fn finite_extreme_coordinates_do_not_overflow_only_from_recentering() {
        let curve = NurbsCurve::new(1, vec![0.0, 0.0, 4.0, 4.0], vec![Vec3::X * -1e308, Vec3::X * 1e308], vec![1.0; 2]).unwrap();
        let jet = curve.jet(2.0).unwrap();
        assert_eq!(jet.point, Vec3::ZERO);
        assert!((jet.d1.x / 5e307 - 1.0).abs() < 1e-14);
        assert_eq!(jet.d2, Vec3::ZERO);
        assert_eq!(jet.d3, Vec3::ZERO);
        assert_eq!(curve.eval(0.0).unwrap(), curve.control_points()[0]);
        assert_eq!(curve.eval(4.0).unwrap(), curve.control_points()[1]);
    }

    #[test]
    fn compensated_accumulation_keeps_small_terms_between_large_opposites() {
        let mut sum = CompensatedSum::default();
        for value in [1e16, 1.0, -1e16] {
            sum.add(value);
        }
        assert_eq!(sum.total(), 1.0);
    }

    #[test]
    fn hostile_geometry_and_work_requests_are_refused_without_mutation() {
        let source = quarter_circle();
        let original = source.clone();
        assert!(source.with_control_point(usize::MAX, Vec3::ZERO).is_err());
        assert!(source.with_control_point(0, Vec3::new(f64::NAN, 0.0, 0.0)).is_err());
        assert_eq!(source, original);
        assert!(source.eval(f64::NAN).is_err());
        assert!(source.jet(1.001).is_err());
        assert!(NurbsCurve::uniform(usize::MAX, vec![Vec3::ZERO; 2]).is_err());
        assert!(NurbsCurve::uniform(1, vec![Vec3::ZERO; MAX_CURVE_CONTROLS + 1]).is_err());
        for weights in [vec![0.0, 1.0], vec![-1.0, 1.0], vec![f64::INFINITY, 1.0], vec![f64::MIN_POSITIVE, f64::MAX]] {
            assert!(NurbsCurve::new(1, vec![0.0, 0.0, 1.0, 1.0], vec![Vec3::ZERO, Vec3::X], weights).is_err());
        }
        assert!(NurbsCurve::new(1, vec![-f64::MAX, -f64::MAX, f64::MAX, f64::MAX], vec![Vec3::ZERO, Vec3::X], vec![1.0; 2]).is_err());
        let tiny_domain =
            NurbsCurve::new(1, vec![0.0, 0.0, f64::MIN_POSITIVE, f64::MIN_POSITIVE], vec![Vec3::ZERO, Vec3::X * 4.0], vec![1.0; 2]).unwrap();
        assert!(tiny_domain.jet(0.0).is_err());
        for options in [
            RebuildOptions { degree: usize::MAX, ..RebuildOptions::default() },
            RebuildOptions { control_points: usize::MAX, ..RebuildOptions::default() },
            RebuildOptions { fit_samples: usize::MAX, ..RebuildOptions::default() },
            RebuildOptions { check_samples: usize::MAX, ..RebuildOptions::default() },
            RebuildOptions { fit_samples: 3, ..RebuildOptions::default() },
            // QR and distance pairs alone fit the budget; basis evaluation pushes it over.
            RebuildOptions { degree: 7, control_points: 53, fit_samples: 2049, check_samples: 1025 },
            RebuildOptions {
                control_points: MAX_REBUILD_CONTROLS,
                fit_samples: MAX_FIT_SAMPLES,
                check_samples: MAX_CHECK_SAMPLES,
                ..RebuildOptions::default()
            },
        ] {
            assert!(rebuild(&source, options, || false).is_err());
        }
        assert!(sampled_deviation(&source, &source, 1, || false).is_err());
        assert!(measure_join(&source, &source, JoinTolerances { gap_mm: f64::NAN, ..JoinTolerances::default() }).is_err());
        assert!(CurveChain::new(Vec::new(), false).is_err());
        assert!(CurveChain::new(vec![source.clone(); MAX_CHAIN_SEGMENTS + 1], false).is_err());
        let large = NurbsCurve::uniform(1, vec![Vec3::ZERO; MAX_CURVE_CONTROLS]).unwrap();
        assert!(CurveChain::new(vec![large; MAX_CHAIN_CONTROLS / MAX_CURVE_CONTROLS + 1], false).is_err());
    }
    #[test]
    fn a_constant_span_is_evaluable_on_a_tiny_parameter_domain() {
        let point = Vec3::new(1e300, -1e300, 0.0);
        let curve = NurbsCurve::new(2, vec![0.0, 0.0, 0.0, 1e-200, 1e-200, 1e-200], vec![point; 3], vec![1e-100, 1e100, 1.0]).unwrap();
        for parameter in [0.0, 0.5e-200, 1e-200] {
            let jet = curve.jet(parameter).unwrap();
            assert_eq!(jet.point, point);
            assert_eq!(jet.d1, Vec3::ZERO);
            assert_eq!(jet.d2, Vec3::ZERO);
            assert_eq!(jet.d3, Vec3::ZERO);
        }
    }

    #[test]
    fn join_measurement_does_not_depend_on_the_parameter_range() {
        let cubic = |scale: f64, offset: Vec3| {
            let mut knots = vec![0.0; 4];
            knots.extend(std::iter::repeat_n(scale, 4));
            let points = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 0.5), Vec3::new(3.0, 2.5, -1.0), Vec3::new(4.0, 0.0, 2.0)];
            NurbsCurve::new(3, knots, points.iter().map(|p| *p + offset).collect(), vec![1.0; 4]).unwrap()
        };
        let reference = {
            let first = cubic(1.0, Vec3::new(0.0, 0.0, 0.0));
            let second = cubic(1.0, Vec3::new(4.0, 0.0, 2.0));
            measure_join(&first, &second, JoinTolerances::default()).unwrap()
        };
        for scale in [1e-104, 1e-50, 1e100, 1e160, 1e300] {
            let first = cubic(scale, Vec3::new(0.0, 0.0, 0.0));
            let second = cubic(scale, Vec3::new(4.0, 0.0, 2.0));
            let report = measure_join(&first, &second, JoinTolerances::default()).unwrap();
            assert!(
                (report.curvature_jump_per_mm - reference.curvature_jump_per_mm).abs() <= 1e-9 * reference.curvature_jump_per_mm.max(1.0),
                "scale {scale}"
            );
            assert!(
                (report.curvature_change_jump_per_mm2 - reference.curvature_change_jump_per_mm2).abs()
                    <= 1e-9 * reference.curvature_change_jump_per_mm2.max(1.0),
                "scale {scale}"
            );
            assert_eq!(report.measured_order, reference.measured_order, "scale {scale}");
        }
    }

    /// Deterministic xorshift generator for the hostile-input tests below (no external crate).
    struct Rng(u64);

    impl Rng {
        fn word(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.word() % n as u64) as usize
        }

        fn unit(&mut self) -> f64 {
            (self.word() >> 11) as f64 / (1u64 << 53) as f64
        }

        /// Ordinary numbers mixed with specials, subnormals, extreme magnitudes and raw bit patterns.
        fn hostile(&mut self) -> f64 {
            const SPECIAL: [f64; 14] = [
                0.0,
                -0.0,
                1.0,
                -1.0,
                f64::NAN,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::MIN_POSITIVE,
                -f64::MIN_POSITIVE,
                5e-324,
                f64::MAX,
                -f64::MAX,
                1e300,
                -1e300,
            ];
            match self.below(4) {
                0 => SPECIAL[self.below(SPECIAL.len())],
                1 => f64::from_bits(self.word()),
                2 => (self.unit() * 2.0 - 1.0) * 10f64.powi(self.below(600) as i32 - 300),
                _ => self.unit(),
            }
        }

        fn point(&mut self) -> Vec3 {
            Vec3::new(self.hostile(), self.hostile(), self.hostile())
        }

        /// Finite coordinates at a random magnitude between 1e-300 and 1e300.
        fn finite_point(&mut self) -> Vec3 {
            let scale = 10f64.powi(self.below(601) as i32 - 300);
            Vec3::new(self.unit() * 2.0 - 1.0, self.unit() * 2.0 - 1.0, self.unit() * 2.0 - 1.0) * scale
        }
    }

    fn finite_jet(jet: CurveJet) {
        assert!([jet.point, jet.d1, jet.d2, jet.d3].iter().all(|v| v.is_finite()), "{jet:?}");
    }

    fn finite_join(report: JoinReport) {
        let values = [report.gap_mm, report.tangent_angle_deg, report.curvature_jump_per_mm, report.curvature_change_jump_per_mm2];
        assert!(values.iter().all(|v| v.is_finite()), "{report:?}");
    }

    fn finite_rebuild(report: &RebuildReport) {
        assert!(report.parameter_rms_mm.is_finite(), "{report:?}");
        assert!(report.deviation.max_mm().is_finite(), "{report:?}");
        assert!(report.curve.control_points().iter().all(|p| p.is_finite()), "{report:?}");
    }

    /// Every public query on `curve` must return finite values or a typed error. Parameters outside
    /// the domain and non-finite parameters must always be refused.
    fn exercise_curve(curve: &NurbsCurve, rng: &mut Rng, deep: bool) {
        let [a, b] = curve.domain();
        let width = b - a;
        let mut params = vec![
            a,
            b,
            a + width * 0.5,
            a - width * 1e-9,
            b + width * 1e-9,
            a - width,
            b + width,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            -f64::MAX,
            0.0,
            f64::MIN_POSITIVE,
        ];
        for _ in 0..6 {
            params.push(rng.hostile());
            params.push(a + width * rng.unit());
        }
        for t in params {
            let inside = t.is_finite() && t >= a && t <= b;
            if let Ok(point) = curve.eval(t) {
                assert!(inside, "eval accepted t = {t:e} outside [{a:e}, {b:e}]");
                assert!(point.is_finite(), "{point:?}");
            }
            if !inside {
                assert!(curve.eval(t).is_err() && curve.jet(t).is_err(), "t = {t:e}");
            }
            if let Ok(jet) = curve.jet(t) {
                assert!(inside);
                finite_jet(jet);
                if let Ok(arc) = jet.arc_length() {
                    assert!([arc.point, arc.tangent, arc.curvature, arc.curvature_change].iter().all(|v| v.is_finite()), "{arc:?}");
                }
            }
        }
        let reversed = curve.reversed();
        if let Ok(reversed) = &reversed {
            assert_eq!(reversed.domain(), curve.domain());
            if let Ok(point) = reversed.eval(a + width * 0.25) {
                assert!(point.is_finite());
            }
        }
        let count = curve.control_points().len();
        for index in [0, count.saturating_sub(1), count, usize::MAX, rng.below(count + 1)] {
            let point = rng.point();
            if let Ok(edited) = curve.with_control_point(index, point) {
                assert!(index < count && point.is_finite());
                assert_eq!(edited.control_points().len(), count);
            }
        }
        let other = reversed.unwrap_or_else(|_| curve.clone());
        let tolerances = [
            JoinTolerances::default(),
            JoinTolerances { gap_mm: f64::NAN, ..JoinTolerances::default() },
            JoinTolerances { tangent_angle_deg: 180.5, ..JoinTolerances::default() },
            JoinTolerances { curvature_per_mm: f64::INFINITY, ..JoinTolerances::default() },
            JoinTolerances {
                gap_mm: rng.hostile(),
                tangent_angle_deg: rng.hostile(),
                curvature_per_mm: rng.hostile(),
                curvature_change_per_mm2: rng.hostile(),
            },
        ];
        for tolerance in tolerances {
            let result = measure_join(curve, &other, tolerance);
            if tolerance.validate().is_err() {
                assert!(result.is_err(), "{tolerance:?}");
            }
            if let Ok(report) = result {
                finite_join(report);
            }
        }
        for closed in [false, true] {
            if let Ok(chain) = CurveChain::new(vec![curve.clone(), other.clone()], closed) {
                if let Ok(joins) = chain.joins(JoinTolerances::default(), || false) {
                    joins.into_iter().for_each(|join| finite_join(join.report));
                }
                assert!(matches!(chain.joins(JoinTolerances::default(), || true), Err(CurveError::Cancelled)));
            }
        }
        assert!(matches!(rebuild(curve, RebuildOptions::default(), || true), Err(CurveError::Cancelled)));
        if deep {
            let degrees = [0, 1, 2, 3, 7, 8, usize::MAX];
            let controls = [0, 1, 2, 3, 4, 8, 9, usize::MAX];
            let fits = [0, 1, 2, 3, 5, 9, 33, 129, 2049, usize::MAX];
            let checks = [0, 1, 2, 9, 65, 257, usize::MAX];
            for _ in 0..3 {
                let options = RebuildOptions {
                    degree: degrees[rng.below(degrees.len())],
                    control_points: controls[rng.below(controls.len())],
                    fit_samples: fits[rng.below(fits.len())],
                    check_samples: checks[rng.below(checks.len())],
                };
                if let Ok(report) = rebuild(curve, options, || false) {
                    finite_rebuild(&report);
                }
            }
            if let Ok(report) = rebuild(curve, RebuildOptions::default(), || false) {
                finite_rebuild(&report);
            }
        }
    }

    #[test]
    fn random_hostile_constructor_inputs_never_panic_or_return_nonfinite_geometry() {
        let mut rng = Rng(0x5EED_0F_C0DE_1234);
        let mut accepted = 0usize;
        for case in 0..4000 {
            // Mode 0: well-formed skeleton. Mode 1: skeleton with one hostile knot. Mode 2: all hostile.
            let mode = rng.below(3);
            let degree = if rng.below(8) == 0 { rng.below(12) } else { 1 + rng.below(7) };
            let (a, b) = if mode == 2 {
                (rng.hostile(), rng.hostile())
            } else {
                let scale = 10f64.powi(rng.below(601) as i32 - 300);
                let a = (rng.unit() - 0.5) * scale;
                (a, a + (0.1 + rng.unit()) * scale)
            };
            let mut inner: Vec<f64> = (0..rng.below(9)).map(|_| if mode == 2 { rng.hostile() } else { a + (b - a) * rng.unit() }).collect();
            if mode != 2 {
                inner.sort_by(f64::total_cmp);
                if let Some(&knot) = inner.first() {
                    for _ in 0..rng.below(degree.max(1)) {
                        inner.push(knot);
                    }
                    inner.sort_by(f64::total_cmp);
                }
            }
            let mut knots = vec![a; degree + 1];
            knots.extend(inner);
            knots.extend(std::iter::repeat_n(b, degree + 1));
            if mode == 1 {
                let index = rng.below(knots.len());
                knots[index] = rng.hostile();
            }
            let available = knots.len().saturating_sub(degree + 1);
            let count = if rng.below(5) == 0 { rng.below(available + 3) } else { available };
            let points: Vec<Vec3> = (0..count).map(|_| if mode == 2 { rng.point() } else { rng.finite_point() }).collect();
            let weights: Vec<f64> = (0..count)
                .map(|_| {
                    if mode == 2 {
                        match rng.below(4) {
                            0 => rng.hostile(),
                            1 => 10f64.powi(rng.below(41) as i32 - 20),
                            2 => rng.unit() + 0.5,
                            _ => f64::MIN_POSITIVE,
                        }
                    } else if rng.below(8) == 0 {
                        // Occasionally a weight ratio at the representable limit.
                        f64::MIN_POSITIVE
                    } else {
                        10f64.powi(rng.below(41) as i32 - 20)
                    }
                })
                .collect();
            if let Ok(curve) = NurbsCurve::new(degree, knots, points, weights) {
                accepted += 1;
                exercise_curve(&curve, &mut rng, case % 4 == 0);
            }
        }
        assert!(accepted > 200, "the generator should reach evaluation often, got {accepted}");
    }

    #[test]
    fn degenerate_and_extremely_short_curves_are_refused_or_finite() {
        let mut rng = Rng(0xD3E7_E4A7_E000_0001);
        let points = [
            Vec3::ZERO,
            Vec3::new(1e300, -1e300, 1e300),
            Vec3::new(f64::MIN_POSITIVE, 0.0, 0.0),
            Vec3::new(1e-310, 1e-310, 0.0),
            Vec3::new(-f64::MAX, f64::MAX, 0.0),
        ];
        for degree in 0..=8 {
            for count in 0..=degree + 6 {
                for point in points {
                    match NurbsCurve::uniform(degree, vec![point; count]) {
                        Ok(curve) => {
                            exercise_curve(&curve, &mut rng, true);
                            // A constant curve has no tangent, so no join may be measured as smooth.
                            assert!(measure_join(&curve, &curve, JoinTolerances::default()).is_err(), "degree {degree}, count {count}");
                        }
                        Err(_) => assert!(count <= degree || degree == 0 || degree > MAX_SPLINE_DEGREE || count > MAX_CURVE_CONTROLS),
                    }
                }
            }
        }
        // Two distinct points a subnormal distance apart, and two points at opposite extremes.
        for end in [Vec3::X * f64::MIN_POSITIVE, Vec3::X * 5e-324, Vec3::new(-f64::MAX, 0.0, 0.0), Vec3::new(f64::MAX, f64::MAX, 0.0)] {
            let short = line(Vec3::ZERO, end);
            exercise_curve(&short, &mut rng, true);
            let degenerate_weights = NurbsCurve::new(2, vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0], vec![Vec3::ZERO, end, end], vec![1.0, 1.0, 1.0]);
            if let Ok(curve) = degenerate_weights {
                exercise_curve(&curve, &mut rng, false);
            }
        }
        // Identical controls with tiny rational weights around a non-identical middle control.
        let tiny = NurbsCurve::new(
            2,
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            vec![Vec3::new(1e300, 0.0, 0.0), Vec3::new(1e300, 0.0, 0.0), Vec3::new(1e300, 0.0, 0.0)],
            vec![f64::MIN_POSITIVE, 1.0, f64::MIN_POSITIVE],
        );
        if let Ok(curve) = tiny {
            exercise_curve(&curve, &mut rng, true);
        }
    }

    #[test]
    fn extreme_scales_and_offsets_are_finite_or_refused() {
        let mut rng = Rng(0xC0FF_EE00_DEAD_BEEF);
        let base = quarter_circle();
        for scale in [5e-324, f64::MIN_POSITIVE, 1e-300, 1e-150, 1.0, 1e150, 1e300, f64::MAX / 4.0, f64::MAX] {
            for offset in [Vec3::ZERO, Vec3::new(scale, -scale, 0.0), Vec3::new(-f64::MAX, f64::MAX, 0.0) * 0.5] {
                let points = base.control_points().iter().map(|p| *p * scale + offset).collect();
                if let Ok(curve) = NurbsCurve::new(base.degree(), base.knots().to_vec(), points, base.weights().to_vec()) {
                    exercise_curve(&curve, &mut rng, scale >= 1.0);
                }
            }
        }
        exercise_curve(&base, &mut rng, true);
    }

    #[test]
    fn rebuild_count_and_work_extremes_are_bounded() {
        let source = quarter_circle();
        let cases = [
            RebuildOptions { degree: 1, control_points: 2, fit_samples: 2, check_samples: 2 },
            RebuildOptions { degree: 1, control_points: 2, fit_samples: 3, check_samples: 2 },
            RebuildOptions { degree: 1, control_points: 3, fit_samples: 3, check_samples: 2 },
            RebuildOptions { degree: 7, control_points: 8, fit_samples: 8, check_samples: 2 },
            RebuildOptions { degree: 1, control_points: MAX_REBUILD_CONTROLS, fit_samples: 2, check_samples: 2 },
            RebuildOptions { degree: 3, control_points: 8, fit_samples: 129, check_samples: 1025 },
            RebuildOptions { degree: 3, control_points: 8, fit_samples: 129, check_samples: 1026 },
            RebuildOptions { degree: 3, control_points: 8, fit_samples: 129, check_samples: 1 },
        ];
        for options in cases {
            match rebuild(&source, options, || false) {
                Ok(report) => finite_rebuild(&report),
                Err(error) => assert!(!matches!(error, CurveError::Cancelled), "{options:?}"),
            }
        }
        assert!(sampled_deviation(&source, &source, 2, || false).is_ok());
        assert!(sampled_deviation(&source, &source, 0, || false).is_err());
        assert!(sampled_deviation(&source, &source, MAX_CHECK_SAMPLES + 1, || false).is_err());
    }

    #[test]
    fn cancellation_at_every_checkpoint_returns_cancelled_or_finite() {
        let curve = quarter_circle();
        let chain = CurveChain::new(vec![curve.clone(), curve.reversed().unwrap(), curve.clone()], true).unwrap();
        for limit in 0..600 {
            let calls = Cell::new(0usize);
            let cancel = || {
                calls.set(calls.get() + 1);
                calls.get() > limit
            };
            match rebuild(&curve, RebuildOptions::default(), cancel) {
                Ok(report) => finite_rebuild(&report),
                Err(error) => assert!(matches!(error, CurveError::Cancelled), "limit {limit}: {error:?}"),
            }
            let calls = Cell::new(0usize);
            let cancel = || {
                calls.set(calls.get() + 1);
                calls.get() > limit
            };
            match sampled_deviation(&curve, &curve, 65, cancel) {
                Ok(deviation) => assert!(deviation.max_mm().is_finite()),
                Err(error) => assert!(matches!(error, CurveError::Cancelled), "limit {limit}: {error:?}"),
            }
            let calls = Cell::new(0usize);
            let cancel = || {
                calls.set(calls.get() + 1);
                calls.get() > limit
            };
            match chain.joins(JoinTolerances::default(), cancel) {
                Ok(joins) => joins.into_iter().for_each(|join| finite_join(join.report)),
                Err(error) => assert!(matches!(error, CurveError::Cancelled), "limit {limit}: {error:?}"),
            }
        }
        let calls = Cell::new(0usize);
        assert!(
            rebuild(&curve, RebuildOptions::default(), || {
                calls.set(calls.get() + 1);
                false
            })
            .is_ok()
        );
        assert!(calls.get() > 0);
    }
}
