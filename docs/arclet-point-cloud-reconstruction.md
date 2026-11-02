# Arclet CAD — Point cloud analysis and reverse-engineering baseline

This change adds **bounded, diagnostic** reconstruction helpers to the existing
PLY / XYZ / PTS point-cloud workflow. The scan remains a point cloud, not a CAD
solid. A plane, circle, sphere or cylinder fit is a numerical estimate, not a
guarantee that the measured surface is a perfect primitive or NURBS surface.

## Supported input and storage

- Existing import accepts PLY (ASCII or binary), XYZ, and PTS; colours are
  stored as per-point RGB bytes when supplied, otherwise the existing fallback
  is used.
- Choose **source units** explicitly on insertion: `mm`, `cm`, `m`, `in`, `ft`.
  Internal coordinates are in mm, with double-precision `origin` and
  single-precision point offsets relative to that origin.
- XYZ/PTS now validates individual UTF-8 lines instead of materializing an
  additional UTF-8 string for the full file. Malformed and overly long text
  rows (over 16 KiB) are skipped and counted in the warnings.
- The importer now refuses positions whose *offset from the first kept
  point* exceeds the document coordinate limit. Large, individually valid
  world positions cannot silently overflow a relative point.
- Existing edit accepts rigid placement matrices for rotation and
  translation. New `scale` is a **uniform** geometric factor applied to
  local point offsets about the cloud's stored origin (not a unit correction,
  not a nonuniform transform). Unit reinterpretation and uniform scaling
  cannot be supplied together; each is undoable. Reject invalid scale or
  scaled coordinates without modifying the original cloud.

## Example commands

Run these through the engine command interface with the given command id and
JSON arguments. The paths and cloud names are illustrative and must refer
to actual files and design objects.

```json
{"id":"pointcloud.insert","params":{"path":"scan.ply","units":"mm","max_points":1000000}}
{"id":"pointcloud.analyze","params":{"cloud":"scan","mode":"plane","tolerance_mm":0.2,"max_points":20000}}
{"id":"pointcloud.analyze","params":{"cloud":"scan","mode":"circle","tolerance_mm":0.2}}
{"id":"pointcloud.analyze","params":{"cloud":"scan","mode":"cylinder","tolerance_mm":0.2}}
{"id":"pointcloud.analyze","params":{"cloud":"scan","mode":"sphere","tolerance_mm":0.2}}
{"id":"pointcloud.analyze","params":{"cloud":"scan","mode":"nearest","point":[0,0,100]}}
{"id":"pointcloud.analyze","params":{"cloud":"scan","mode":"outliers"}}
{"id":"pointcloud.analyze","params":{"cloud":"scan","mode":"cad_distance","body":"Existing Body","tolerance_mm":0.2}}
{"id":"pointcloud.analyze","params":{"cloud":"moving","mode":"align","target":"fixed","tolerance_mm":0.2}}
{"id":"pointcloud.edit","params":{"cloud":"scan","scale":1.02}}
```

`pointcloud.analyze` never modifies the document or the CAD feature history.
An alignment result is a suggested **translation** in world millimetres;
the user must separately apply it (for example by editing a rigid placement).
It does not estimate a rotation or perform a guaranteed 6-DOF ICP alignment.

## Interpretation of numerical results

- `sampled`: how many stored points were actually evaluated.
  `source_points` reports the cloud's stored count before sampling, not
  necessarily every point in the original scanner file (which can be thinned
  during import); `stride` is the deterministic sampling spacing.
- `rms_mm`: root-mean-square **unsigned geometric residual** in millimetres.
  `p95_mm` is its 95th percentile, `max_mm` its maximum.
- `tolerance_mm`: user-supplied absolute distance limit (default 0.1 mm).
  `inlier_fraction` is the fraction of **sampled** points at or below that
  threshold; it is *not* a statistically calibrated confidence probability.
- Plane: centroid and least-squares normal, using the smallest eigenvector
  of the covariance matrix; `planarity_ratio` is the ratio of smallest to
  middle variance. Collinear or coincident scans are rejected.
- Circle: 3D plane fit followed by an algebraic circular-section fit;
  residuals include both radial error and out-of-plane distance.
- Sphere: linear least-squares sphere estimate; a planar, collinear or
  insufficiently diverse sample is rejected as underdetermined.
- Cylinder: PCA principal elongated axis, followed by a circle fit in the
  transverse plane. This is designed for **long and sufficiently sampled**
  cylinders; short, oblique, partial or mixed scan surfaces can fail and
  are not silently treated as certified cylinders.
- Outliers: median of nearest-distinct-*index* neighbor distances, median
  absolute deviation (MAD), threshold and detected outlier fraction; zero
  MAD uses a spacing-dependent floor. No points are deleted.
- `cad_distance`: unsigned nearest distance from sampled scan points to the
  evaluated CAD body's *reference tessellation*, using an indexed triangle
  surface. It is **not** exact analytic NURBS/B-rep surface distance, nor a
  signed inside/outside evaluation. Mesh approximation influences the result.
- `align`: coarse translation-only nearest-neighbour ICP, initialized by
  centroids and trimmed to the closest 90% of correspondences for up to 12
  iterations. It does not solve for rotation, scale, or correspondence identity.

## Performance and memory envelope

| Stage | Present limit | Key memory effect |
|---|---|---|
| Raw point-cloud file | 1 GiB existing input ceiling | The file bytes are still held in memory while parsing; streaming file I/O remains future work |
| Stored scan points | Default 5 million, maximum 20 million per cloud | 12 bytes per point for XYZ offsets; optional 3 bytes per RGB colour |
| Fit/search sample | Default 20,000, maximum 100,000 | Up to ~2.4 MB of f64 offset triples plus O(N) spatial index / residual buffers |
| CAD tessellation distance | Maximum 500,000 triangles | An additional triangle hierarchy is required; creating the reference mesh itself can consume memory |
| GPU point dots | Approximately 750,000 combined visible samples | Bounded converted byte buffer and cached vertices; sampling is by file order rather than camera-aware LOD |
| CPU point dots | 100,000 combined visible samples | Existing CPU fallback cap |
| Text scan row | 16 KiB | Avoid unbounded per-line scanning / full-file string copying |

**No observed runtime or peak-RSS benchmark is claimed.** The container used
for this PR does not have `cargo`/`rustc` installed. Existing GitHub Actions
test checks should supply compile/regression results; the staged performance
benchmark added under `crates/io/src/point_cloud.rs` is optional and ignored by
default.

To measure import throughput (100k, 1m, optional 10m points), run:

```sh
cargo test --locked -p solidcraft-io million_point_cloud_benchmark -- --ignored --nocapture
ARCLET_LARGE_POINT_BENCH=1 cargo test --locked -p solidcraft-io million_point_cloud_benchmark -- --ignored --nocapture
```

The ignored benchmark reports input bytes, retained bytes and elapsed
milliseconds. Use an OS process monitor to measure peak RSS separately.
Time assertions are intentionally avoided on shared CI runners.

Run focused regression checks:

```sh
cargo test --locked -p solidcraft-doc point_cloud
cargo test --locked -p solidcraft-io point_cloud
cargo test --locked -p solidcraft-engine point_cloud
cargo test --locked -p solidcraft-ui-egui point_clouds
cargo test --locked -p solidcraft-engine schema::point_cloud
```

## Numerical precision and current limitations

- An f32 offset near **1,000 mm** has about 0.000061 mm quantization spacing;
  at **1,000,000 mm**, about 0.0625 mm; at **1,000,000,000 mm**, about
  64 mm. The double-precision origin helps with distant coordinate
  *offsets* but does not preserve high precision across enormous scan extents.
  Spatial tiling/chunked f64 origins would be required for that.
- The GPU path still uploads f32 **absolute world** positions. Distant scans
  can visibly lose sub-millimetre detail; a camera-relative GPU transform
  requires work outside this mission's ownership.
- Current sampling is deterministic file-order thinning, **not** voxel-grid
  sampling or adaptive screen-space level of detail. Structured scanners can
  create aliasing; range-select tools and an interactive outlier editor
  remain future work.
- Each analysis reconstructs a bounded sample/index when requested.
  Interactions are not incremental or cancellable; heavy mesh creation may
  still block the command thread until the computation completes.
- ICP initialization can fail without approximate correspondence or when the
  scans differ by rotation; mixed shapes, occlusion, high noise and strong
  outliers can also distort primitive fits. There is no calibrated
  statistical confidence interval.
- Spline/curve network fitting, arbitrary NURBS surface recovery, mesh
  repair and automatic feature-history generation are not implemented.
  Reconstruct these deliberately from the reported primitive parameters
  after checking fit quality.
- Invalid scans, invalid coordinates, insufficient geometry and invalid
  models produce errors; no original point data or CAD geometry is changed
  by analytical commands.

## Regression dataset specification

The tests generate, rather than download, known geometric samples:

- A 24×24 tilted plane near a 900,000,000 mm origin (and a noisy horizontal
  plane): check normalized normal, RMS and inlier fraction.
- 180 points on a 7 mm circle, 384 points on a 12 mm sphere and 900 points
  on a 7 mm cylinder: check radius, axis and RMS against explicit numeric
  tolerances.
- A regular grid with one distant point: verify KD-tree nearest answers
  against exhaustive search and the MAD outlier response.
- 20,000 repeated coordinates: confirm the search can prune zero-distance
  branches instead of exploring every duplicate on each query.
- Matching 99-point scans at two known translations: verify an estimated
  translation and near-zero residual, without modifying the inputs.
- 200,000 input positions sampled down to 10,000 for bounded memory.
- A cloud 0.25 mm above a two-triangle mesh and a real CAD box: verify the
  unsigned distance error. Damaged/NaN/overlong input cases and
  lossless save/reopen tests supplement existing PLY/XYZ/PTS coverage.

All numerical thresholds in tests are **assertion targets**. They must not be
reported as measured passing values until those tests actually run.
