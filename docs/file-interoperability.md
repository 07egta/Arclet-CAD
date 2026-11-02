# Arclet CAD — file interchange compatibility matrix

**Scope:** Mission 06, `crates/io`, 2026-11-02. This describes source inspection and the tests added on the feature branch, **not third-party product certification**. No native Rhino, FreeCAD, or commercial CAD code was copied. The STEP/IGES kernel is explicitly outside the scope of this PR while `CONTRIBUTING.md` pauses kernel changes.

## Capabilities and tested promises

| Format | Read path | Write path | What can be retained | Known loss / unsupported | Evidence status |
| --- | --- | --- | --- | --- | --- |
| Native `.sc` / JSON | Document container / JSON deserializer | Document container / JSON serializer | Document features, embedded imports, parameters and mesh data | Not a foreign CAD exchange format; depends on serialized representation | Existing unit tests; new import→container→reload regressions **added, not run** |
| STEP `.stp`, `.step` | `step_import_feature` → kernel STEP reader | `step_export_bodies` → AP242 | Supported B-rep solids, part names/placements, colour, normalized millimetres | No complete AP203/AP214/AP242 entity coverage guarantee; materials, PMI, layers and every exotic surface are not proven preserved | New AP242 export→import→native-save→reload→export test **added, not run** |
| STEP AP203 | Generic STEP reader; no AP203-only matrix measured | **Not offered** (writer is AP242) | Only entities supported by the existing parser | AP203 full conformance and round-trip **unverified** | **Not certified** |
| STEP AP214 | Generic STEP reader; cadmpeg uses AP214 as a *conversion intermediate* | **Not offered** (writer is AP242) | Only entities supported by the existing parser | Full AP214 entity, naming, presentation and metadata retention **unverified** | **Not certified** |
| STEP AP242 | Generic STEP reader | AP242 Part 21 writer (`AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF`) | Common solids, units, placement, names, colors | No PMI/semantic tolerance certification or broad external-CAD corpus | Schema and code inspected; regression **not run** |
| IGES `.igs`, `.iges` (5.3) | Kernel IGES-to-STEP interpreter | Kernel IGES 5.3 writer | Supported trimmed curves/surfaces, B-reps, names, colors, units converted to mm | Some entity types, trimmed surfaces, topology and singular cases may be unsupported or approximated; non-mm unit cases not broadly measured | New solid import→save→export test **added, not run** |
| Rhino 3DM `.3dm` | `cadmpeg` Rhino decoder → neutral IR → STEP AP214 → kernel B-rep | **Not implemented** | Some supported solid B-rep geometry and nominal measurements | Unknown across Rhino versions and entity types; unsupported NURBS/trim, lights, annotations, layers/materials, plugin objects and design history are not asserted retained | Existing Rhino 7 10×20×30 mm box fixture smoke test; new save/re-export STEP test **not run** |
| OBJ | In-house polygon reader | In-house OBJ writer (optional MTL as separate `export_obj`) | Mesh triangles, object names, positions, face orientation; body diffuse color only when MTL file is emitted alongside OBJ | No formal unit tag (**mm assumed**), UVs/source normals/materials/per-vertex colors ignored by importer; now **warned**, nongeneric n-gons triangulated as fans | New original mm-box corpus and bounds test **added, not run** |
| Binary / ASCII STL | Permissive legacy reader; new strict binary verifier | Export of triangulated bodies | Mesh coordinates and winding, in mm by convention | No names, layers, colors, source units or materials; geometric accuracy depends on tessellation | New count, invalid-facet and round-trip tests **added, not run** |
| 3MF | 3MF package reader, unit conversion to mm | Standard or face-tagged 3MF writer | Mesh coordinates, names and supported colours; standardized 3MF unit field; optional face tags are Arclet-specific | Native B-rep topology lost on conventional mesh export; extra slicer extensions and material semantics not fully portable | Existing 3MF unit and mesh tests; new save→re-export test **added, not run** |
| glTF 2.0 / GLB | glTF reader (metres → mm, Y-up positioning, supported mesh colours) | **Not implemented** | Meshes, scene/placement under supported transforms, some base-color PBR | Textures, skins, animation and many PBR properties not retained; no external asset retrieval by `mesh_import_feature` | Existing glTF unit tests; export path absent |
| FreeCAD `.FCStd` | `cadmpeg` decoder → STEP | **Not implemented** | Supported solid geometry | Native feature history, constraints and detailed presentation not guaranteed | Existing 10×20×30 mm box fixture smoke test |

**Exactness conditions.** B-rep geometry is not interchangeable with a faceted mesh: mesh-to-STEP export intentionally fails rather than claiming to recover analytic surfaces. STL/OBJ are unitless on disk; source units cannot be inferred accurately from arbitrary files. Neither a supported `.3dm` file import nor Rhino-like command naming enables `RhinoCommon`, `.rhp` or `.gha` plugin execution.

## New hardening on the Mission 06 branch

1. Binary STL's declared triangle count matches facets actually emitted by the low-level writer; the model export path rejects invalid indices, unrepresentable coordinates, or excess facets instead of silently dropping data. `read_stl_verified` rejects truncated/overlong binary files and missing/invalid facets without changing legacy permissive `read_stl`.
2. ASCII STL only accepts complete three-coordinate `vertex` lines and discards malformed facets rather than shifting coordinates around nonnumeric tokens.
3. OBJ rejects inputs over the configured mesh size limit, limits any single polygon to 65,536 vertices, and enforces the total triangle count *before* allocating the face triangulation.
4. OBJ now explicitly warns when it could not retain source materials, UVs, normals or per-vertex colours. The reader still imports the geometry and does not pretend those properties round-trip.
5. Original OBJ fixture, existing Rhino fixture, and programmatic B-rep tests check body counts, 10×20×30 mm dimensional bounds, 6,000 mm³ volume, mesh closure / face count and the save/reload cycle as applicable.

## 3DM export feasibility

A valid 3DM writer would need versioned chunk tables, object and layer records, attributes, colour/material/units, properly serialized exact NURBS/B-rep topology (including trimming, knots and weights), archive encoding and version interoperability. `crates/io/src/rhino3dm.rs` presently lists headers and top-level chunks only. The installed `cadmpeg` integration is **decoder-only** for Rhino. A 3DM exporter is therefore **not** added here: generating a syntactically plausible 3DM without verifiable geometry would violate the exact-exchange requirement.

A future independent, permissively licensed encoder could be explored only after its licence and actual B-rep round-trip capability are reviewed. Do not substitute a mesh disguised as a 3DM B-rep; do not claim Rhino plugin/API compatibility. Existing original `MIT OR Apache-2.0` code and bundled permitted dependencies remain unchanged.

## How to reproduce the checks

```sh
cargo test --locked -p solidcraft-io --lib -- --nocapture
cargo xtask licences --check
cargo xtask ci
```

Tests are added next to existing format modules, per `AGENTS.md`. Current runtime has neither Rust/Cargo nor GitHub connectivity from its local shell, so **no local compilation, cargo test, cargo xtask ci, Windows, web, or external Rhino verification was performed** for these changes. See `tests/file-interoperability/RESULTS.md`. Do not merge this draft until CI has compiled the changes and the stricter round-trip tests have passed or have been corrected based on measured output.
