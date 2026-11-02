# Mission 06 test results — 2026-11-02

## Verification status

**Code committed; compile and runtime tests NOT RUN in this execution environment.**
The available local terminal does not include `rustc` or `cargo`, and
`git ls-remote` cannot resolve github.com; GitHub contents and PR writes were
performed through the authenticated GitHub connector instead.

| Area | Regression test | Execution |
| --- | --- | --- |
| STL binary exact count, malformed indices, NaN, length mismatch | `stl_binary_header_matches_written_facets_and_verified_reader_rejects_damage` | NOT RUN |
| ASCII STL malformed vertex fields | `stl_rejects_malformed_ascii_coordinates_without_shifting_columns` | NOT RUN |
| STEP AP242 + IGES box exchange; `.sc` save/reopen/re-export | `step_and_iges_exchange_survive_native_save_and_second_export` | NOT RUN |
| STL/OBJ/3MF box exchange; `.sc` save/reopen/re-export | `stl_obj_3mf_exchange_survives_native_save_and_second_export` | NOT RUN |
| Original OBJ fixture geometry, bounds and triangle count | `corpus_mm_box_preserves_name_bounds_and_topology` | NOT RUN |
| OBJ oversized polygon protection | `overlarge_obj_faces_fail_before_unbounded_index_allocation` | NOT RUN |
| OBJ loss warnings | `obj_warns_when_exchange_loses_materials_uvs_normals_or_vertex_colours` | NOT RUN |
| Rhino 3DM fixture → `.sc` → STEP | `rhino_import_persists_as_a_measured_brep_and_reexports_step` | NOT RUN |
| Full Rust workspace CI and existing tests | `cargo xtask ci` | NOT RUN |
| Dependency-license check | `cargo xtask licences --check` | NOT RUN |

The source repository already contains previous smoke tests, for instance
3DM and FCStd import tests and an AP242/IGES geometric box round-trip. Their
historical existence is not evidence that this branch's new changes pass.

## Required reviewer/CI actions

1. Run `cargo test --locked -p solidcraft-io --lib -- --nocapture`.
2. Run `cargo xtask ci` and `cargo xtask licences --check` in a configured
   Rust 1.95+ environment.
3. Inspect the logs for each expected geometry assertion, fix any regressions
   without weakening exactness checks, and record evidence in this document.
4. Expand fixtures beyond a cube for trimmed NURBS, non-mm STEP/IGES files,
   assemblies, colours, open shells and adversarial archives before any broad
   interchange-compatibility claims.

**Shipping status:** draft PR, no claims of tested Rhino interoperability,
certified 3DM export, AP203/AP214 completeness or plugin execution support.
