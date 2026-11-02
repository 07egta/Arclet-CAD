# Mission 25 — Arclet CAD sheet-metal numerical engineering

## Scope and reused implementation

This change builds on `crates/doc/src/sheet.rs`, the existing sheet-metal model (original SolidCraft Rust code). It **does not** introduce a second blank representation or change the document schema, B-rep kernel, Boolean implementation, 2D drawing subsystem or existing DXF writer.

Already implemented prior to this PR:

- Rule-controlled thickness, K-factor, bend radius, relief widths/depths, flange gap and hem gap (including expressions).
- Base/edge/contour/path flanges, chained/multiple bends, up/down direction, hem and conventional bend/corner relief variants.
- Bending, unfolding, refolding and flat-cut mapping, including holes and notches.
- Flat pattern, bend-line metadata, DXF with OUTLINE/CUT/BEND_UP/BEND_DOWN layers, optional bend labels and machine-oriented curve approximations.
- Conversion of supported existing bodies to sheet metal (not arbitrary general solids).

The full geometry implementation and existing comprehensive command/test suites remain in place.

## Mathematical reference and units

`thickness` (`T`) and inner bend `radius` (`R`) are millimetres, the K-factor `K` is dimensionless, and the bend turn `θ` is stored in radians.

```text
neutral-axis radius:          R_neutral = R + K*T
bend allowance:               BA = abs(theta) * R_neutral
outer-sharp setback:          S  = (R + T) * tan(abs(theta)/2)
inner-sharp setback:             R * tan(abs(theta)/2)
90-degree outer-sharp blank:  (base depth - S) + BA + (flange height - S)
```

`theta=90°` therefore has `S = R+T`, and `BA = (π/2)*(R+K*T)`.

Numerical oracle from the command-level test (unrelieved 100×60 mm base, 20 mm flange, 90° bend):

| Inputs | BA (mm, analytical) | Expected blank depth (mm) |
|---|---:|---:|
| T 2, R 1, K 0.4 | 2.827433388 | 76.827433388 |
| T 2, R 3, K 0.5 | 6.283185307 | 76.283185307 |

The 90° oracle in `sheet.rs` additionally sweeps:
`(T,R,K) = (0.8,0.8,0.33), (2,1,0.5), (3.2,5,0.44)`, each with upward and downward bends. It compares BA, leg length, flat dimensions, bend line direction/length, fold/flat B-rep validity and flat blank volume against independent closed-form expectations. It does **not** assert that formed volume equals the neutral-axis-based flat volume for every K: that is not true for an arbitrary K-factor.

## Fixes in this PR

1. **Rule dimension validation.** The rule evaluator now rejects invalid/non-finite K, and negative, non-finite or extremely large bend/relief/gap lengths before geometry evaluation. Valid zero-gap and zero-relief rules are explicitly preserved.
2. **Per-flange input validation.** Explicit edge-flange bend radii, sheet thickness/K and corner-relief sizes are checked before trimming any existing panel, with error text naming the quantity.
3. **Dependent-rule deletion guard.** `sheet.manage_rules {name, delete:true}` refuses deletion when a sheet base, contour or convert feature names that rule or implicitly uses it as the current effective rule. Otherwise an existing feature could silently rebuild under a different thickness/radius and change its DXF. Both active and rolled-back features count. Unused rules can still be removed. To change a rule intentionally, edit its parameters (or update the affected sheet's rule selection first).

These checks are additive for valid inputs and do not change the serialization layout or existing DXF output code.

## Regression coverage

**New Rust tests:**

- `arclet_ninety_degree_blank_matches_neutral_axis_across_thickness_radius_and_direction`: six analytical fold/flat cases with B-rep validation.
- `arclet_invalid_radius_and_corner_relief_never_mutate_an_existing_sheet`: invalid per-flange geometry inputs rejected without geometry changes.
- `arclet_rule_length_fields_are_bounded_without_rejecting_zero_relief`: invalid rule dimensions and legacy zero allowances.
- `arclet_sheet_rule_deletion_preserves_existing_flat_pattern_and_dxf`: active and explicit rule references, unused rule removal, unchanged DXF on rejected operation.
- `arclet_k_factor_and_radius_recompute_flats_and_dxf_after_save_reload`: formula, parametric rule edit, DXF difference, reload, unfold and refold.
- `arclet_invalid_sheet_rule_and_custom_bend_radius_leave_design_unchanged`: command-level invalid-input protection and unchanged feature JSON/flat.

**Pre-existing cases still necessary for full acceptance:** L bracket, tray/U-channel, contour flanges, multi-flange boxes, reliefs at corners, round holes and slots, notches, converted fillets/chamfers, exact DXF arcs/ellipses, sketch attachments surviving fold/unfold, edit/undo and parameter changes. The extensive existing tests are in `crates/doc/src/sheet.rs`, `crates/engine/src/cmd/sheet_tests.rs`, `sheet_convert_tests.rs`, `sheet_dxf_tests.rs` and `sheet_face_names_tests.rs`. This PR does not claim these broader tests passed without seeing an actual CI run.

**Dedicated CI workflow:** `.github/workflows/arclet-sheet-metal-engineering.yml` runs the new tests, three existing geometry/DXF reference tests, and document/engine Clippy on a PR.

Commands for additional acceptance:

```sh
cargo test --profile ci --locked -p solidcraft-doc --lib
cargo test --profile ci --locked -p solidcraft-engine --lib -- --skip robustness --skip hostile_params
cargo clippy --profile ci --locked -p solidcraft-doc -p solidcraft-engine --lib -- -D warnings
cargo xtask ci
```

## Manufacturing and release limitations

- A constant K-factor models geometric bend allowance only. **Real springback, anisotropy, die opening, bend deduction tables, tooling clearance, material tolerances and machine-specific compensations are not predicted or certified here.** Use measured coupon bends and machine-specific shop tables before production cutting.
- In the existing implementation a bend turn approaches neither 0 nor 180 degrees exactly; extremely acute returns and self-intersections must be reviewed using actual folded geometry and the shop's bend sequence.
- Flat DXF describes the geometric blank and bend lines under existing functionality; it is **not a certified fabrication drawing**, tolerance stack-up, bend sequence simulator or guarantee that a particular CAM system interprets all entity classes correctly.
- Tests validate mathematical geometry and Rust-side workflow behavior. They do not replace visual inspection of the real desktop application or physical press-brake trial bends.
- The authoring environment has no Rust/Cargo available, and direct GitHub cloning fails DNS lookup. Therefore changes were reviewed as source and CI added but **local compilation, tests, model rendering and the complete `cargo xtask ci` were not performed**. Keep the PR Draft until CI results and desktop acceptance are reviewed.

**Compatibility:** old valid sheet rules, saved design fields, existing DXF entity generation, B-rep and unrelated UI paths are unchanged. Only explicit invalid states and unsafe rule deletion now fail early.
