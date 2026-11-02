//! Bounded and read-only double-precision point-cloud analysis.
 //! Results are estimates with residuals, not exact B-rep or NURBS reconstructions.

use serde::Serialize;
use solidcraft_geom::Vec3;

use crate::point_cloud::PointCloud;
use crate::{DocError, Result, apply_point, apply_vector};

pub const MAX_ANALYSIS_POINTS: usize = 100_000;
pub const DEFAULT_ANALYSIS_POINTS: usize = 20_000;

fn invalid(message: &str) -> DocError {
    DocError::Invalid(format!("point cloud analysis: {message}"))
}
fn coordinate(p: Vec3, i: usize) -> f64 {
    match i { 0 => p.x, 1 => p.y, _ => p.z }
}
fn check_tolerance(t: f64) -> Result<()> {
    if t.is_finite() && (1e-9..=1e6).contains(&t) {
        Ok(())
    } else {
        Err(invalid("tolerance must be between 1e-9 and 1e6 mm"))
    }
}

/// A bounded sample of the cloud in its **world-aligned local frame**.
/// `offsets` are f64 relative to `frame`, avoiding premature f32 world conversion.
pub struct CloudSample {
    pub frame: Vec3,
    pub offsets: Vec<Vec3>,
    pub source_count: usize,
    pub stride: usize,
}
impl CloudSample {
    pub fn new(cloud: &PointCloud, limit: usize) -> Result<Self> {
        if !(1..=MAX_ANALYSIS_POINTS).contains(&limit) {
            return Err(invalid("sample limit must be 1..100000"));
        }
        cloud.check()?;
        if cloud.is_empty() {
            return Err(invalid("the point cloud is empty"));
        }
        let n = cloud.len();
        let stride = n.div_ceil(limit).max(1);
        let frame = apply_point(&cloud.transform, cloud.origin);
        let mut offsets = Vec::with_capacity(n.div_ceil(stride));
        for p in cloud.points.iter().step_by(stride) {
            let local = Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
            let offset = apply_vector(&cloud.transform, local);
            if !offset.is_finite() {
                return Err(invalid("non-finite transformed point"));
            }
            offsets.push(offset);
        }
        Ok(Self { frame, offsets, source_count: n, stride })
    }

    pub fn centroid(&self) -> Vec3 {
        self.offsets.iter().fold(Vec3::ZERO, |sum, p| sum + *p) / self.offsets.len() as f64
    }

    pub fn nearest_world(&self, world: Vec3) -> Result<NearestPoint> {
        if !world.is_finite() {
            return Err(invalid("the query point must be finite"));
        }
        let query = world - self.frame;
        let index = KdIndex::new(&self.offsets);
        let (sample_index, distance_mm) = index.nearest(query, None).ok_or_else(|| invalid("empty index"))?;
        Ok(NearestPoint {
            sample_index, world_point: self.frame + self.offsets[sample_index],
            distance_mm, sampled: self.offsets.len(), source_points: self.source_count,
        })
    }

    pub fn fit_plane(&self, tolerance_mm: f64) -> Result<PlaneEstimate> {
        check_tolerance(tolerance_mm)?;
        if self.offsets.len() < 3 {
            return Err(invalid("at least three points are needed"));
        }
        let (center, cov) = covariance(&self.offsets);
        let (eigen, directions) = symmetric_eigen(cov);
        if eigen[1] <= 1e-12 * eigen[2].max(1.0) {
            return Err(invalid("coincident or collinear points cannot define a plane"));
        }
        let normal = directions[0];
        let residuals: Vec<f64> = self.offsets.iter().map(|p| (*p - center).dot(normal).abs()).collect();
        Ok(PlaneEstimate {
            point: self.frame + center, normal,
            quality: quality(&residuals, tolerance_mm, self.source_count),
            planarity_ratio: eigen[0].max(0.0) / eigen[1].max(1e-30),
        })
    }

    pub fn outliers(&self) -> Result<OutlierEstimate> {
        if self.offsets.len() < 5 {
            return Err(invalid("outlier detection needs at least five points"));
        }
        let index = KdIndex::new(&self.offsets);
        let mut distances = Vec::with_capacity(self.offsets.len());
        for (i, p) in self.offsets.iter().enumerate() {
            distances.push(index.nearest(*p, Some(i)).ok_or_else(|| invalid("no neighbor"))?.1);
        }
        let med = median(&distances);
        let deviations: Vec<f64> = distances.iter().map(|d| (d - med).abs()).collect();
        let mad = median(&deviations);
        // Regular grids have MAD=0; keep a 25% spacing floor for stability.
        let threshold_mm = med + (mad * 6.0).max(med * 0.25).max(1e-6);
        let outlier_count = distances.iter().filter(|d| **d > threshold_mm).count();
        Ok(OutlierEstimate {
            sampled: self.offsets.len(), source_points: self.source_count,
            median_neighbor_mm: med, median_absolute_deviation_mm: mad,
            threshold_mm, outlier_count,
            outlier_fraction: outlier_count as f64 / self.offsets.len() as f64,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FitQuality {
    pub sampled: usize,
    pub source_points: usize,
    pub rms_mm: f64,
    pub p95_mm: f64,
    pub max_mm: f64,
    pub tolerance_mm: f64,
    pub inlier_fraction: f64,
}
pub(crate) fn quality(residuals: &[f64], tolerance_mm: f64, source_points: usize) -> FitQuality {
    let mut ordered = residuals.to_vec();
    ordered.sort_by(f64::total_cmp);
    let n = ordered.len();
    let rms = (residuals.iter().map(|d| d * d).sum::<f64>() / n as f64).sqrt();
    FitQuality {
        sampled: n, source_points, rms_mm: rms, p95_mm: ordered[(n - 1) * 95 / 100],
        max_mm: ordered[n - 1], tolerance_mm,
        inlier_fraction: residuals.iter().filter(|d| **d <= tolerance_mm).count() as f64 / n as f64,
    }
}
fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let m = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) { (sorted[m - 1] + sorted[m]) / 2.0 } else { sorted[m] }
}

#[derive(Clone, Debug, Serialize)]
pub struct NearestPoint {
    /// Index in the *sample*, not in the original file.
    pub sample_index: usize,
    pub world_point: Vec3,
    pub distance_mm: f64,
    pub sampled: usize,
    pub source_points: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct PlaneEstimate {
    pub point: Vec3,
    pub normal: Vec3,
    pub quality: FitQuality,
    /// PCA smallest/middle variance ratio: smaller implies more planar.
    pub planarity_ratio: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct OutlierEstimate {
    pub sampled: usize,
    pub source_points: usize,
    pub median_neighbor_mm: f64,
    pub median_absolute_deviation_mm: f64,
    pub threshold_mm: f64,
    pub outlier_count: usize,
    pub outlier_fraction: f64,
}

/// Covariance in f64 with the origin subtracted *before* accumulating squares.
pub(crate) fn covariance(points: &[Vec3]) -> (Vec3, [[f64; 3]; 3]) {
    let center = points.iter().fold(Vec3::ZERO, |sum, p| sum + *p) / points.len() as f64;
    let mut c = [[0.0; 3]; 3];
    for p in points {
        let d = *p - center;
        let a = [d.x, d.y, d.z];
        for i in 0..3 {
            for j in 0..3 { c[i][j] += a[i] * a[j]; }
        }
    }
    for row in &mut c {
        for entry in row { *entry /= points.len() as f64; }
    }
    (center, c)
}

/// Symmetric Jacobi eigen solver: ascending eigenvalues and unit eigenvectors.
pub(crate) fn symmetric_eigen(mut a: [[f64; 3]; 3]) -> ([f64; 3], [Vec3; 3]) {
    let mut v = [[0.0; 3]; 3];
    for (i, row) in v.iter_mut().enumerate() { row[i] = 1.0; }
    for _ in 0..40 {
        let mut p = 0;
        let mut q = 1;
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            if a[i][j].abs() > a[p][q].abs() { p = i; q = j; }
        }
        if a[p][q].abs() <= 1e-14 * (a[0][0].abs() + a[1][1].abs() + a[2][2].abs()).max(1.0) { break; }
        let angle = 0.5 * (2.0 * a[p][q]).atan2(a[q][q] - a[p][p]);
        let (sn, cs) = angle.sin_cos();
        for row in &mut v {
            let (vp, vq) = (row[p], row[q]);
            row[p] = cs * vp - sn * vq;
            row[q] = sn * vp + cs * vq;
        }
        let (app, aqq, apq) = (a[p][p], a[q][q], a[p][q]);
        a[p][p] = cs * cs * app - 2.0 * sn * cs * apq + sn * sn * aqq;
        a[q][q] = sn * sn * app + 2.0 * sn * cs * apq + cs * cs * aqq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for i in 0..3 {
            if i == p || i == q { continue; }
            let (ip, iq) = (a[i][p], a[i][q]);
            a[i][p] = cs * ip - sn * iq;
            a[p][i] = a[i][p];
            a[i][q] = sn * ip + cs * iq;
            a[q][i] = a[i][q];
        }
    }
    let mut pairs: [(f64, Vec3); 3] = std::array::from_fn(|i| (a[i][i], Vec3::new(v[0][i], v[1][i], v[2][i])));
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    (pairs.map(|p| p.0), pairs.map(|p| p.1))
}

pub(crate) fn solve3(a: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let mut rows: [[f64; 4]; 3] = std::array::from_fn(|i| [a[i][0], a[i][1], a[i][2], b[i]]);
    let scale = a.iter().flatten().map(|v| v.abs()).fold(0.0, f64::max);
    if !scale.is_finite() || scale == 0.0 { return None; }
    for k in 0..3 {
        let pivot = (k..3).max_by(|i, j| rows[*i][k].abs().total_cmp(&rows[*j][k].abs()))?;
        if rows[pivot][k].abs() <= scale * 1e-10 { return None; }
        rows.swap(k, pivot);
        let d = rows[k][k];
        for col in k..4 { rows[k][col] /= d; }
        for row in 0..3 {
            if row == k { continue; }
            let factor = rows[row][k];
            for col in k..4 { rows[row][col] -= factor * rows[k][col]; }
        }
    }
    let output = [rows[0][3], rows[1][3], rows[2][3]];
    output.iter().all(|v| v.is_finite()).then_some(output)
}

struct KdNode { index: usize, low: Option<usize>, high: Option<usize>, split: usize }
/// Deterministic balanced k-d tree, O(N) additional memory, O(log N) typical nearest lookup.
pub(crate) struct KdIndex<'a> { points: &'a [Vec3], nodes: Vec<KdNode>, root: Option<usize> }
impl<'a> KdIndex<'a> {
    pub(crate) fn new(points: &'a [Vec3]) -> Self {
        let mut ids: Vec<usize> = (0..points.len()).collect();
        let mut nodes = Vec::with_capacity(points.len());
        let root = Self::build(points, &mut ids, &mut nodes, 0);
        Self { points, nodes, root }
    }
    fn build(points: &[Vec3], ids: &mut [usize], nodes: &mut Vec<KdNode>, depth: usize) -> Option<usize> {
        if ids.is_empty() { return None; }
        let split = depth % 3;
        let middle = ids.len() / 2;
        ids.select_nth_unstable_by(middle, |a, b| coordinate(points[*a], split).total_cmp(&coordinate(points[*b], split)).then_with(|| a.cmp(b)));
        let id = nodes.len();
        nodes.push(KdNode { index: ids[middle], low: None, high: None, split });
        let (left, tail) = ids.split_at_mut(middle);
        let low = Self::build(points, left, nodes, depth + 1);
        let high = Self::build(points, &mut tail[1..], nodes, depth + 1);
        nodes[id].low = low;
        nodes[id].high = high;
        Some(id)
    }
    pub(crate) fn nearest(&self, q: Vec3, exclude: Option<usize>) -> Option<(usize, f64)> {
        if !q.is_finite() { return None; }
        let mut best = None;
        self.visit(self.root?, q, exclude, &mut best);
        best.map(|(i, d2)| (i, d2.sqrt()))
    }
    fn visit(&self, node_id: usize, q: Vec3, exclude: Option<usize>, best: &mut Option<(usize, f64)>) {
        let node = &self.nodes[node_id];
        let p = self.points[node.index];
        if Some(node.index) != exclude {
            let d2 = (q - p).len2();
            if best.is_none_or(|(id, d)| d2 < d || (d2 == d && node.index < id)) {
                *best = Some((node.index, d2));
            }
        }
        let delta = coordinate(q, node.split) - coordinate(p, node.split);
        let (near, far) = if delta <= 0.0 { (node.low, node.high) } else { (node.high, node.low) };
        if let Some(child) = near { self.visit(child, q, exclude, best); }
        if let Some(child) = far
            && best.is_none_or(|(_, distance)| delta * delta <= distance)
        {
            self.visit(child, q, exclude, best);
        }
    }
}

/// Translation-only nearest-point ICP: useful for coarse registration after
/// coordinate frames have been aligned, but does not estimate rotation.
#[derive(Clone, Debug, Serialize)]
pub struct TranslationAlignment {
    pub translation_mm: Vec3,
    pub iterations: usize,
    pub quality: FitQuality,
    pub method: &'static str,
}
pub fn align_translation(fixed: &CloudSample, moving: &CloudSample, tolerance_mm: f64) -> Result<TranslationAlignment> {
    check_tolerance(tolerance_mm)?;
    if fixed.offsets.len() < 4 || moving.offsets.len() < 4 {
        return Err(invalid("alignment requires four sampled points per cloud"));
    }
    let index = KdIndex::new(&fixed.offsets);
    let mut delta = fixed.centroid() - moving.centroid();
    let mut iterations = 0;
    for _ in 0..12 {
        let mut pairs = Vec::with_capacity(moving.offsets.len());
        for p in &moving.offsets {
            let query = *p + delta;
            if let Some((target, dist)) = index.nearest(query, None) {
                pairs.push((dist, fixed.offsets[target] - query));
            }
        }
        if pairs.len() < 4 { return Err(invalid("alignment has too few correspondences")); }
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let count = (pairs.len() * 9 / 10).max(4);
        let correction = pairs[..count].iter().fold(Vec3::ZERO, |sum, (_, error)| sum + *error) / count as f64;
        if !correction.is_finite() { return Err(invalid("non-finite alignment correction")); }
        delta += correction;
        iterations += 1;
        if correction.len() < 1e-9 { break; }
    }
    let residuals = moving.offsets.iter().map(|p| {
        index.nearest(*p + delta, None).map(|(_, dist)| dist)
    }).collect::<Option<Vec<_>>>().ok_or_else(|| invalid("alignment failed to find neighbors"))?;
    Ok(TranslationAlignment {
        translation_mm: fixed.frame - moving.frame + delta, iterations,
        quality: quality(&residuals, tolerance_mm, moving.source_count),
        method: "translation_only_trimmed_icp",
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::point_cloud::{ColorBy, Colors, Points};

    fn cloud(points: Vec<[f64; 3]>, origin: Vec3) -> PointCloud {
        let values: Vec<f32> = points.into_iter().flat_map(|p| p.map(|x| x as f32)).collect();
        PointCloud {
            id: 42, name: "Survey".into(), file: String::new(),
            origin, points: Points::new(values.into()), colors: Colors::default(),
            source_points: 0, units: "mm".into(),
            transform: crate::IDENTITY, point_size: 2.0,
            color_by: ColorBy::File, color: None, height_range: None, visible: true,
        }
    }

    #[test]
    fn pca_fits_noisy_and_exact_planes_with_remote_frame() {
        let points: Vec<[f64; 3]> = (0..24).flat_map(|i| (0..24).map(move |j| {
            let x = 0.3 * i as f64;
            let y = 0.5 * j as f64;
            [x, y, 2.0 * x - 0.5 * y + 3.0]
        })).collect();
        let sample = CloudSample::new(&cloud(points, Vec3::new(9e8, -9e8, 1e8)), 10_000).unwrap();
        let fit = sample.fit_plane(0.001).unwrap();
        let expected = Vec3::new(-2.0, 0.5, 1.0).normalized().unwrap();
        assert!(fit.normal.dot(expected).abs() > 0.999999, "{fit:?}");
        assert!(fit.quality.rms_mm < 1e-5, "{fit:?}");
        assert!(fit.quality.inlier_fraction > 0.999);
        let noisy: Vec<[f64; 3]> = (0..24).flat_map(|i| (0..24).map(move |j| {
            [i as f64, j as f64, 0.02 * ((i * 7 + j * 3) as f64).sin()]
        })).collect();
        let sample = CloudSample::new(&cloud(noisy, Vec3::new(9e8, 9e8, 0.0)), 10_000).unwrap();
        let fit = sample.fit_plane(0.06).unwrap();
        assert!(fit.quality.rms_mm < 0.03, "{fit:?}");
        assert!(fit.quality.inlier_fraction > 0.99, "{fit:?}");
    }

    #[test]
    fn kd_nearest_matches_brute_force_and_finds_an_isolated_outlier() {
        let mut points: Vec<[f64; 3]> = (0..13).flat_map(|i| (0..11).map(move |j| {
            [i as f64, j as f64, (i * j) as f64 * 0.01]
        })).collect();
        points.push([500.0, 700.0, 600.0]);
        let sample = CloudSample::new(&cloud(points, Vec3::ZERO), 10_000).unwrap();
        let index = KdIndex::new(&sample.offsets);
        for k in 0..50 {
            let q = Vec3::new(k as f64 * 0.27, k as f64 * 0.11, 0.1);
            let actual = index.nearest(q, None).unwrap();
            let expected = sample.offsets.iter().enumerate().map(|(i, p)| (i, (q - *p).len()))
                .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0))).unwrap();
            assert_eq!(actual.0, expected.0);
            assert!((actual.1 - expected.1).abs() < 1e-10);
        }
        let report = sample.outliers().unwrap();
        assert!(report.outlier_count >= 1, "{report:?}");
        assert!(report.threshold_mm < 10.0, "{report:?}");
        let near = sample.nearest_world(Vec3::new(3.05, 4.05, 0.12)).unwrap();
        assert!(near.distance_mm < 0.3, "{near:?}");
    }

    #[test]
    fn translation_alignment_reports_error_without_mutating_scans() {
        let points: Vec<[f64; 3]> = (0..11).flat_map(|i| (0..9).map(move |j| {
            [i as f64, j as f64, (i * j) as f64 * 0.05]
        })).collect();
        let a = cloud(points.clone(), Vec3::new(9e8, 0.0, 0.0));
        let b = cloud(points, Vec3::new(9e8 + 130.0, -70.0, 25.0));
        let fixed = CloudSample::new(&a, 10_000).unwrap();
        let moving = CloudSample::new(&b, 10_000).unwrap();
        let alignment = align_translation(&fixed, &moving, 1e-5).unwrap();
        assert!((alignment.translation_mm - Vec3::new(-130.0, 70.0, -25.0)).len() < 1e-5, "{alignment:?}");
        assert!(alignment.quality.rms_mm < 1e-6, "{alignment:?}");
        assert_eq!(b.origin, Vec3::new(9e8 + 130.0, -70.0, 25.0));
    }

    #[test]
    fn large_cloud_is_sampled_to_a_strict_upper_bound() {
        let points: Vec<[f64; 3]> = (0..200_000).map(|i| {
            [(i % 1000) as f64, (i / 1000) as f64, 0.0]
        }).collect();
        let scan = cloud(points, Vec3::ZERO);
        let sample = CloudSample::new(&scan, 10_000).unwrap();
        assert_eq!(sample.source_count, 200_000);
        assert_eq!(sample.stride, 20);
        assert_eq!(sample.offsets.len(), 10_000);
        assert!(CloudSample::new(&scan, 0).is_err());
        assert!(CloudSample::new(&scan, MAX_ANALYSIS_POINTS + 1).is_err());
        let mut broken = scan.clone();
        broken.origin.x = f64::NAN;
        assert!(CloudSample::new(&broken, 1000).is_err());
    }

    #[test]
    fn degenerate_planes_and_invalid_queries_fail_safely() {
        let raw = (0..16).map(|i| [i as f64, 0.0, 0.0]).collect();
        let scan = cloud(raw, Vec3::ZERO);
        let sample = CloudSample::new(&scan, 100).unwrap();
        assert!(sample.fit_plane(0.1).is_err());
        assert!(sample.fit_plane(f64::NAN).is_err());
        assert!(sample.nearest_world(Vec3::new(f64::NAN, 0.0, 0.0)).is_err());
    }
}
