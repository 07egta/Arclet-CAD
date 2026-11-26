# Arclet technical drawings: derived geometry (Mission 16)

This implements **runtime, backend-neutral drafting geometry** from the shared
SolidCraft crates. The Arclet UI does **not yet** have a drawing editor, printing
pipeline or newly persisted drawing-view records. Do not claim completed ISO
or ASME compliance or describe the existing viewport section clip as a
manufacturing section drawing.

## Supported today

- Orthographic principal views (front/top/right) and user-defined orthographic
  view bases via existing `NamedView` / `basis_looking`. First- and third-angle
  triplets can be fitted to A4/A3/other supported sheets with one ISO 5455
  **preset scale**, independent of rendering DPI.
- Orthographic B-rep edge and silhouette projection using the **existing** HLR
  engine in `crates/geom/src/hlr`. The returned sheet-mm geometry keeps
  `visible`, `hidden`, `uncertain`, underlying source identity, and curve
  approximation accuracy. Options can include or suppress hidden lines;
  unsupported faces are returned separately, not claimed accurate.
- Scaled section geometry (including wires, material region references, sample
  accuracy and contacts) from **a separately validated section producer**,
  when the producer's section plane faces the drawing view. This is not itself
  a new exact B-rep/mesh cutting algorithm or hatch generator.
- Explicit perspective illustration with focal length and near-plane
  validation. Perspective is **not** a dimensionable/scale-bearing view.
- Detail magnification by constructing a new view at the selected model-space
  centre; model measurements do not change.
- Aligned linear dimensions, radial/diameter dimensions of verified face-on
  circles, and angular dimensions. Outputs include sheet-mm line and arc
  primitives, triangular arrowheads, measured values, and label anchors.
  Text placement and rendering are left to the user interface. No collision
  avoidance, GD&T tolerancing or standard-compliant layout is claimed.
- Unit formatting for all length units accepted in drawing records. Stored
  `FollowModel` requires the caller's actual model unit; `Fixed` overrides it.
- `Drawing::three_view_frames`, `Drawing::render_orthographic_view` and
  `Drawing::dimension_style_for` link runtime views to an existing
  drawing sheet and source identifier.

## API sketch

```rust
use solidcraft_doc::drawing::runtime::LinkedView;
use solidcraft_geom::drawing_view::technical::{
    LineDisplay, ProjectionConvention,
};
use solidcraft_geom::drawing_view::dimensions::{
    DimensionStyle, LengthUnit, linear_dimension,
};
use solidcraft_geom::hlr::Options;

// drawing: the validated drawing document; sheet_id / source_id refer to
// existing entries in drawing.sheets / drawing.sources.
// bounds: model-space (mm) bounding box from the latest evaluated model.
// definitions: Vec<Prepared> from hlr::prepare; placements: Vec<Placement>.
// These are provided by the model evaluator after loading source_id.
let views = drawing.three_view_frames(
    sheet_id, bounds, ProjectionConvention::ThirdAngle, 12.0
)?;
let request = LinkedView {
    sheet_id,
    source_id,
    frame: views.front,
    display: LineDisplay::VisibleAndHidden,
};
let lines = drawing.render_orthographic_view(
    &request, &definitions, &placements, &Options::default(), &|| false
)?;
let style = drawing.dimension_style_for(
    LengthUnit::Millimetre, DimensionStyle::default()
)?;
let annotation = linear_dimension(
    &views.front, first_model_point, second_model_point, 12.0, style
)?;
```

After an edit, **evaluate the model again**, re-prepare changed B-reps, and call
`render_orthographic_view` again with the latest definitions/placements.
The API does not cache geometry or subscribe to model-change events; the
caller owns the correctness of the model ↔ source ID association.

The original `Drawing` JSON schema is unchanged; generated lines, view frames
and dimensions are **not saved** in a drawing file. Persisting these requires
a separate change in
`crates/doc/src/drawing/format/pending/` with the documented migration
procedure. This PR intentionally does not touch the format.

## Validation and limits

Tests added to existing area test files cover:

- A 100 × 50 × 20 mm box on A3 in the two projection conventions, equal
  scale for the three views, aligned positions, bounds contained in the border,
  and deterministic generation.
- Perspective depth falloff vs depth-independent orthographic views.
- A 1:5 → 4× detail magnification, and input rejection.
- Section wire/curve/contact projection and scaled declared error bounds.
- Linear, angular, radius and diameter dimensions: correct model values at
  1:2 and correct converted display units.
- B-rep cube HLR line visibility, geometry regenerated after changing model
  width, deterministic curves, and two distinct assembly placements.
- Nonexistent drawing source/sheet IDs and derived geometry not modifying JSON.

Run after installing Rust:

```bash
cargo fmt --check
cargo test -p solidcraft-geom drawing_view::tests
cargo test -p solidcraft-geom hlr::tests::technical_
cargo test -p solidcraft-doc drawing::tests
cargo xtask ci
```

**Not verified here:** a local Rust toolchain was not available. CI must
compile and run the tests before merging. Existing global CI failures should
be distinguished from newly introduced failures.

**Remaining follow-ups:** section producer from arbitrary solids; true
section hatch, robust end-to-end views for curved trimmed NURBS, annotation
layout/text collision detection, datum/tolerance symbols, persistence and
migration of editable views/dimensions, UI workflow, print/export and
associative invalidation/rebuild scheduling. See the HLR engine's declared
`unsupported` / `uncertain` outputs for the exact-shape limits.
