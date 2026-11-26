//! Double-precision camera and picking math; the renderer uses a local floating origin.
use glam::{DMat4, DVec3, DVec4};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewKind {
    Perspective,
    Top,
    Front,
    Right,
}
impl ViewKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Perspective => "Perspective",
            Self::Top => "Top",
            Self::Front => "Front",
            Self::Right => "Right",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Camera {
    pub kind: ViewKind,
    pub target: DVec3,
    pub span: f64,
    yaw: f64,
    pitch: f64,
}
impl Camera {
    pub fn new(kind: ViewKind) -> Self {
        Self {
            kind,
            target: DVec3::ZERO,
            span: 70.0,
            yaw: -0.9,
            pitch: 0.65,
        }
    }
    pub fn fit(&mut self, bounds: [f64; 6], aspect: f64) {
        if !bounds.iter().all(|x| x.is_finite()) || !aspect.is_finite() || aspect <= 0.0 {
            return;
        }
        let lo = DVec3::new(bounds[0], bounds[1], bounds[2]);
        let hi = DVec3::new(bounds[3], bounds[4], bounds[5]);
        if hi.cmplt(lo).any() {
            return;
        }
        self.target = lo + (hi - lo) * 0.5;
        self.span = ((hi - lo).length() * 0.65 / aspect.min(1.0)).clamp(0.001, 1e10);
    }
    fn eye_direction(&self) -> DVec3 {
        match self.kind {
            ViewKind::Top => DVec3::Z,
            ViewKind::Front => -DVec3::Y,
            ViewKind::Right => DVec3::X,
            ViewKind::Perspective => DVec3::new(
                self.yaw.cos() * self.pitch.cos(),
                self.yaw.sin() * self.pitch.cos(),
                self.pitch.sin(),
            ),
        }
    }
    pub fn eye(&self) -> DVec3 {
        self.target + self.eye_direction() * self.span * 3.0
    }
    fn world_up(&self) -> DVec3 {
        if self.kind == ViewKind::Top {
            DVec3::Y
        } else {
            DVec3::Z
        }
    }
    pub fn basis(&self) -> (DVec3, DVec3) {
        let forward = -self.eye_direction();
        let right = forward.cross(self.world_up()).normalize();
        (right, right.cross(forward).normalize())
    }
    pub fn orbit(&mut self, dx: f64, dy: f64) {
        if self.kind != ViewKind::Perspective || !dx.is_finite() || !dy.is_finite() {
            return;
        }
        self.yaw = (self.yaw - dx * 0.008).rem_euclid(std::f64::consts::TAU);
        self.pitch = (self.pitch + dy * 0.008).clamp(-1.52, 1.52);
    }
    pub fn pan(&mut self, dx: f64, dy: f64, height: f64) {
        if !dx.is_finite() || !dy.is_finite() || !height.is_finite() || height <= 0.0 {
            return;
        }
        let (right, up) = self.basis();
        self.target += (-right * dx + up * dy) * (2.0 * self.span / height);
    }
    pub fn zoom(&mut self, amount: f64) {
        if amount.is_finite() {
            self.span = (self.span * (-amount * 0.002).clamp(-2.0, 2.0).exp()).clamp(0.0001, 1e10);
        }
    }
    pub fn matrix(&self, aspect: f64, origin: DVec3) -> DMat4 {
        let aspect = if aspect.is_finite() {
            aspect.clamp(0.01, 100.0)
        } else {
            1.0
        };
        let near = (self.span * 0.0001).max(1e-8);
        let far = (self.span * 100.0).max(1.0);
        let proj = match self.kind {
            ViewKind::Perspective => {
                DMat4::perspective_rh_gl(2.0 * (1.0_f64 / 3.0).atan(), aspect, near, far)
            }
            _ => DMat4::orthographic_rh_gl(
                -self.span * aspect,
                self.span * aspect,
                -self.span,
                self.span,
                near,
                far,
            ),
        };
        proj * DMat4::look_at_rh(self.eye() - origin, self.target - origin, self.world_up())
    }
    pub fn project(&self, point: DVec3, aspect: f64) -> Option<DVec3> {
        let p = self.matrix(aspect, self.target) * (point - self.target).extend(1.0);
        if !p.is_finite() || p.w <= 1e-15 {
            return None;
        }
        let ndc = p.truncate() / p.w;
        if !(-1.0..=1.0).contains(&ndc.z) {
            return None;
        }
        Some(ndc)
    }
    /// NDC y points up. The returned ray is in document coordinates (millimeters).
    pub fn ray(&self, x: f64, y: f64, aspect: f64) -> Option<(DVec3, DVec3)> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let inv = self.matrix(aspect, self.target).inverse();
        let a = inv * DVec4::new(x, y, -1.0, 1.0);
        let b = inv * DVec4::new(x, y, 1.0, 1.0);
        if !a.is_finite() || !b.is_finite() || a.w.abs() < 1e-15 || b.w.abs() < 1e-15 {
            return None;
        }
        let a = a.truncate() / a.w;
        let b = b.truncate() / b.w;
        let direction = (b - a).try_normalize()?;
        Some((a + self.target, direction))
    }
    /// Point on the view's construction plane through the world origin.
    pub fn construction_point(&self, x: f64, y: f64, aspect: f64) -> Option<DVec3> {
        let (o, d) = self.ray(x, y, aspect)?;
        let n = match self.kind {
            ViewKind::Front => DVec3::Y,
            ViewKind::Right => DVec3::X,
            _ => DVec3::Z,
        };
        let den = d.dot(n);
        if den.abs() < 1e-10 {
            return None;
        }
        let t = -o.dot(n) / den;
        if t < 0.0 || !t.is_finite() {
            return None;
        }
        let p = o + d * t;
        if p.abs().max_element() > 1e8 {
            None
        } else {
            Some(p)
        }
    }
}
pub fn ray_box(origin: DVec3, direction: DVec3, bounds: [f64; 6]) -> bool {
    let mut near = 0.0_f64;
    let mut far = f64::INFINITY;
    for i in 0..3 {
        if direction[i].abs() < 1e-15 {
            if origin[i] < bounds[i] || origin[i] > bounds[i + 3] {
                return false;
            }
        } else {
            let a = (bounds[i] - origin[i]) / direction[i];
            let b = (bounds[i + 3] - origin[i]) / direction[i];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if far < near {
                return false;
            }
        }
    }
    far >= near
}
pub fn ray_triangle(origin: DVec3, direction: DVec3, a: DVec3, b: DVec3, c: DVec3) -> Option<f64> {
    let ab = b - a;
    let ac = c - a;
    let h = direction.cross(ac);
    let det = ab.dot(h);
    let tolerance = ab.length() * ac.length() * 1e-12;
    if det.abs() <= tolerance {
        return None;
    }
    let inverse = 1.0 / det;
    let s = origin - a;
    let u = inverse * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(ab);
    let v = inverse * direction.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = inverse * ac.dot(q);
    if distance.is_finite() && distance >= 0.0 {
        Some(distance)
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn center_ray_hits_target_in_every_view() {
        for kind in [
            ViewKind::Perspective,
            ViewKind::Top,
            ViewKind::Front,
            ViewKind::Right,
        ] {
            let mut c = Camera::new(kind);
            c.fit([1e8, 1e8, 0.0, 1e8 + 3.0, 1e8 + 5.0, 6.0], 1.8);
            let (o, d) = c.ray(0.0, 0.0, 1.8).unwrap();
            assert!((c.target - o).cross(d).length() < 1e-6);
            let p = c.project(c.target, 1.8).unwrap();
            assert!(p.x.abs() < 1e-9 && p.y.abs() < 1e-9);
        }
    }
    #[test]
    fn fit_includes_all_corners_for_tall_and_wide_views() {
        for aspect in [0.25, 1.0, 4.0] {
            for kind in [
                ViewKind::Perspective,
                ViewKind::Top,
                ViewKind::Front,
                ViewKind::Right,
            ] {
                let mut c = Camera::new(kind);
                c.fit([-10.0, -20.0, -5.0, 10.0, 20.0, 5.0], aspect);
                for x in [-10.0, 10.0] {
                    for y in [-20.0, 20.0] {
                        for z in [-5.0, 5.0] {
                            let p = c.project(DVec3::new(x, y, z), aspect).unwrap();
                            assert!(p.x.abs() < 1.0 && p.y.abs() < 1.0);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn triangle_picking_is_double_sided_and_rejects_behind_eye() {
        let a = DVec3::new(-1.0, -1.0, 0.0);
        let b = DVec3::new(1.0, -1.0, 0.0);
        let c = DVec3::new(0.0, 1.0, 0.0);
        assert_eq!(ray_triangle(DVec3::Z, -DVec3::Z, a, b, c), Some(1.0));
        assert_eq!(ray_triangle(DVec3::Z, -DVec3::Z, c, b, a), Some(1.0));
        assert!(ray_triangle(DVec3::Z, DVec3::Z, a, b, c).is_none());
        assert!(ray_triangle(DVec3::Z, -DVec3::Z, a, a, a).is_none());
    }
    #[test]
    fn parallel_box_rays_do_not_divide_by_zero() {
        let b = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        assert!(ray_box(DVec3::new(0.0, 0.0, 5.0), -DVec3::Z, b));
        assert!(!ray_box(DVec3::new(4.0, 0.0, 5.0), -DVec3::Z, b));
        assert!(!ray_box(DVec3::new(0.0, 0.0, 5.0), DVec3::Z, b));
    }
    #[test]
    fn top_construction_plane_and_zoom_remain_finite() {
        let mut c = Camera::new(ViewKind::Top);
        assert!(c.construction_point(0.0, 0.0, 1.0).unwrap().length() < 1e-9);
        for _ in 0..1000 {
            c.zoom(1e9);
            c.pan(1.0, 1.0, 600.0);
        }
        assert!(c.matrix(1.0, DVec3::ZERO).is_finite());
        assert!(c.span >= 0.0001);
    }
}
