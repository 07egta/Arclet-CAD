//! Stateless, UI-independent screen-space snapping on editable geometry.
//!
//! Analytic snaps are computed from line and circle equations, never a rendered mesh.
//! NURBS points are evaluated on the actual rational curve but numerical closest-point
//! searches are **estimates**, not guaranteed global minima. Explicit polylines are
//! classified as DisplayApproximate. All inputs are world space after the object's
//! affine transform; circle transforms must be similarities (nonuniform scaling would
//! turn the circle into an ellipse).
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use solidcraft_geom::{Vec3, nurbs_curve::NurbsCurve};

use super::{SnapKind, snap_points};
use crate::{Sel, Session};

const MAX_OBJECTS: usize = 4096;
const MAX_PAIRS: usize = 128;
const MAX_CANDIDATES: usize = 1024;
const MAX_TRACKING: usize = 64;
const SEARCH_STEPS: usize = 64;

/// The provenance of a snap position. Evaluated means a point on the exact
/// curve found by a numerical search, not a certified closest-point solution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapAccuracy {
    Analytic,
    Evaluated,
    DisplayApproximate,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapClipDepth {
    ZeroToOne,
    MinusOneToOne,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapScreenPoint {
    pub x: f64,
    pub y: f64,
}

impl SnapScreenPoint {
    fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapView {
    /// Row-major projection × view matrix; clip = matrix × [world.xyz, 1].
    pub clip_from_world: [[f64; 4]; 4],
    /// The viewport size in physical pixels, not logical egui points.
    pub viewport_px: [f64; 2],
    pub depth: SnapClipDepth,
}

#[derive(Clone, Copy)]
struct Projected {
    pixel: SnapScreenPoint,
    depth: f64,
    w: f64,
}

impl SnapView {
    fn validate(self) -> Result<(), SnapError> {
        if self.clip_from_world.iter().flatten().any(|v| !v.is_finite())
            || self.viewport_px.iter().any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err(SnapError::Invalid("invalid view matrix or viewport"));
        }
        Ok(())
    }

    fn project(self, p: Vec3) -> Option<Projected> {
        if !p.is_finite() {
            return None;
        }
        let q = [p.x, p.y, p.z, 1.0];
        let mut clip = [0.0; 4];
        for (row, out) in self.clip_from_world.iter().zip(&mut clip) {
            *out = row.iter().zip(q).map(|(a, b)| a * b).sum();
        }
        let w = clip[3];
        if !(w > 0.0 && w.is_finite()) {
            return None;
        }
        let x = clip[0] / w;
        let y = clip[1] / w;
        let depth = clip[2] / w;
        let min_depth = match self.depth {
            SnapClipDepth::ZeroToOne => 0.0,
            SnapClipDepth::MinusOneToOne => -1.0,
        };
        if !x.is_finite() || !y.is_finite() || !(min_depth..=1.0).contains(&depth) {
            return None;
        }
        let pixel = SnapScreenPoint {
            x: (x + 1.0) * self.viewport_px[0] * 0.5,
            y: (1.0 - y) * self.viewport_px[1] * 0.5,
        };
        (pixel.x.is_finite() && pixel.y.is_finite()).then_some(Projected { pixel, depth, w })
    }
}

/// World position = translation + x * local.x + y * local.y + z * local.z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapTransform {
    pub translation: Vec3,
    pub x: Vec3,
    pub y: Vec3,
    pub z: Vec3,
}

impl Default for SnapTransform {
    fn default() -> Self {
        Self { translation: Vec3::ZERO, x: Vec3::X, y: Vec3::Y, z: Vec3::Z }
    }
}

impl SnapTransform {
    pub fn point(self, p: Vec3) -> Vec3 {
        self.translation + self.x * p.x + self.y * p.y + self.z * p.z
    }

    fn vector(self, v: Vec3) -> Vec3 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }

    fn valid(self) -> bool {
        [self.translation, self.x, self.y, self.z].iter().all(|p| p.is_finite())
    }
}

/// Input geometry. A polyline is only an approximation; use Segment, Circle, Arc,
/// or Nurbs for authoring snaps, and Anchor to wrap legacy picked points.
#[derive(Clone, Debug, PartialEq)]
pub enum SnapGeometry {
    Anchor { at: Vec3, kind: SnapKind, accuracy: SnapAccuracy },
    Segment { a: Vec3, b: Vec3 },
    Circle { center: Vec3, normal: Vec3, radius: f64, x_axis: Vec3 },
    Arc { center: Vec3, normal: Vec3, radius: f64, start: Vec3, sweep: f64 },
    Nurbs(NurbsCurve),
    Polyline(Vec<Vec3>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapObject {
    /// Stable, unique identifier, independent of rendering and iteration order.
    pub id: String,
    pub geometry: SnapGeometry,
    pub transform: SnapTransform,
    pub visible: bool,
    pub locked: bool,
}

impl SnapObject {
    pub fn new(id: impl Into<String>, geometry: SnapGeometry) -> Self {
        Self { id: id.into(), geometry, transform: SnapTransform::default(), visible: true, locked: false }
    }
}

/// Creates UI-independent candidates from the existing selection API. Because the
/// legacy picker can fit curves and face centres to meshes, provenance is not
/// guaranteed and all converted anchors are labelled DisplayApproximate.
pub fn legacy_snap_objects(session: &Session, selections: &[Sel]) -> Vec<SnapObject> {
    selections.iter().enumerate().flat_map(|(selection, sel)| {
        snap_points(session, sel).into_iter().enumerate().map(move |(index, point)| {
            SnapObject::new(
                format!("selection:{selection}:{index}:{}", point.body),
                SnapGeometry::Anchor { at: point.at, kind: point.kind, accuracy: SnapAccuracy::DisplayApproximate },
            )
        })
    }).collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapPlane {
    pub origin: Vec3,
    /// These axes must be perpendicular and nonzero.
    pub x: Vec3,
    pub y: Vec3,
}

impl SnapPlane {
    fn basis(self) -> Option<(Vec3, Vec3)> {
        if !self.origin.is_finite() || !self.x.is_finite() || !self.y.is_finite() {
            return None;
        }
        let x = unit(self.x)?;
        let y = unit(self.y)?;
        (x.dot(y).abs() <= 1e-10).then_some((x, y))
    }

    fn coords(self, point: Vec3) -> Option<(f64, f64)> {
        let (x, y) = self.basis()?;
        let delta = point - self.origin;
        (delta.is_finite()).then_some((delta.dot(x), delta.dot(y)))
    }

    fn position(self, x: f64, y: f64) -> Option<Vec3> {
        let (u, v) = self.basis()?;
        let p = self.origin + u * x + v * y;
        p.is_finite().then_some(p)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapInference {
    /// Reference for perpendiculars and tangencies; also the inference origin.
    pub reference: Option<Vec3>,
    /// World-space cursor projected onto the active construction plane.
    /// It can come from a ray–plane intersection computed by the caller.
    pub cursor_world: Option<Vec3>,
    pub plane: Option<SnapPlane>,
    /// A positive angle step <= 180 degrees, measured on the active plane.
    pub angle_step_deg: Option<f64>,
    /// Acquired points from which temporary horizontal/vertical tracking starts.
    pub acquired: Vec<Vec3>,
}

impl Default for SnapInference {
    fn default() -> Self {
        Self { reference: None, cursor_world: None, plane: None, angle_step_deg: None, acquired: Vec::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapSettings {
    pub enabled: BTreeSet<SnapKind>,
    /// Physical-pixel cursor tolerance, independent of zoom and DPI.
    pub radius_px: f64,
    pub include_hidden: bool,
    pub include_locked: bool,
    /// Maximum returned candidates, capped internally.
    pub max_candidates: usize,
}

impl Default for SnapSettings {
    fn default() -> Self {
        let enabled = [
            SnapKind::Centre, SnapKind::Middle, SnapKind::End,
            SnapKind::FaceCentre, SnapKind::Vertex, SnapKind::Intersection,
            SnapKind::Nearest, SnapKind::Perpendicular, SnapKind::Tangent,
            SnapKind::Quadrant, SnapKind::Projected, SnapKind::Horizontal,
            SnapKind::Vertical, SnapKind::Angle, SnapKind::Tracking,
        ].into_iter().collect();
        Self { enabled, radius_px: 12.0, include_hidden: false, include_locked: false, max_candidates: 64 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapQuery {
    pub cursor_px: SnapScreenPoint,
    pub view: SnapView,
    pub settings: SnapSettings,
    pub inference: SnapInference,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SnapCandidate {
    pub at: Vec3,
    pub kind: SnapKind,
    pub object_id: String,
    pub distance_px: f64,
    /// Normalized device depth, from the provided projection matrix.
    pub depth: f64,
    pub accuracy: SnapAccuracy,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SnapResult {
    /// Sorted nearest-first by a stable quantized screen-distance, kind priority,
    /// accuracy, real distance, depth, identifier, and world coordinates.
    pub candidates: Vec<SnapCandidate>,
    /// True if pairwise intersections were skipped for excessive input or
    /// if the candidate list reached its configured limit.
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SnapError {
    #[error("invalid snap request: {0}")]
    Invalid(&'static str),
    #[error("too many snap objects")]
    TooManyObjects,
}

fn unit(v: Vec3) -> Option<Vec3> {
    let length = v.x.hypot(v.y).hypot(v.z);
    (length > 0.0 && length.is_finite()).then_some(v / length).filter(|u| u.is_finite())
}

fn priority(k: SnapKind) -> u8 {
    match k {
        SnapKind::Intersection => 0,
        SnapKind::End | SnapKind::Vertex => 1,
        SnapKind::Middle | SnapKind::Centre => 2,
        SnapKind::Quadrant | SnapKind::FaceCentre => 3,
        SnapKind::Perpendicular | SnapKind::Tangent => 4,
        SnapKind::Horizontal | SnapKind::Vertical | SnapKind::Tracking => 5,
        SnapKind::Angle | SnapKind::Projected => 6,
        SnapKind::Nearest => 7,
    }
}

fn accuracy_rank(a: SnapAccuracy) -> u8 {
    match a {
        SnapAccuracy::Analytic => 0,
        SnapAccuracy::Evaluated => 1,
        SnapAccuracy::DisplayApproximate => 2,
    }
}

fn cmp_candidate(a: &SnapCandidate, b: &SnapCandidate) -> std::cmp::Ordering {
    // Quantization yields transitive ordering even when two distances differ by
    // less than a quarter pixel. A pairwise epsilon comparison would not.
    let bucket = |d: f64| (d * 4.0).floor() as u64;
    bucket(a.distance_px).cmp(&bucket(b.distance_px))
        .then_with(|| priority(a.kind).cmp(&priority(b.kind)))
        .then_with(|| accuracy_rank(a.accuracy).cmp(&accuracy_rank(b.accuracy)))
        .then_with(|| a.distance_px.total_cmp(&b.distance_px))
        .then_with(|| a.depth.total_cmp(&b.depth))
        .then_with(|| a.object_id.cmp(&b.object_id))
        .then_with(|| a.kind.cmp(&b.kind))
        .then_with(|| a.at.x.total_cmp(&b.at.x))
        .then_with(|| a.at.y.total_cmp(&b.at.y))
        .then_with(|| a.at.z.total_cmp(&b.at.z))
}

struct Collector<'a> {
    query: &'a SnapQuery,
    candidates: Vec<SnapCandidate>,
}

impl Collector<'_> {
    fn offer(&mut self, id: &str, kind: SnapKind, at: Vec3, accuracy: SnapAccuracy) {
        if !self.query.settings.enabled.contains(&kind) {
            return;
        }
        let Some(p) = self.query.view.project(at) else { return };
        let distance_px = p.pixel.distance(self.query.cursor_px);
        if distance_px.is_finite() && distance_px <= self.query.settings.radius_px {
            self.candidates.push(SnapCandidate { at, kind, object_id: id.into(), distance_px, depth: p.depth, accuracy });
        }
    }
}

/// Analytic line and circle snaps plus evaluated (not tessellated) NURBS
/// suggestions. No UI or graphics dependency. Invalid individual objects are
/// ignored; invalid query settings cause a typed error.
pub fn query_snaps(objects: &[SnapObject], query: &SnapQuery) -> Result<SnapResult, SnapError> {
    query.view.validate()?;
    if objects.len() > MAX_OBJECTS {
        return Err(SnapError::TooManyObjects);
    }
    let settings = &query.settings;
    if !query.cursor_px.x.is_finite() || !query.cursor_px.y.is_finite()
        || !settings.radius_px.is_finite() || !(0.0..=4096.0).contains(&settings.radius_px)
        || settings.max_candidates == 0 || settings.max_candidates > MAX_CANDIDATES
        || query.inference.acquired.len() > MAX_TRACKING
        || query.inference.acquired.iter().any(|p| !p.is_finite())
        || query.inference.reference.is_some_and(|p| !p.is_finite())
        || query.inference.cursor_world.is_some_and(|p| !p.is_finite())
        || query.inference.plane.is_some_and(|p| p.basis().is_none())
        || query.inference.angle_step_deg.is_some_and(|x| !x.is_finite() || !(0.0 < x && x <= 180.0))
    {
        return Err(SnapError::Invalid("nonfinite coordinates, plane, angle or invalid settings"));
    }
    let mut out = Collector { query, candidates: Vec::new() };
    let active: Vec<(&SnapObject, WorldGeometry)> = objects.iter()
        .filter(|o| (o.visible || settings.include_hidden) && (!o.locked || settings.include_locked))
        .filter_map(|o| Some((o, world_geometry(o)?))).collect();

    for (obj, shape) in &active {
        offer_shape(&mut out, &obj.id, shape);
    }
    let mut truncated = false;
    if settings.enabled.contains(&SnapKind::Intersection) {
        if active.len() > MAX_PAIRS {
            truncated = true; // no order-dependent arbitrary truncation
        } else {
            for i in 0..active.len() {
                for j in i + 1..active.len() {
                    for (point, accuracy) in intersections(&active[i].1, &active[j].1) {
                        let id = if active[i].0.id <= active[j].0.id {
                            format!("{}&{}", active[i].0.id, active[j].0.id)
                        } else {
                            format!("{}&{}", active[j].0.id, active[i].0.id)
                        };
                        out.offer(&id, SnapKind::Intersection, point, accuracy);
                    }
                }
            }
        }
    }
    offer_inference(&mut out);

    out.candidates.sort_by(cmp_candidate);
    // Deduplicate *identical* proposals only. Coincident points on separate
    // named objects are kept so the user can cycle between their references.
    out.candidates.dedup_by(|a, b| a.object_id == b.object_id && a.kind == b.kind && a.at == b.at);
    if out.candidates.len() > settings.max_candidates {
        truncated = true;
        out.candidates.truncate(settings.max_candidates);
    }
    Ok(SnapResult { candidates: out.candidates, truncated })
}

#[derive(Clone)]
enum WorldGeometry {
    Anchor(Vec3, SnapKind, SnapAccuracy),
    Segment(Vec3, Vec3, SnapAccuracy),
    Round(Round),
    Nurbs(NurbsCurve, SnapTransform),
    Polyline(Vec<Vec3>),
}

#[derive(Clone, Copy)]
struct Round {
    center: Vec3,
    x: Vec3,
    y: Vec3,
    normal: Vec3,
    radius: f64,
    start_angle: f64,
    sweep: f64,
    closed: bool,
}

impl Round {
    fn at(self, theta: f64) -> Vec3 {
        self.center + (self.x * theta.cos() + self.y * theta.sin()) * self.radius
    }

    fn contains(self, point: Vec3) -> bool {
        if self.closed {
            return true;
        }
        let delta = point - self.center;
        let theta = delta.dot(self.y).atan2(delta.dot(self.x));
        let turn = if self.sweep >= 0.0 {
            (theta - self.start_angle).rem_euclid(std::f64::consts::TAU)
        } else {
            (self.start_angle - theta).rem_euclid(std::f64::consts::TAU)
        };
        let tol = 1e-11;
        turn <= self.sweep.abs() + tol || (std::f64::consts::TAU - turn) <= tol
    }

    fn endpoints(self) -> (Vec3, Vec3) {
        (self.at(self.start_angle), self.at(self.start_angle + self.sweep))
    }
}

fn world_geometry(obj: &SnapObject) -> Option<WorldGeometry> {
    let tr = obj.transform;
    if !tr.valid() || obj.id.is_empty() || obj.id.len() > 512 {
        return None;
    }
    match &obj.geometry {
        SnapGeometry::Anchor { at, kind, accuracy } =>
            tr.point(*at).is_finite().then_some(WorldGeometry::Anchor(tr.point(*at), *kind, *accuracy)),
        SnapGeometry::Segment { a, b } => {
            let (a, b) = (tr.point(*a), tr.point(*b));
            (a.is_finite() && b.is_finite()).then_some(WorldGeometry::Segment(a, b, SnapAccuracy::Analytic))
        }
        SnapGeometry::Polyline(points) => {
            if points.len() < 2 || points.len() > 16_384 {
                return None;
            }
            let world: Vec<Vec3> = points.iter().map(|p| tr.point(*p)).collect();
            world.iter().all(|p| p.is_finite()).then_some(WorldGeometry::Polyline(world))
        }
        SnapGeometry::Nurbs(c) => Some(WorldGeometry::Nurbs(c.clone(), tr)),
        SnapGeometry::Circle { center, normal, radius, x_axis } => {
            world_round(tr, *center, *normal, *radius, *x_axis, 0.0, std::f64::consts::TAU, true).map(WorldGeometry::Round)
        }
        SnapGeometry::Arc { center, normal, radius, start, sweep } => {
            world_round(tr, *center, *normal, *radius, *start, 0.0, *sweep, false).map(WorldGeometry::Round)
        }
    }
}

fn world_round(tr: SnapTransform, center: Vec3, normal: Vec3, radius: f64, start: Vec3,
               start_angle: f64, sweep: f64, closed: bool) -> Option<Round> {
    if !radius.is_finite() || radius <= 0.0 || !sweep.is_finite()
        || sweep == 0.0 || sweep.abs() > std::f64::consts::TAU + 1e-10 {
        return None;
    }
    let axis = unit(normal)?;
    let x = unit(start)?;
    if x.dot(axis).abs() > 1e-10 {
        return None;
    }
    let y = unit(axis.cross(x))?;
    let (wx, wy) = (tr.vector(x), tr.vector(y));
    let (lx, ly) = (wx.x.hypot(wx.y).hypot(wx.z), wy.x.hypot(wy.y).hypot(wy.z));
    // Reject nonuniform scale and shear rather than snapping to a false circle.
    if !(lx > 0.0 && lx.is_finite() && ly > 0.0 && ly.is_finite())
        || (lx - ly).abs() > 1e-9 * lx.max(ly)
        || wx.dot(wy).abs() > 1e-9 * lx * ly {
        return None;
    }
    let xx = unit(wx)?;
    let yy = unit(wy)?;
    let nn = unit(xx.cross(yy))?;
    let center = tr.point(center);
    let radius = radius * lx;
    (center.is_finite() && radius.is_finite() && radius > 0.0).then_some(Round {
        center, x: xx, y: yy, normal: nn, radius, start_angle, sweep, closed,
    })
}

fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}

fn segment_closest(a: Vec3, b: Vec3, point: Vec3) -> Option<Vec3> {
    let d = b - a;
    let scale = d.x.abs().max(d.y.abs()).max(d.z.abs());
    if !(scale > 0.0 && scale.is_finite()) {
        return None;
    }
    let direction = d / scale;
    let q = (point - a) / scale;
    let t = (q.dot(direction) / direction.dot(direction)).clamp(0.0, 1.0);
    let p = a * (1.0 - t) + b * t;
    p.is_finite().then_some(p)
}

fn screen_nearest_segment(view: SnapView, cursor: SnapScreenPoint, a: Vec3, b: Vec3) -> Option<Vec3> {
    let pa = view.project(a)?;
    let pb = view.project(b)?;
    let (dx, dy) = (pb.pixel.x - pa.pixel.x, pb.pixel.y - pa.pixel.y);
    let denom = dx * dx + dy * dy;
    if !(denom > 0.0 && denom.is_finite()) {
        return None;
    }
    let lambda = (((cursor.x - pa.pixel.x) * dx + (cursor.y - pa.pixel.y) * dy) / denom).clamp(0.0, 1.0);
    // Perspective-correct interpolation maps projected segment fraction back to
    // world segment fraction (orthographic w is constant).
    let numerator = lambda * pa.w;
    let divisor = (1.0 - lambda) * pb.w + numerator;
    if !(divisor > 0.0 && divisor.is_finite()) {
        return None;
    }
    let t = numerator / divisor;
    let p = a * (1.0 - t) + b * t;
    p.is_finite().then_some(p)
}

fn sample_curve_point(curve: &NurbsCurve, tr: SnapTransform, fraction: f64) -> Option<Vec3> {
    let [a, b] = curve.domain();
    let t = a * (1.0 - fraction) + b * fraction;
    let p = tr.point(curve.eval(t).ok()?);
    p.is_finite().then_some(p)
}

/// Simple bounded 1D search. Returns a point on the analytic curve, but does
/// not certify it is the global nearest point for high-curvature rational data.
fn evaluated_nearest(mut at: impl FnMut(f64) -> Option<Vec3>, metric: impl Fn(Vec3) -> Option<f64>) -> Option<Vec3> {
    let mut best = None;
    let mut best_index = 0;
    let mut best_value = f64::INFINITY;
    for i in 0..=SEARCH_STEPS {
        let x = i as f64 / SEARCH_STEPS as f64;
        if let Some(p) = at(x)
            && let Some(d) = metric(p).filter(|d| d.is_finite() && *d >= 0.0)
            && d < best_value {
            best_index = i;
            best_value = d;
            best = Some(p);
        }
    }
    best?;
    let mut low = best_index.saturating_sub(1) as f64 / SEARCH_STEPS as f64;
    let mut high = (best_index + 1).min(SEARCH_STEPS) as f64 / SEARCH_STEPS as f64;
    for _ in 0..32 {
        let a = low + (high - low) * 0.3819660112501051;
        let b = high - (high - low) * 0.3819660112501051;
        let fa = at(a).and_then(&metric).unwrap_or(f64::INFINITY);
        let fb = at(b).and_then(&metric).unwrap_or(f64::INFINITY);
        if fa <= fb { high = b; } else { low = a; }
    }
    let p = at((low + high) * 0.5)?;
    if let Some(d) = metric(p).filter(|v| v.is_finite() && *v < best_value) {
        let _ = d;
        Some(p)
    } else {
        best
    }
}

fn offer_shape(out: &mut Collector<'_>, id: &str, shape: &WorldGeometry) {
    let query = out.query;
    let reference = query.inference.reference;
    match shape {
        WorldGeometry::Anchor(point, kind, accuracy) => out.offer(id, *kind, *point, *accuracy),
        WorldGeometry::Segment(a, b, accuracy) => {
            out.offer(id, SnapKind::End, *a, *accuracy);
            out.offer(id, SnapKind::End, *b, *accuracy);
            out.offer(id, SnapKind::Middle, a * 0.5 + b * 0.5, *accuracy);
            if let Some(p) = screen_nearest_segment(query.view, query.cursor_px, *a, *b) {
                out.offer(id, SnapKind::Nearest, p, *accuracy);
            }
            if let Some(r) = reference
                && let Some(p) = segment_closest(*a, *b, r) {
                out.offer(id, SnapKind::Perpendicular, p, *accuracy);
            }
        }
        WorldGeometry::Polyline(points) => {
            if let (Some(a), Some(b)) = (points.first(), points.last()) {
                out.offer(id, SnapKind::End, *a, SnapAccuracy::DisplayApproximate);
                out.offer(id, SnapKind::End, *b, SnapAccuracy::DisplayApproximate);
            }
            let segments = points.windows(2).map(|w| length(w[1] - w[0])).collect::<Vec<_>>();
            let total = segments.iter().sum::<f64>();
            if total.is_finite() && total > 0.0 {
                let mut remaining = total * 0.5;
                for (edge, distance) in points.windows(2).zip(&segments) {
                    if *distance > 0.0 && remaining <= *distance {
                        let t = (remaining / distance).clamp(0.0, 1.0);
                        out.offer(id, SnapKind::Middle, edge[0] * (1.0 - t) + edge[1] * t, SnapAccuracy::DisplayApproximate);
                        break;
                    }
                    remaining -= distance;
                }
            }
            // Search all segments but publish just one closest proposal. A very
            // dense displayed polyline must not allocate thousands of candidates.
            let nearest = points.windows(2)
                .filter_map(|edge| screen_nearest_segment(query.view, query.cursor_px, edge[0], edge[1]))
                .filter_map(|p| Some((p, query.view.project(p)?.pixel.distance(query.cursor_px))))
                .min_by(|a, b| a.1.total_cmp(&b.1)
                    .then_with(|| a.0.x.total_cmp(&b.0.x))
                    .then_with(|| a.0.y.total_cmp(&b.0.y))
                    .then_with(|| a.0.z.total_cmp(&b.0.z)));
            if let Some((p, _)) = nearest {
                out.offer(id, SnapKind::Nearest, p, SnapAccuracy::DisplayApproximate);
            }
        }
        WorldGeometry::Round(circle) => {
            out.offer(id, SnapKind::Centre, circle.center, SnapAccuracy::Analytic);
            if !circle.closed {
                let (a, b) = circle.endpoints();
                out.offer(id, SnapKind::End, a, SnapAccuracy::Analytic);
                out.offer(id, SnapKind::End, b, SnapAccuracy::Analytic);
                out.offer(id, SnapKind::Middle, circle.at(circle.start_angle + circle.sweep * 0.5), SnapAccuracy::Analytic);
            }
            for i in 0..4 {
                let p = circle.at(i as f64 * std::f64::consts::FRAC_PI_2);
                if circle.contains(p) {
                    out.offer(id, SnapKind::Quadrant, p, SnapAccuracy::Analytic);
                }
            }
            if let Some(p) = evaluated_nearest(
                |t| Some(circle.at(circle.start_angle + circle.sweep * t)),
                |p| query.view.project(p).map(|projection| projection.pixel.distance(query.cursor_px)),
            ) {
                out.offer(id, SnapKind::Nearest, p, SnapAccuracy::Evaluated);
            }
            if let Some(r) = reference {
                for p in circle_tangencies(*circle, r) {
                    out.offer(id, SnapKind::Tangent, p, SnapAccuracy::Analytic);
                }
                let signed_height = (r - circle.center).dot(circle.normal);
                let projected = r - circle.normal * signed_height;
                if let Some(p) = evaluated_nearest(
                    |t| Some(circle.at(circle.start_angle + circle.sweep * t)),
                    |p| Some(length(projected - p)),
                ) {
                    out.offer(id, SnapKind::Perpendicular, p, SnapAccuracy::Evaluated);
                }
            }
        }
        WorldGeometry::Nurbs(curve, tr) => {
            if let Some(a) = sample_curve_point(curve, *tr, 0.0) {
                out.offer(id, SnapKind::End, a, SnapAccuracy::Evaluated);
            }
            if let Some(b) = sample_curve_point(curve, *tr, 1.0) {
                out.offer(id, SnapKind::End, b, SnapAccuracy::Evaluated);
            }
            if let Some(p) = sample_curve_point(curve, *tr, 0.5) {
                out.offer(id, SnapKind::Middle, p, SnapAccuracy::Evaluated);
            }
            if let Some(p) = evaluated_nearest(
                |t| sample_curve_point(curve, *tr, t),
                |p| query.view.project(p).map(|s| s.pixel.distance(query.cursor_px)),
            ) {
                out.offer(id, SnapKind::Nearest, p, SnapAccuracy::Evaluated);
            }
            if let Some(r) = reference
                && let Some(p) = evaluated_nearest(
                    |t| sample_curve_point(curve, *tr, t),
                    |p| Some(length(p - r)),
                ) {
                out.offer(id, SnapKind::Perpendicular, p, SnapAccuracy::Evaluated);
            }
        }
    }
}

fn circle_tangencies(circle: Round, reference: Vec3) -> Vec<Vec3> {
    let q = reference - circle.center;
    let height = q.dot(circle.normal);
    let q = q - circle.normal * height;
    let distance = length(q);
    let scale = distance.max(circle.radius);
    if !(distance > circle.radius && distance.is_finite())
        || height.abs() > scale * 1e-9 {
        return Vec::new();
    }
    let direction = q / distance;
    let cos = circle.radius / distance;
    let sin = (1.0 - cos * cos).max(0.0).sqrt();
    let cross = circle.normal.cross(direction);
    [direction * cos + cross * sin, direction * cos - cross * sin]
        .into_iter().map(|u| circle.center + u * circle.radius)
        .filter(|p| p.is_finite() && circle.contains(*p)).collect()
}

fn offer_inference(out: &mut Collector<'_>) {
    let inf = &out.query.inference;
    let (Some(cursor), Some(plane)) = (inf.cursor_world, inf.plane) else { return };
    let Some((cx, cy)) = plane.coords(cursor) else { return };
    if let Some(p) = plane.position(cx, cy) {
        out.offer("@plane", SnapKind::Projected, p, SnapAccuracy::Analytic);
    }
    if let Some(base) = inf.reference
        && let Some((bx, by)) = plane.coords(base) {
        if let Some(p) = plane.position(cx, by) {
            out.offer("@horizontal", SnapKind::Horizontal, p, SnapAccuracy::Analytic);
        }
        if let Some(p) = plane.position(bx, cy) {
            out.offer("@vertical", SnapKind::Vertical, p, SnapAccuracy::Analytic);
        }
        if let Some(step) = inf.angle_step_deg {
            let dx = cx - bx;
            let dy = cy - by;
            let radius = dx.hypot(dy);
            if radius > 0.0 && radius.is_finite() {
                let turn = step.to_radians();
                let theta = (dy.atan2(dx) / turn).round() * turn;
                if let Some(p) = plane.position(bx + radius * theta.cos(), by + radius * theta.sin()) {
                    out.offer("@angle", SnapKind::Angle, p, SnapAccuracy::Analytic);
                }
            }
        }
    }
    for (index, p) in inf.acquired.iter().enumerate() {
        if let Some((bx, by)) = plane.coords(*p) {
            if let Some(h) = plane.position(cx, by) {
                out.offer(&format!("@track:{index}:h"), SnapKind::Tracking, h, SnapAccuracy::Analytic);
            }
            if let Some(v) = plane.position(bx, cy) {
                out.offer(&format!("@track:{index}:v"), SnapKind::Tracking, v, SnapAccuracy::Analytic);
            }
        }
    }
}

/// The following exact intersections require geometrically coplanar inputs.
/// Skew 3D segments are not a visual intersection. Overlapping / coincident
/// infinite solution families are intentionally omitted.
fn intersections(a: &WorldGeometry, b: &WorldGeometry) -> Vec<(Vec3, SnapAccuracy)> {
    let mut result = Vec::new();
    match (a, b) {
        (WorldGeometry::Segment(a0, a1, aa), WorldGeometry::Segment(b0, b1, ab)) => {
            if let Some(p) = segment_intersection(*a0, *a1, *b0, *b1) {
                result.push((p, if *aa == SnapAccuracy::DisplayApproximate || *ab == SnapAccuracy::DisplayApproximate {
                    SnapAccuracy::DisplayApproximate
                } else { SnapAccuracy::Analytic }));
            }
        }
        (WorldGeometry::Segment(a0, a1, _), WorldGeometry::Round(c))
        | (WorldGeometry::Round(c), WorldGeometry::Segment(a0, a1, _)) => {
            result.extend(line_circle_intersections(*a0, *a1, *c).into_iter().map(|p| (p, SnapAccuracy::Analytic)));
        }
        (WorldGeometry::Round(a), WorldGeometry::Round(b)) => {
            result.extend(circle_intersections(*a, *b).into_iter().map(|p| (p, SnapAccuracy::Analytic)));
        }
        _ => {}
    }
    result
}

fn segment_intersection(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> Option<Vec3> {
    let (u, v, w) = (b - a, d - c, c - a);
    let normal = u.cross(v);
    let denom = normal.dot(normal);
    let scale = length(u) * length(v);
    if !(scale > 0.0 && scale.is_finite() && denom.is_finite() && denom > (scale * 1e-12).powi(2)) {
        return None;
    }
    let s = w.cross(v).dot(normal) / denom;
    let t = w.cross(u).dot(normal) / denom;
    if !(-1e-12..=1.0 + 1e-12).contains(&s) || !(-1e-12..=1.0 + 1e-12).contains(&t) {
        return None;
    }
    let p = a * (1.0 - s.clamp(0.0, 1.0)) + b * s.clamp(0.0, 1.0);
    let q = c * (1.0 - t.clamp(0.0, 1.0)) + d * t.clamp(0.0, 1.0);
    (p.is_finite() && q.is_finite() && length(p - q) <= scale.sqrt().max(length(w)) * 1e-10)
        .then_some((p + q) * 0.5)
}

fn line_circle_intersections(a: Vec3, b: Vec3, circle: Round) -> Vec<Vec3> {
    let d = b - a;
    let dlength = length(d);
    if !(dlength > 0.0 && dlength.is_finite()) {
        return Vec::new();
    }
    let dir = d / dlength;
    let q = a - circle.center;
    let height = q.dot(circle.normal);
    if height.abs() > circle.radius.max(dlength).max(length(q)) * 1e-10
        || dir.dot(circle.normal).abs() > 1e-10 {
        return Vec::new();
    }
    let along = -q.dot(dir);
    let transverse = q + dir * along;
    let distance = length(transverse);
    let discr = (circle.radius - distance) * (circle.radius + distance);
    if !(discr >= 0.0 && discr.is_finite()) {
        return Vec::new();
    }
    let root = discr.sqrt();
    let mut result = Vec::new();
    for t in [along - root, along + root] {
        if (0.0..=dlength).contains(&t) {
            let p = a + dir * t;
            if p.is_finite() && circle.contains(p) && !result.contains(&p) {
                result.push(p);
            }
        }
    }
    result
}

fn circle_intersections(a: Round, b: Round) -> Vec<Vec3> {
    let cos = a.normal.dot(b.normal).abs();
    let delta = b.center - a.center;
    let scale = a.radius.max(b.radius).max(length(delta));
    if cos < 1.0 - 1e-10 || delta.dot(a.normal).abs() > scale * 1e-10 {
        return Vec::new();
    }
    let d = length(delta);
    if !(d > 0.0 && d.is_finite()) || d > a.radius + b.radius || d < (a.radius - b.radius).abs() {
        return Vec::new();
    }
    let along = (d * 0.5) + ((a.radius - b.radius) * (a.radius + b.radius)) / (2.0 * d);
    let h2 = (a.radius - along) * (a.radius + along);
    if !h2.is_finite() || h2 < -1e-10 * scale * scale {
        return Vec::new();
    }
    let axis = delta / d;
    let perp = a.normal.cross(axis);
    let foot = a.center + axis * along;
    let h = h2.max(0.0).sqrt();
    let mut result = Vec::new();
    for p in [foot + perp * h, foot - perp * h] {
        if p.is_finite() && a.contains(p) && b.contains(p) && !result.contains(&p) {
            result.push(p);
        }
    }
    result
}
