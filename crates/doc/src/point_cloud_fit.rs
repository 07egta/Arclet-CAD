//! Least-squares primitive fits. No automatic topology or NURBS is generated.
 //! Every result includes a geometric residual; small residuals do not certify a shape.

use serde::Serialize;
use solidcraft_geom::Vec3;

use crate::point_cloud_analysis::{CloudSample, FitQuality, covariance, quality, solve3, symmetric_eigen};
use crate::{DocError, Result};

fn bad(message: &str) -> DocError {
    DocError::Invalid(format!("point cloud fitting: {message}"))
}
fn valid_tolerance(t: f64) -> Result<()> {
    if t.is_finite() && (1e-9..=1e6).contains(&t) { Ok(()) } else { Err(bad("tolerance must be 1e-9..1e6 mm")) }
}

/// A fitted 3D circle, including both radial and off-plane residual.
#[derive(Clone, Debug, Serialize)]
pub struct CircleEstimate {
    pub center: Vec3,
    pub normal: Vec3,
    pub radius_mm: f64,
    pub quality: FitQuality,
}
#[derive(Clone, Debug, Serialize)]
pub struct CylinderEstimate {
    pub axis_point: Vec3,
    pub axis_direction: Vec3,
    pub radius_mm: f64,
    pub elongation: f64,
    pub quality: FitQuality,
}
#[derive(Clone, Debug, Serialize)]
pub struct SphereEstimate {
    pub center: Vec3,
    pub radius_mm: f64,
    pub quality: FitQuality,
}

/// Fit a circle from x,y coordinates in orthonormal u,v axes through `origin`.
/// Assumes the given origin is the centroid projected to the plane.
fn section(points: &[Vec3], origin: Vec3, u: Vec3, v: Vec3) -> Result<(Vec3, f64)> {
    let mut xx = 0.0;
    let mut xy = 0.0;
    let mut yy = 0.0;
    let mut bx = 0.0;
    let mut by = 0.0;
    let mut mean_r2 = 0.0;
    for point in points {
        let d = *point - origin;
        let x = d.dot(u);
        let y = d.dot(v);
        let r2 = x * x + y * y;
        xx += x * x;
        xy += x * y;
        yy += y * y;
        bx += x * r2;
        by += y * r2;
        mean_r2 += r2;
    }
    let det = xx * yy - xy * xy;
    if !(det.is_finite() && det > (xx + yy).powi(2) * 1e-12) {
        return Err(bad("the fitted circular section has insufficient angular coverage"));
    }
    let cx = (bx * yy - by * xy) / (2.0 * det);
    let cy = (by * xx - bx * xy) / (2.0 * det);
    let center = origin + u * cx + v * cy;
    let radius_sq = mean_r2 / points.len() as f64 + cx * cx + cy * cy;
    let radius = radius_sq.sqrt();
    if !radius.is_finite() || radius <= 1e-9 {
        return Err(bad("the fitted radius is invalid"));
    }
    Ok((center, radius))
}

impl CloudSample {
    pub fn fit_circle(&self, tolerance_mm: f64) -> Result<CircleEstimate> {
        valid_tolerance(tolerance_mm)?;
        if self.offsets.len() < 6 { return Err(bad("circle fitting requires at least six points")); }
        let (mean, cov) = covariance(&self.offsets);
        let (e, vectors) = symmetric_eigen(cov);
        if e[1] <= e[2].max(1.0) * 1e-12 {
            return Err(bad("the circle's plane is degenerate"));
        }
        let normal = vectors[0];
        let (center, radius) = section(&self.offsets, mean, vectors[1], vectors[2])?;
        let residuals: Vec<f64> = self.offsets.iter().map(|p| {
            let d = *p - center;
            let planar = d - normal * d.dot(normal);
            ((planar.len() - radius).powi(2) + d.dot(normal).powi(2)).sqrt()
        }).collect();
        Ok(CircleEstimate {
            center: self.frame + center, normal, radius_mm: radius,
            quality: quality(&residuals, tolerance_mm, self.source_count),
        })
    }

    /// Fit a sphere from a linearized least-squares system. Reports, rather than hides,
    /// all radial residuals including scan noise and non-spherical portions.
    pub fn fit_sphere(&self, tolerance_mm: f64) -> Result<SphereEstimate> {
        valid_tolerance(tolerance_mm)?;
        if self.offsets.len() < 4 { return Err(bad("sphere fitting requires four points")); }
        let (mean, cov) = covariance(&self.offsets);
        let mut rhs = [0.0; 3];
        let mut r2_mean = 0.0;
        for point in &self.offsets {
            let d = *point - mean;
            let r2 = d.len2();
            r2_mean += r2;
            rhs[0] += d.x * r2;
            rhs[1] += d.y * r2;
            rhs[2] += d.z * r2;
        }
        let count = self.offsets.len() as f64;
        let delta = solve3(cov, rhs.map(|x| x / (2.0 * count))).ok_or_else(|| bad("sphere estimation is underdetermined"))?;
        let center = mean + Vec3::from(delta);
        let radius = (r2_mean / count + delta.iter().map(|x| x * x).sum::<f64>()).sqrt();
        if !radius.is_finite() || radius <= 1e-9 { return Err(bad("sphere radius is invalid")); }
        let residuals: Vec<f64> = self.offsets.iter().map(|p| ((p - center).len() - radius).abs()).collect();
        Ok(SphereEstimate {
            center: self.frame + center, radius_mm: radius,
            quality: quality(&residuals, tolerance_mm, self.source_count),
        })
    }

    /// Fit the axis of an elongated cylindrical scan by PCA, then a circular section.
    /// A short cylinder, mixed scene or near-isotropic point distribution is deliberately
    /// rejected; the axis is not generally identifiable from PCA in those cases.
    pub fn fit_cylinder(&self, tolerance_mm: f64) -> Result<CylinderEstimate> {
        valid_tolerance(tolerance_mm)?;
        if self.offsets.len() < 8 { return Err(bad("cylinder fitting requires eight points")); }
        let (mean, cov) = covariance(&self.offsets);
        let (eigen, vectors) = symmetric_eigen(cov);
        if !(eigen[2] > 1.05 * eigen[1].max(0.0) && eigen[1] > 1e-10) {
            return Err(bad("there is no distinguishable elongated cylinder axis"));
        }
        let axis = vectors[2];
        let (center, radius) = section(&self.offsets, mean, vectors[0], vectors[1])?;
        let residuals: Vec<f64> = self.offsets.iter().map(|p| {
            let d = *p - center;
            ((d - axis * d.dot(axis)).len() - radius).abs()
        }).collect();
        Ok(CylinderEstimate {
            axis_point: self.frame + center,
            axis_direction: axis,
            radius_mm: radius,
            elongation: eigen[2] / eigen[1].max(1e-30),
            quality: quality(&residuals, tolerance_mm, self.source_count),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point_cloud::PointCloud;
    use crate::point_cloud::{ColorBy, Colors, Points};

    fn sample(raw: Vec<[f64; 3]>, origin: Vec3, limit: usize) -> CloudSample {
        let values: Vec<f32> = raw.into_iter().flat_map(|p| p.map(|x| x as f32)).collect();
        let cloud = PointCloud {
            id: 1, name: "Known".into(), file: String::new(),
            origin, points: Points::new(values.into()), colors: Colors::default(),
            source_points: 0, units: "mm".into(), transform: crate::IDENTITY,
            point_size: 2.0, color_by: ColorBy::File, color: None,
            height_range: None, visible: true,
        };
        CloudSample::new(&cloud, limit).unwrap()
    }

    #[test]
    fn exact_circle_and_sphere_have_small_residuals_at_remote_origin() {
        let circle: Vec<[f64; 3]> = (0..180).map(|i| {
            let t = std::f64::consts::TAU * i as f64 / 180.0;
            [12.0 + 7.0 * t.cos(), -5.0 + 7.0 * t.sin(), 3.5]
        }).collect();
        let sample = sample(circle, Vec3::new(9e8, -9e8, 1e8), MAX_ANALYSIS_POINTS);
        let fit = sample.fit_circle(1e-3).unwrap();
        assert!((fit.radius_mm - 7.0).abs() < 1e-5, "{fit:?}");
        assert!(fit.quality.rms_mm < 1e-5, "{fit:?}");
        assert!(fit.normal.z.abs() > 0.9999);
        let sphere: Vec<[f64; 3]> = (0..16).flat_map(|i| (0..24).map(move |j| {
            let theta = std::f64::consts::PI * (i as f64 + 0.5) / 16.0;
            let phi = std::f64::consts::TAU * j as f64 / 24.0;
            [
                8.0 + 12.0 * theta.sin() * phi.cos(),
                -3.0 + 12.0 * theta.sin() * phi.sin(),
                5.0 + 12.0 * theta.cos(),
            ]
        })).collect();
        let fit = sample(sphere, Vec3::new(9e8, 0.0, 0.0), MAX_ANALYSIS_POINTS).fit_sphere(1e-3).unwrap();
        assert!((fit.radius_mm - 12.0).abs() < 1e-4, "{fit:?}");
        assert!(fit.quality.rms_mm < 1e-4, "{fit:?}");
    }

    #[test]
    fn known_cylinder_and_noisy_cylinder_report_numerical_error() {
        let clean: Vec<[f64; 3]> = (0..25).flat_map(|z| (0..36).map(move |i| {
            let phi = std::f64::consts::TAU * i as f64 / 36.0;
            [4.0 + 7.0 * phi.cos(), -6.0 + 7.0 * phi.sin(), z as f64 * 5.0]
        })).collect();
        let fit = sample(clean, Vec3::ZERO, MAX_ANALYSIS_POINTS).fit_cylinder(1e-3).unwrap();
        assert!((fit.radius_mm - 7.0).abs() < 1e-4, "{fit:?}");
        assert!(fit.axis_direction.z.abs() > 0.999);
        assert!(fit.quality.rms_mm < 1e-4, "{fit:?}");
        let noisy: Vec<[f64; 3]> = (0..25).flat_map(|z| (0..36).map(move |i| {
            let phi = std::f64::consts::TAU * i as f64 / 36.0;
            let radius = 7.0 + 0.02 * ((i * 7 + z * 3) as f64).sin();
            [4.0 + radius * phi.cos(), -6.0 + radius * phi.sin(), z as f64 * 5.0]
        })).collect();
        let fit = sample(noisy, Vec3::ZERO, MAX_ANALYSIS_POINTS).fit_cylinder(0.1).unwrap();
        assert!((fit.radius_mm - 7.0).abs() < 0.01, "{fit:?}");
        assert!(fit.quality.rms_mm < 0.04, "{fit:?}");
    }

    #[test]
    fn collinear_samples_reject_underdetermined_shapes() {
        let raw: Vec<[f64; 3]> = (0..30).map(|i| [i as f64, 0.0, 0.0]).collect();
        let sample = sample(raw, Vec3::ZERO, MAX_ANALYSIS_POINTS);
        assert!(sample.fit_circle(0.1).is_err());
        assert!(sample.fit_sphere(0.1).is_err());
        assert!(sample.fit_cylinder(0.1).is_err());
    }
}
