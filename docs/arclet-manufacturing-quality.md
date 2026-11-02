# Arclet CAD — Mesh manufacturing preflight (Mission 29)

This page describes **implemented, testable code**, not a claim that a 3D printer will successfully
print any particular part. The preflight is read-only and is independent of printer firmware.

## Commands

`mesh.check {"body":"Mesh", "tolerance":"0 mm"}` checks an **imported triangle mesh**.
Its result contains:

- `closed`: requires no invalid triangle/coordinate, a two-manifold edge incidence,
  correct opposite edge winding and a finite **positive signed volume**.
- `topologically_closed`: counts every edge twice but **does not** mean correctly oriented.
- `consistently_oriented`, `outward_oriented`, `winding_conflicts`: independently report
  the facet orientation and the signed volume.
- `invalid_index_triangles`, `nonfinite_vertices`, `zero_area_triangles`,
  `collapsed_after_weld`, `duplicate_vertices`, `duplicate_faces`: malformed or
  duplicate geometry counters.
- `naked_edges`, `non_manifold_edges`, `non_manifold_vertices`, `boundary_loops`: edge and vertex-fan diagnostics. A pair of otherwise closed shells touching at one vertex is rejected, even if all edges are paired.
  `boundary_loops` is **the count of connected naked-edge components**; a branching
  boundary is not necessarily a simple geometric loop.
- `signed_volume_mm3`, `surface_area_mm2`: computed from valid oriented triangles.
  The signed volume is null when orientation/topology is not verified.

`mesh.manufacturing_check {"body":"Cube","bed_size_mm":[180,180,180],
"nozzle_diameter_mm":0.4,"overhang_deg":45}` accepts an imported mesh **or a B-rep**.
It uses the original mesh for diagnostics and a welded candidate mesh using the STL/3MF
exporter's weld-radius convention, including T-junction splitting. A B-rep is tessellated
before welding; the chord target is stated in `chord_target_mm`. Its **default**
chord and meshing path match the standard STL/3MF exporter; overriding `chord_mm`
changes this analysis only. The exporter may additionally quantize coordinates to
f32 (STL and untagged 3MF), so this remains a candidate rather than a byte-for-byte
file validation. Set `weld_tolerance_mm` and `chord_mm` explicitly
for a repeatable analysis.

The report separates:

- `source_topology`: original mesh; malformed triangles/duplicate faces are never quietly
  discarded and represented as clean.
- `topology`: integrity of the welded export candidate.
- `potential_self_intersections`: triangle pairs that touch/cross after excluding
  shared-vertex neighbours. A sweep along X prunes pairs. Above 8,000 triangles or
  500,000 candidates, `complete: false` explicitly prevents a false all-clear.
  Even if complete, this is **not** an exhaustive numerical intersection certificate.
- `sampled_wall_thickness`: approximate inward-ray screening (up to 96 source faces
  and 20,000 triangles); `minimum_sampled_mm` is **not** the minimum manufacturable wall.
- `orientation_and_support`: downward-facing triangle area outside the lowest plane,
  contact area of horizontal downward facets at that plane, and Z translation needed
  to place that plane at Z=0. No bridging or real support-path simulation is performed.
- `bed_fit`: optional XY-bounding-box fit (translation allowed, rotation not considered)
  against [width, depth, height] in **millimetres**.
- `blockers` / `warnings`: actionable reasons for rejecting or reviewing the candidate.
- `verdict`: `invalid_mesh`, `inspection_incomplete`, `requires_review`, or
  `passed_limited_screening`. No verdict means "guaranteed printable".
  `manufacturing_certified` is always false.

### Units and limitations

Distances, bounding boxes, and source vertices use mm; areas use mm² and volumes mm³.
3MF explicitly includes `unit="millimeter"` and preserves separately named objects,
placement and supported colours. STL does not encode units or materials: users must
select millimetres when loading STL into an external slicer. Binary STL rounds vertices
to f32; extremely fine features or coordinates far from the origin can lose precision.

A 0.4 mm nozzle is an **illustrative screening threshold**, not a minimum wall thickness
or an FDM guarantee. Printability depends on line width, material, filament condition,
layer height, printer calibration, and the slicer's thin-wall algorithms.

The existing `solidcraft-slice` handles slice contours, perimeter offsets, infill,
solid skins and generic G-code with bounds, cancellation and open-contour guards.
It does not model supports or validate actual printer firmware. Its overlapping-solid
even/odd behaviour can cancel geometry, and its sharp-corner wall offsets are limited.
Use a supported external slicer to evaluate final toolpaths and supports. Neither
printer-specific G-code nor physical print success is certified by this PR.

### Regression fixtures in the source tree

- Outward 20×20×20 mm cube: exactly 8,000 mm³, 2,400 mm², correct winding.
- Reversed single triangle and fully reversed cube: invalid winding and outwardness.
- A cube with missing faces: exposed boundary edges and a boundary component.
- Two valid tetrahedra touching at one point: disconnected vertex link; not a manifold solid.
- Duplicate face, nonmanifold edge, out-of-range index, zero-area facet,
  non-finite vertex, over-aggressive weld of a 0.02 mm high box: refuse closure.
- STL binary export/import: 12 faces, mm-sized coordinates and expected volume.
- Two-object 3MF export/import: separate names, dimensions, positions, and RGB.
- Capped cylinder and cylindrical through-hole annular plate: outward winding, zero nonmanifold edges, approximate analytic volume and underside area.
- Crossing nonadjacent triangles: intersection screen returns a potential pair;
  an oversized test explicitly returns incomplete.
- Thin open-top but watertight cylindrical vase: 0.2 mm candidate wall vs
  illustrative 0.4 mm nozzle, not a manufacturability certificate.
- Existing `crates/slice/src/lib.rs` tests: layers, open contours, solid skins,
  infill, G-code, planning caps, and cancellation.

All new test tolerances are expressed in source assertions. The dedicated
`arclet-manufacturing-quality.yml` CI workflow runs targeted engine, IO and slicer
regressions. **Do not report them as passing until the GitHub Actions run succeeds.**

### Scope boundaries

This PR deliberately avoids direct changes to `crates/io/src/threemf.rs` (separate
file-interoperability ownership), `crates/geom/src/mesh.rs` and the 3D printer's
proprietary firmware. It does **not** repair arbitrary self-intersections or
non-manifold geometry, provide exhaustive minimal-thickness certificates, validate
multiple body clearances, or promise machine-specific G-code. Use existing
`inspect.clearance` for inter-part distance queries and inspect its `unchecked`
array before relying on a negative result.

### Preflight resource limits

The read-only command refuses mesh candidates above 180,000 vertices or 60,000
triangles **before** running the export-style weld, because T-junction splitting
may be superlinear for hostile open meshes. That is a safe failure, not evidence
that a larger model is unprintable. Vertex-fan manifold classification also
returns an explicitly incomplete result above 500,000 triangles rather than
pretending to certify the topology. The underlying STL/3MF format limits are
separate and much higher.
