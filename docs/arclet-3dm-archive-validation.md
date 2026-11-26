# Arclet 3DM archive structural validation (Mission 49)

## Scope and reader boundary

`crates/io/src/rhino3dm.rs` is an **independent structural lister**. It checks
the Rhino archive signature, version and outer chunk boundaries; it does **not**
read geometric objects, layers or attributes. Actual import still uses
`cadmpeg-registry 0.6.3` (Rhino decoder) -> STEP AP214 -> internal STEP/B-rep
reader. There is **no 3DM exporter**. No new dependency, C++ SDK or proprietary
implementation was introduced in this change.

## Defect fixed

The old structural reader always treated an outer chunk as a 4-byte type +
4-byte length; it skipped the value field entirely for short chunks. Real
Rhino 7 archives use **4-byte type + 8-byte value** and short chunks still
carry their inline value. With a 32-bit reader, archive version 70 would
misplace the next record and produce a misleading error or listing.

The revised reader uses a 4-byte chunk value for archive versions below 50
and 8 bytes for 50 and newer. It bounds-checks the entire header and body,
retains the inline value for short records, and refuses oversized/truncated
records without reading outside the source. This fix does not claim to
validate compressed or nested chunks, checksums or geometry semantics.

## Regression cases

The tests added to the existing `rhino3dm.rs` test module cover:

- Legacy 32-bit ordinary/short/empty records and their offsets.
- Modern 64-bit ordinary and short records (including a value > u32::MAX).
- The **committed, independently generated Rhino 7 3DM fixture**
  `crates/io/src/native_cad/fixtures/block.3dm`: header version 70,
  19,497 bytes, 18 top-level records, first chunk at byte 32 with length
  124, final 0x7fff chunk at byte 19,477 with length 8.
- Truncated modern length fields, impossibly long body lengths and short
  inline values that must not be interpreted as payload lengths.
- Geometry remains explicitly unsupported by the lister.

The archive fixture was inspected byte-for-byte; the structural assertions
encode those observations. **These modified tests are not compiled or run in
this session**: this environment has no `cargo` or `rustc`, and its shell
has no external GitHub network access. Do not mark the patch as verified.

## Format fidelity and outstanding work

| Format | Current path | Retained / known gaps |
| --- | --- | --- |
| 3DM | cadmpeg decode -> neutral IR -> STEP AP214 -> B-rep | Existing 10×20×30 mm box fixture checks 6,000 mm³. Object/layer/material/colour/placement/trim fidelity is not established; no 3DM write. |
| STEP | Own STEP reader -> Import feature, own writer on export | Analytical surfaces and solids supported in the reader; real-world trimmed face, winding, knot, rational weight and assembly round-trips require expanded fixtures. STEP interchange is not a lossless replacement for native design history. |
| IGES | IGES -> STEP -> Import feature; IGES writer | Some IGES types are unsupported. Unreconstructable geometry can warn or fail; not equivalent to preserving the original input. |
| STL/OBJ/3MF | Mesh import/export | Tessellated geometry is not analytic NURBS/B-rep; detailed tests are owned by separate draft PR #3. |

**Losses to expose to users:** 3DM neutral decoding and STEP encoding already
return warning lists via `native_cad_to_step`; STEP import combines these with
kernel warnings. This change does not establish whether the codec reports every
unsupported attribute. A follow-up should compare explicit unit, layer,
identity, colour, transform, NURBS knot/weight and trimming fixtures against
the source before certifying export equivalence.

## Licences and policy

No new third-party code or dependency is added. Existing cadmpeg codecs are
locked to version 0.6.3 by Cargo; see `LICENSE_AUDIT.md` for the scope and
limitations of the **existing** dependency review. openNURBS and
OpenCASCADE remain unintegrated. Nothing in this change relies on copying
commercial CAD code.

The repository's `AGENTS.md` references `skills/solidcraft-file-formats/SKILL.md`,
`skills/solidcraft-contributing/SKILL.md`, and related skills, but those
paths do not exist in this fork; `CONTRIBUTING.md` and `docs/policy.md`
were read instead. `CONTRIBUTING.md` pauses edits to the geometry kernel,
so this patch changes only `crates/io/src/rhino3dm.rs` and documentation.

## Required verification before merge

```sh
cargo test --locked -p solidcraft-io --lib rhino3dm::tests
cargo test --locked -p solidcraft-io --lib native_cad::tests
cargo test --locked -p solidcraft-io --lib
cargo xtask licences --check
cargo xtask ci
```

On Windows and macOS, additionally open the committed real 3DM box, inspect its
6,000 mm³ volume and 10×20×30 mm extent, export STEP, reload it and compare
body count and dimensions. Repeat with multi-layer, rational, trimmed and
multi-body external fixtures. Capture real UI screenshots; none are supplied
by this unexecuted test-only patch.
