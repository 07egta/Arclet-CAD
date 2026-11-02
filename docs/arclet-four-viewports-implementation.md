# Arclet four-viewport implementation — Mission 22

Status: **Draft / unverified**. This branch preserves the existing one-pane view by default. The four-pane layout is opt-in via Arclet's View menu.

## Architecture

The existing `SolveApp` document and selection remain shared. The new `four_viewports::FourViewports` owns four independent camera, animation and `ViewportState` bundles. A lightweight swap exposes each pane to the already existing `viewport::show`, `navigation::handle`, picking, selection, snapping and drawing overlay pipeline. Top, Front and Right use orthographic projection; Perspective uses a perspective camera. Orthographic panes prohibit orbit through the existing `NavCaps` input path. A press establishes the pane that owns a drag until all pointer buttons are released. Zooming and panning in an inactive pane changes only that pane's camera.

The GPU callback currently supports one prepared camera uniform. Four callbacks would overwrite it before any paint call. Therefore the active pane retains the original single GPU renderer and all other panes draw cached CPU rasterizations under their interactive viewport overlays. The GPU renderer is *moved* between pane states when a different pane becomes active; this does not allocate four renderers or four GPU geometry caches. A cached CPU picture is rebuilt only for edits, camera changes, selection, style, section changes or size changes. Device loss does not make the CPU picture API depend on a GPU adapter.

**Important trade-off:** CPU previews are not fully equivalent to the native GPU renderer, notably for advanced screen-space analyses, occlusion, and performance on large models. A later renderer change should allocate camera-specific uniforms, cull/transparency buffers, depth and supersampling caches per pane while sharing pipelines and mesh uploads. Do **not** claim GPU-accelerated simultaneous four-pane rendering for this PR.

## Existing behavior / persistence

One view is still the default. Toggling to four saves the original single camera in memory and restores it on leaving; the four cameras persist independently for the next switch and in Arclet's own `arclet.four_viewports.v1` eframe storage, not in the model document or the shared SolidCraft prefs. Window geometry is measured each egui frame, so panes respond to window resizes. Maximize shows one pane; restoring shows all four.

## Verification matrix

| Case | Expected | Status |
|---|---|---|
| Four cameras independent | Changing Top zoom and pan does not change Front, Right or Perspective | Unit test authored; not run |
| Ortho/perspective | Fixed Top/Front/Right FOV zero, perspective nonzero | Unit test authored; not run |
| Single view | Same behavior by default, camera restored on exit | Code authored; not run |
| Toggle/maximize/resize | Valid rectangle bounds and four-way tiling | Pure layout test authored; not run |
| Selection consistency | All views share the CAD document selection | UI path wired; interactive test not run |
| Cross-view drag owner | Press origin selects a fixed pane until release | Code authored; interactive test not run |
| GPU memory | Exactly one existing GPU renderer | Static architecture only; memory not measured |
| Device initialization failure | CPU companion pictures do not require GPU | Unit test authored; not run |
| Windows/macOS visual screenshots | All four images, labels and active border visible | Not available in this execution environment |
| Large model | Cached previews responsive and no repeated GPU upload | Not measured |
| Sketch editing, snaps, gizmos | Usable and geometrically correct in each pane | Not validated |

Run `cargo test --locked -p solidcraft-ui-egui --lib four_viewports`, `cargo test --locked -p arclet --bin arclet` and `cargo xtask ci`. Test the actual application at 1440×900 and 960×600, with at least a box and a circle sketch extruded, using every pane to select, pan, zoom and (Perspective only) orbit. Capture genuine Arclet screenshots after building rather than generated mockups.

## Collaboration with Mission 21

[Mission 05 / PR #2](https://github.com/07egta/Arclet-CAD/pull/2) already owns header responsiveness and the View menu placeholder. This branch intentionally does not copy its UI shell redesign. The one overlapping line is the View menu toggle in `apps/arclet/src/app.rs`; resolve it after review so the finished four-view control replaces the disabled placeholder without undoing Mission 21's accessibility or mobile-width menu changes.

Changes a default: **No**.
