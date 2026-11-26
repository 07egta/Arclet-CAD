# Mission 42: experimental native precision interaction

## Change status

**Draft: not release-ready.** The implemented branch introduces a four-pane host and
per-pane camera/viewport state. Native compilation, real screenshots, end-to-end
picking and model edits have not yet been verified. `main` remains untouched.

## Opt-in

The existing `ui.set` command can set `{"quadViews":true}` for this session.
The native viewport has a Single view button and Maximize active / Restore
four views controls. Ctrl+Shift+4 toggles the workspace. The four panes are
Top, Front, Right and Perspective. Document geometry is shared.

## Engineering decisions

- Per-pane `ViewportState` means independent pick caches, hover feedback,
  sketch display state and navigation gestures.
- Each pane has its own camera, animation slot and pending fit; activation
  swaps state into the established `SolveApp.cam` and `SolveApp.viewport`
  before calling the existing `navigation::handle` and `viewport::show`.
- Separate GPU renderers are used because egui-wgpu prepares **all**
  callbacks before painting **any** callback. One renderer/uniform buffer
  would leave every pane using the last prepared view. This is intentionally
  a correctness-first experiment, not a memory-efficient final design.
- Picking cache keys now use `proj.cam` (including FOV) rather than the
  global camera; rect origin and dimensions are included.
- Border classification is exclusive (right and bottom take shared edges).
  Mouse-press ownership persists until release after all pane interactions.

## Tests added

`crates/ui-egui/src/viewport_layout_tests.rs` contains checks for
four disjoint panes, exclusive shared boundaries, maximize/restore,
independent camera targets and their restoration, plus camera-ray versus
screen projection for each of the four standard views.

**The presence of tests is not evidence that they passed.** CI results
must be inspected and any compiler or runtime failures repaired before
marking this draft ready.

## Known risks and gaps

1. 4 GPU Renderers can multiply memory usage, pipeline compilation and scene
   uploads. Benchmark large meshes, transparent objects and loss/recovery.
2. Global controls, camera-follow, snapshots, sketches and transient
   interactions were designed for one pane; some could still use the wrong
   active pane or global coordinates. GUI tests must exercise real pointer
   events, Escape, Undo/Redo and drag across dividers.
3. Snapping and Gumball still use the existing partial implementation.
   End/Mid/Center/Intersection/Perpendicular/Tangent/Quadrant/Nearest/
   Vertex/Grid/Ortho/Projected Point are **not** fully delivered here.
   In particular, numeric entry, local/world axis scaling, persistent
   snap chips and full preview/cancel transactions remain separate work.
4. No interactive Windows/macOS screenshot was captured in this session.
5. Pane layouts are session state, not persistent per-document views.
6. Keyboard shortcut and controls currently contain hardcoded English
   strings and require localization before product release.
7. The `ui.set` toggle and per-pane renderer still require UI/CI
   verification. Headless proof alone cannot establish a working desktop UX.

## Coexistence with other PRs

Avoid modifying `apps/arclet/src/statusbar.rs` (PR #23/#1),
`crates/engine/src/snap.rs` (#20),
`crates/gpu/src/view.rs` (#7), and Arclet application panel
files (#2/#23). Integrate the independent changes only after all
conflicts and required checks are resolved.

## Release gate

Do not merge until cargo fmt, UI compilation, native render screenshots,
per-pane picking, multi-selection and gizmo transaction tests are green.
Measure latency and memory against the existing single viewport, then
fix any regressions. The four-pane behavior must never silently replace
the existing default.
