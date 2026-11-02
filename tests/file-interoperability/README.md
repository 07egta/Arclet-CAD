# Arclet CAD file-interoperability regression corpus

These files and test cases belong to Mission 06. The original OBJ fixture
`fixtures/mm_box.obj` was written for this repository, released as CC0-1.0
for test-data reuse. It does not contain source code, geometry, or screenshots
copied from Rhino or any commercial CAD application.

## Exact fixture

- OBJ object: `Millimetre Block`.
- Unit convention: **millimetres** (OBJ contains no normative unit field).
- Vertices: **8**; original polygon faces: **6** quads.
- After import fan triangulation: **12** triangles.
- Bounds: **[0, 0, 0]** to **[10, 20, 30]** mm.
- Volume of closed oriented box: **6,000 mm³**.
- Topology expectation: closed watertight shell.

Additional regression inputs are generated from the existing pure-Rust box
feature and the existing `crates/io/src/native_cad/fixtures/block.3dm`
(Rhino 7 box created using the MIT-licensed rhino3dm tool already documented
in that fixture's source).

## Reproducibility

Run `cargo test --locked -p solidcraft-io --lib` from repository root. The
tests in `crates/io/src/lib.rs`, `crates/io/src/obj.rs` and
`crates/io/src/native_cad/tests.rs` verify import, native container write,
native reopen, geometry rebuild, re-export, re-import, volumes, dimensions,
triangle/face counts and rejected hostile inputs. A parser-only test does not
certify the original CAD system's export or import compatibility.

See [compatibility matrix](../../docs/file-interoperability.md) and
[results](RESULTS.md).
