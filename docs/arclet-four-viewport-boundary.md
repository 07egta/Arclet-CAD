# Arclet four-viewport boundary (Mission 05)

Status: **not implemented**. This is an API hand-off for the rendering/GPU owner. The
Arclet View menu intentionally disables "Four views" and explains why; it must
never suggest that four independent live cameras are available before they are.

## Existing constraints

- `SolveApp.cam`, `SolveApp.cam_anim`, pending fit and `ViewportState` are
  singletons. `viewport::show` and picking read this one camera.
- `viewport_layout::Layout::quad` and `ViewPreset` already supply the pure
  rectangle split and Top/Front/Right/Perspective presets, but render nothing.
- The wgpu callback system prepares callbacks before painting; one renderer's
  uniform/culling state is not independent for concurrent callbacks. Calling
  the existing `viewport::show` four times would render incorrect cameras.
- See `docs/design-four-viewports.md` for audited call sites and trade-offs.

## GPU / UI contract to implement on a separate GPU-owned change

1. GPU provides a stable viewport ID (0–3) in its paint callback and separately
   owned **view-projection uniforms**, viewport/scissor, depth target, visibility
   and per-view culling/transparent sorting state. Model geometry/uploads
   should be shared or demonstrably bounded instead of blindly quadrupled.
2. UI owns a `PaneState` per viewport: camera, animation, projection mode,
   rect, hover, pick cache, active interaction, fit, orbit/pan and settle state.
   Only the active pane receives shortcut and 3D-mouse camera requests.
3. The UI calls the existing `navigation::handle` pathway against the pane's
   camera (rather than introducing an independent navigation algorithm).
   Pointer-down claims a pane until release; a drag crossing the divider
   must not modify another camera.
4. Top, Front and Right are **always orthographic**. Perspective uses its own
   perspective setting and cannot be silently switched to orthographic by
   `auto_ortho_faces`. Selection and model edits remain document-wide.
5. Screenshot, agent `ui.inspect` / `ui.pick`, named-view and fit APIs accept
   an optional pane ID and preserve the single-pane default for old callers.
6. A user preference controls single vs four-pane layout; do **not** change the
   default without visual and performance evidence on Windows and wasm.

## Acceptance before enabling the menu action

- Four independently navigable rendered viewports simultaneously, with
  selection highlight, layer visibility, and accurate screen/world picking.
- egui pointer and keyboard frame tests: resizing, DPI, focus, pane ownership,
  Top/Front/Right/Perspective projection, Escape and viewport switching.
- Before/after screenshots captured from the **running Arclet executable**
  at 1440x900 and 960x600 and checked for missing/clipped controls.
- `cargo test --profile ci -p arclet -p solidcraft-ui-egui`, clippy, fmt,
  wasm check and `cargo xtask ci`, plus a GPU budget check on a large design.

This branch only changes `apps/arclet`, its pending UI strings and this
contract. It does not modify `apps/meltcad` or shared renderer behavior.

## Visual evidence capture

An Arclet screenshot already present on the base branch is
[`docs/screenshots/arclet-ui-2026-11-02.png`](screenshots/arclet-ui-2026-11-02.png).
The dedicated `arclet-desktop-ux.yml` PR workflow builds and runs the **changed
Arclet app**, taking 1440x900 and 960x600 PNGs using the real `ui.screenshot`
path and the explicit `ARCLET_WINDOW_SIZE` capture override. Download the
`arclet-desktop-ux-screenshots` workflow artifact and compare the wide capture
to the committed baseline. The baseline does not show a narrow-window run.
Capture availability by itself is not visual verification; check the images
manually for legibility, clipping and focus states before marking PR ready.

The narrow-screen More menu has a height-bounded scroll area, so commands
remain reachable even with a small window. The command prompt displays the
current design's unit string instead of a fixed `mm` suffix. The capture job
uploads the already-committed baseline alongside two real post-change captures.
