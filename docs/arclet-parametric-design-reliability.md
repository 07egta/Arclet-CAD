# Mission 48 — Parametric history and named-reference reliability

## Existing implementation examined

Arclet already has a parametric feature timeline, saved feature IDs, parameter-driven sketches, history replay, face and edge semantic names, lineage, incremental snapshots, suppression, rollback, Undo/Redo, and edit/redefine commands. These mechanisms are reused, not duplicated.

PR #16 already owns dependency-graph validation and stale-result detection; PR #6 owns opt-in incremental evaluation. PR #8 owns persistence. Mission 48 deliberately does not modify those overlapping modules.

## Confirmed safety defect

`crates/doc/src/hole_reference.rs::find_edge` previously ran `naming::locate_edge` on a saved hole/cylinder reference name, but if it returned an error, the reference was silently replaced with the closest geometric edge within one quarter of the body's size. A deleted edge could therefore make an edited design drill a hole at the wrong location, even though the stored semantic identity no longer existed.

## Implemented correction

- If an edge name is stored, resolve the name strictly through the existing semantic naming API. A deleted, unresolvable name raises `DocError::Invalid` with the reference number and stored name. Do not substitute another edge by proximity.
- Designs with no saved edge name preserve the existing geometric fallback, maintaining legacy document compatibility.
- Both hole and cylinder references use the same existing `reference_frame` path, so no duplicate reference resolver or serialized field is added.
- Previously valid named references still follow their semantic identities when an upstream edit moves geometry.

## Regression cases

The existing `crates/engine/src/cmd/edit_tests.rs` now includes:

- `arclet_named_hole_reference_never_retargets_a_neighbor_after_history_edit`: build a named-reference hole; simulate an upstream-lost edge by invalidating only its saved semantic name; assert the subsequent evaluation reports the lost reference instead of publishing the hole; verify an untouched serialized document remains healthy and has the same measured volume.
- `arclet_named_hole_reference_follows_upstream_resize_and_undo_redo`: a named-reference hole follows a resized base feature; feature IDs are preserved; Undo/Redo restore geometry and the saved JSON; reload preserves the edited volume.

These tests are written but **not yet confirmed green**. The authoring runtime lacks `cargo`/`rustc`, and the GitHub clone host is not resolvable. CI validation and desktop visual checks remain required.

## Follow-up acceptance workflow

The full Sketch → dimension → Extrude → Fillet → Hole → Pattern → edit original dimension → recompute → Undo → Redo → Save → Open → edit again scenario should be executed with real model fixtures and checked for precise body volume, bounds and references before the PR is marked ready. Existing test coverage in `edit_tests.rs`, `patterns_tests.rs`, `lineage_face_tests.rs`, `sketch_point_expr_tests.rs` and `single_hole_tests.rs` can be reused. Do not claim all variants were verified without an actual test run.

```sh
cargo fmt --all -- --check
cargo test --locked -p solidcraft-engine --lib arclet_named_hole_reference
cargo test --locked -p solidcraft-engine --lib
cargo test --locked -p solidcraft-doc --lib
cargo xtask ci
```

## Known limitations

- The stored semantic name can itself be ambiguous after a topological split; this patch stops **missing-name fallback**, but does not prove unambiguous matching of every face or fillet edge.
- Existing fillet edge selection still has compatibility-oriented fallback logic; it is outside this isolated fix and should be separately tested before tightening it.
- Legacy unnamed references can still follow a nearby edge by design.
- On a successful command, the application still requires real UI workflow verification (history browser, errors, progress, Undo/Redo, and saving).
- No guarantee is made for arbitrary multi-body Boolean topology, all pattern references, large histories, or real-world CAD fidelity.

**Changes a default:** Only for named references that have disappeared: now error rather than silently drilling near another edge. **Design format:** unchanged. **Dependencies:** unchanged, no GPL/LGPL/AGPL code. **main:** untouched.
