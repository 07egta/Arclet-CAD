# Verification and remaining work

## Executed Linux checkpoint, 2026-11-12

- 45 native geometry/document/command regressions and 7 camera/worker/UI unit tests passed.
- `cargo clippy --features desktop --all-targets -- -D warnings` passed. OCCT 7.6 system headers still emit upstream C++ deprecation warnings; this is not described as a warning-free C++ build.
- `cargo fmt --all -- --check` passed.
- 20 actual X11-window checks passed using XTest input, not internal command injection.
- Exact B-rep geometry was constructed, displayed, saved, restored and exported. Screenshots show actual running application pixels.
- The initial GitHub CI checkpoint passed 41/42 tests. Its failure was an over-strict test comparing serialized OCCT cache flags byte-for-byte. The revised regression checks metadata, exact measurements, topology counts and geometric samples, including a rational quarter circle. Byte-identical native internal flags and identical triangle ordering are not required.

## Actual-window checks

Startup with real OCCT meshes; background deselection; four non-blank colored viewports; ray picking; orbit; pan; zoom; exact Move; Undo; failed fillet rollback; worker recovery; quoted-path save; failed save preservation; saved-snapshot dirty-state recovery; STEP export; binary STL record validation; unsaved Open guard and Escape; project reload; OS-close dirty guard; normal clean exit.

The test writes `ui-results.json`, `application.log`, screenshots, actual project files and actual STEP/STL exports. An opt-in read-only state file makes assertions deterministic. It does not replace keyboard/mouse input.

## Defects found and fixed during execution

1. Four-view OpenGL callbacks initially cleared other viewports because egui's painter clip covered the whole panel. The scissor now intersects the current viewport with the painter clip, and pixel checks cover all four views.
2. Very fast Ctrl-key events could be missed by testing end-of-frame modifiers. Shortcuts now consume individual input events with their recorded modifiers.
3. An MSAA configuration failed on software GL. The default uses zero MSAA and a depth buffer, and the real window launches under Xvfb.
4. A rejected display mesh could linger in a cache and defeat the total display-memory budget. Hidden/rejected meshes are removed; the document retains exact geometry.
5. Temporary-file cleanup is installed only after exclusive reservation succeeds; it cannot delete someone else's pre-existing temp file.
6. Final file flushing opens the temporary file with write access for Windows FlushFileBuffers compatibility. Windows behavior is tested by the dedicated CI job rather than inferred from Linux.
7. GL uploads use a compile-time Pod bound instead of an unconstrained generic byte cast.

## Not verified / not implemented

No full Serpentine3D migration, Rhino benchmark win, general production reliability, large customer corpus, Intel GPU stress run, macOS launch, browser target, 3DM interchange, Grasshopper plugins, Class-A surface certification or independent GPT Luna review is claimed.
Windows CI compilation/native tests and packaging are recorded separately. Linux screenshots are not presented as Windows screenshots. Successful compilation does not prove interactive Windows rendering.
A native kernel operation can still take a long time, exhaust memory or crash; the worker keeps the UI responsive but is not process isolation or hard cancellation.

## Architectural boundaries

`kernel.rs` owns non-Send/non-Sync OCCT handles behind a small exception-catching C ABI. `model.rs` commits candidate snapshots transactionally. `command.rs` is shared by UI and terminal. The worker owns the Session and native handles; the UI receives ordinary numbers and immutable mesh Arcs. The renderer never creates CAD geometry. Display triangulation is deliberately separate from exact saved/exported B-reps.

Project files store exact geometry and metadata, not an editable parametric operation graph or cross-session Undo. Meshes are disposable display data. Failed tessellation reports a warning and retains the geometry rather than silently substituting a different shape.
