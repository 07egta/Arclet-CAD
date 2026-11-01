# Solid skins and per-layer infill

## Decision

Extend the existing Rust slicer without adding a dependency. A scanline is a sorted set of
occupied intervals, so intersection and subtraction can identify exposed roofs and undersides
without importing another CAD or slicer implementation. The same code builds for native and
wasm targets. This is a geometry implementation from elementary line-intersection maths.

`PrintConfig` now has:

| Setting | Meaning | Default |
|---|---|---|
| `top_solid_layers` | Solid layers below each exposed top surface | 3 |
| `bottom_solid_layers` | Solid layers above each exposed bottom surface | 3 |
| `layer_infill` | `LayerInfill { layer, density }` overrides; layer is zero-based and density is a fraction in `0..=1` | Empty |
| `infill_spacing` | Existing sparse-infill line spacing for layers without an override | Unchanged, 3 mm |

**Changes a default:** top and bottom skins are enabled. Set both skin-layer counts to zero
to retain the previous open-infill behavior. A density of zero disables only sparse infill;
it does not remove a requested skin. An override of one uses the extrusion line width as its
spacing. Duplicate layer indices, nonexistent layers, non-finite values and out-of-range
densities are errors.

## What the planner does

For each layer, material present in every requested neighboring cross-section remains the
sparse core. The rest of the current layer receives solid skin paths. This detects shelves,
partial roofs and local undersides within the model, as well as its first and last layers.
Holes remain holes under the existing even-odd contour rule. Skin and sparse paths are clipped
to complementary intervals, including at 100% density, so the same region is not filled twice.
Skin direction alternates by layer using the existing diagonal infill directions.

The final fractional layer is retained. Its cutting plane is the midpoint of its actual
thickness, and extrusion uses that thickness rather than the nominal layer height. A model
thinner than one nominal layer therefore has one layer instead of being discarded.
A final remainder smaller than the writer's 0.001 mm Z precision is merged into the previous
layer, preserving the exact mesh height in the sliced geometry. Emitted Z positions are rounded
to that precision, and extrusion uses those same rounded height increments. Already-sliced
input whose rounded heights repeat is rejected instead of extruding twice at the same height.

`slice_until(triangles, height, stop)` and `gcode_until(layers, config, stop)` accept a cancellation
callback. The original `slice` and `gcode` entry points remain available. Cancellation is checked
inside geometry batches and scanline planning. Cancellation and limit errors never return a
partial G-code string.

Planning is bounded: at most 64 solid layers in either direction, 2,000,000 stored contour
vertices, 20,000 scanlines per fill pass, 200,000,000 geometry/scanline work units per operation,
1,000,000 path-budget units, and 128 MiB of G-code. The path budget charges both planned fill
segments and emitted extrusion segments. Exceeding a limit returns an error; it does not quietly
drop remaining fill. Branching slice contours are rejected before adjacency searches can grow
quadratically, and open contours are refused before G-code is generated. Coordinates and
printer settings must be finite. Both the centered model and the final extrusion paths, including
offset-induced excursions and bead width, are checked against the configured bed.

## Verification and limits

Regression tests cover the number of solid end layers, partial roofs at depth, local undersides,
through-holes, complementary solid fill, per-layer overrides, fractional last layers, malformed
input, open contours, planning bounds, and cancellation during work.

**Bambu A1 / A1 mini 実機: 未確認 (not tested on hardware).** Start/end G-code remains the existing
generic placeholder. This change does not add Bambu profiles, an AMS filament manifest, or a
`.gcode.3mf` package. It does not make the output a verified Bambu print file.

Supports, bridge-direction planning, seam placement and wipe/coasting remain unimplemented.
A detected underside receives skin paths but may still need physical support. Overlapping
solids still follow the existing even-odd rule, and the existing miter offset remains unsuitable
for some sharp or narrow contours. Skin planning does not repair those geometric limits.
