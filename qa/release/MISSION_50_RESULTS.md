# Mission 50 — Arclet professional release readiness

Baseline main: `67455fd84633f03cf188f63617806ab132d5bd83` (2026-11-02). Branch: `test/arclet-professional-release-readiness`.

## Confirmed baseline limitations and predecessor evidence

- Existing `.github/workflows/build.yml` Windows job builds **meltcad and solidcraft**, not Arclet. Its Web job builds the SolidCraft web app, which is out of scope. Windows success there is *not* evidence of an Arclet distributable.
- Existing Melt 3D release workflow packages Melt3D, **not Arclet**. Do not change it for this work.
- Main workflow run [41402057078](https://github.com/07egta/Arclet-CAD/actions/runs/41402057078) on `2ea11ed3` built Windows/Web but failed the Tests job and skipped publication. In its captured logs geometry/document test suites reported 483 and 322 passes; UI tests included failures in help text, translations, font tokens and icons. The runner received a shutdown signal before the UI run finished. These failures **pre-date this PR** and must not be attributed to this packaging change.
- `TEST_RESULTS.md` reports historical Windows `cargo test -p arclet`: 35 application tests + 3 matrix tests passed, plus clippy/fmt checks. This is historical evidence for a different checkout, **not a result for this branch**.
- `docs/arclet-validation.md` documents historical Linux software-renderer GUI screenshots; it does not verify Windows or macOS native presentation.
- Existing PR #1 owns CI regression fixes, #8 saving/recovery, #5 Boolean, #7 performance and #32 the 100-workflow suite. No overlapping application/geometry changes are included here.

## Implementation: isolated Arclet release verification

- Dedicated manual/PR-only GitHub Actions workflow: `.github/workflows/arclet-release-readiness.yml`. Builds **arclet** (not Melt3D) for Windows x64, macOS Apple Silicon, macOS Intel; it does **not** tag, publish or merge anything.
- Native `arclet` test and strict Clippy jobs on Windows x64 and macOS ARM64. Intel macOS is **cross-compiled only**, not tested on actual Intel hardware.
- Packaging verifier `qa/release/candidate.py` checks a real PE x64 or little-endian Mach-O ARM64/x64 architecture, mandatory license/notice files, bundle identity and format, a versioned provenance manifest, ZIP traversal/symlink/content boundaries, byte-for-byte executable SHA-256, and an independent archive SHA-256 file.
- macOS binary gets an **ad-hoc local code signature**, if the macOS runner successfully executes codesign. This is explicitly **not** Developer ID signing or Apple notarization. Actual Apple Silicon and Intel Mac interactive launch tests are **not claimed**.
- `qa/release/test_candidate.py`: deterministic synthetic PE/Mach-O header test fixtures for verifier success, wrong architecture, wrong commit, checksum tampering and ZIP path traversal. These synthetic bytes are **not** actual executables and prove only the packaging verifier logic.
- Candidates upload as short-lived Actions artifacts (7 days). No stable download URL/release created. New code imports only Python standard library; no new Rust/C++ dependency or copyleft code.

## Execution status on this branch

| Check | Result |
| --- | --- |
| Source inspection, build job scope, previous failing CI log | Completed |
| GitHub branch and dedicated CI/verifier implementation | Committed |
| New Python verifier unit tests | **NOT RUN locally** (GitHub PR CI must be checked) |
| Windows Arclet build and tests | **NOT VERIFIED** |
| macOS ARM64 Arclet build and tests | **NOT VERIFIED** |
| macOS Intel cross build | **NOT VERIFIED** |
| cargo fmt, cargo clippy, cargo xtask ci (current branch) | **NOT RUN** |
| Interactive Windows/macOS native GUI, GPU startup, four-view/snap/gizmo | **NOT VERIFIED** |
| Production model editing, saving, restart/re-edit | **NOT VERIFIED on this branch** |
| Startup, render, memory, Boolean and fillet benchmarks | **NOT MEASURED on this branch** |
| Install on clean machine, DLL dependencies, Gatekeeper, notarization | **NOT VERIFIED** |
| Distributable ZIP and SHA256 | Code added; **no actual archive yet produced in this session** |

Runtime limitation: local container has Git but no Rust/Cargo and cannot resolve github.com. Authenticated GitHub connector was used to inspect and commit. DO NOT infer that tests pass from source inspection.

## Manual professional acceptance scenarios before shipping

Record command, application commit, hardware/OS/GPU, exact inputs, output artifact SHA-256, expected/actual geometry measurements and video/screenshots for each.

1. Mechanical plate with through hole: Box → sketch circle → cut → fillet → undo/redo → native save/reopen → STEP export/re-import → compare bounding box, volume and topology.
2. Injection-molded case: outer rectangle + fillet → shell → bosses and screw holes → parameter change → save/reopen → STL and 3MF manifold tests.
3. Bottle: profiles → loft → match/continuity → zebra/curvature → edit a section and rebuild → STEP/native save and reload.
4. Chair: surface loft/sweep/patch → edit CVs → stitch and curvature diagnostics → save/reopen.
5. Lighting: surface blend/revolve with multiple trimmed patches → evaluate G0/G1/G2 joins numerically → save/reopen.
6. Assembly: two components, occurrences/transform and material/layer assignments → edit shared definition, test isolation, save/reopen and STEP assembly export.
7. UI: first frame in both themes; 960x600 and 1920x1080; command line, selection, snap, gizmo, property panel and 4-view behaviour. Do not claim four genuinely independent viewports until validated.
8. Performance: 10-run distributions of cold/warm startup, frame time, drag response, Boolean/fillet/loft rebuilds, save/reopen large models, peak resident memory and idle CPU. Record p50/p95, exact data and hardware; no guessed speedups.

## Release blockers / recommended priority

P0: No recorded successful final-commit build and artifact verification; incomplete interactive save/re-edit acceptance; old failing UI tests need root-cause validation. Verify preservation and reload on malformed/hostile files without data loss.

P1: Independent viewport, snap, gizmo and native GPU start still require on-device verification. macOS bundle/notarization and unsigned Windows distribution need product/release decisions.

P2: Performance and UI visual regression benchmarks, material/layer metadata, CAD third-party interoperability breadth.

**Release decision:** NOT READY / draft. This PR produces a verification harness, not a certified CAD release. `Changes a default: no`; no existing application behavior changed. No merge or auto-merge requested.
