# Design study: four-viewport layout (Top, Front, Right, Perspective)

Status: design study only. No code changed. Read-only investigation of `main` at commit `a3b97de`. No `cargo` run, so no timings or pixel results exist yet; every performance statement below is a reading of the code, not a measurement.

Scope: a classic four-pane modelling layout in the egui app. Each pane shows the model through its own camera.

Line references are `path:line` as read at `a3b97de`. Non-test code only unless stated.

## 0. Ground truth in one paragraph

The app has exactly one camera (`SolveApp.cam`, `crates/ui-egui/src/lib.rs:491`), one animation slot for it (`lib.rs:493`), one pending fit (`lib.rs:679`), and one `ViewportState` (`lib.rs:513`). That state holds one rect (`crates/ui-egui/src/viewport.rs:107`), one `Layout` (`viewport.rs:108`), one hover (`viewport.rs:153`), one pick cache (`viewport.rs:106`), and one set of GPU scene slots (`viewport.rs:128-134`). The draw entry point `viewport::show` is one function of about 740 lines (`viewport.rs:3867-4606`) that draws one model into one rect. The GPU side is one `Renderer` stored once in egui's callback resources (`crates/ui-egui/src/gpu.rs:58`), with one uniform buffer (`crates/gpu/src/view.rs:2030`). Pipelines and most state were written for one picture per frame.

The pure parts are already parameterised: `pick(app, proj, pos)` takes a `Proj` (`viewport.rs:2062`), `Layout` is a pure function of rects (`crates/ui-egui/src/viewport_layout.rs:52-160`), and `Camera` is a value type (`crates/render/src/camera.rs:221`). The coupling is in the call sites, which reach for `app.cam` and `app.viewport` directly.

## 1. Where single-camera assumptions live

Counts are from grep over `crates/ui-egui/src`, excluding test files: `app.cam` or `self.cam` about 734 lines, `viewport.rect` about 80, `viewport.layout` 16, `viewport.hover` 85, and `projection(` about 116 call sites.

### 1a. Picking and hover

- `viewport.rs:2021-2061` `pick_cached`: the cache key hashes `app.cam` yaw, pitch, roll, distance and target (`:2026`), `app.perspective_view()` (`:2044`), and the rect. The result is stored in one slot, `app.viewport.last_pick` (`:106`, stored near `:2060`). Four panes would evict each other's entry every frame.
- `viewport.rs:3926` `pick_cached(app, &proj, p)` and `viewport.rs:3938` `app.viewport.hover = ...`: hover is one value. With four panes it needs to be "hover in pane k" (at minimum the pane under the pointer).
- `viewport.rs:162` `boxsel` (box select) is one screen-space rectangle, and `viewport.rs:193` `axis_drag` is one point. A drag that starts in one pane and ends in another has no defined owner.
- `viewport.rs:3872` `ui.interact(rect, ui.id().with("viewport"), ...)`: one interaction id for the whole area.
- `viewport.rs:2490` `section_plane(app)` and `viewport.rs:2510` the section state are model-level (not camera-level), so they should be shared. Good news.
- Callers that build a `Proj` from `app.viewport.rect`: `select_other.rs:378`, `context_menu.rs:1205`, `view_pick.rs:83`, `inference.rs:369`, `control.rs:835-948` (agent `ui.pick` and friends), `render_workspace.rs:1789`, `agent_cursor.rs:248,538`, `browser.rs:2736`, `edit_clipboard.rs:339,440`.

### 1b. Navigation (the one camera path)

- `navigation.rs:453` `handle(app, ui, rect, resp, caps)`: already takes a rect and response, but writes `app.cam` directly: zoom at `:536,551`, pan at `:561,696`, zoom at `:697`, invalid-camera reset at `:728-730`, snapped-axis at `:746-750`.
- `navigation.rs:296` and `:362`: read `app.viewport.layout` and `app.viewport.rect` to decide whether the pointer is over the navigation bar or over the modelling rect.
- `navigation.rs:474` view-angles readout and `navigation.rs:472` axis gizmo rect: both are computed from the one rect.
- `navigation.rs:376` `from = app.cam_anim.map_or(app.cam, ...)`, `:412-423` key-driven turns, `:385-405` key hold: all one camera.
- `navigation.rs:757` and `:761` (`device_camera(&mut app.cam, ...)`, `bridge_camera(&mut app.cam, ...)`): 3D mouse and bridge motion write one camera. Which pane receives them must be defined.
- `navigation.rs:862-905` `orbit_drag`, `:949-979` orbit-about-pivot (`OrbitFrom`), `:995-1006` orbit helpers: all act on `app.cam` and `app.viewport.orbit_free` (`viewport.rs:195`).
- `viewport.rs:146-203` per-camera interaction state, all in `ViewportState`: `nav` `:146`, `primary_nav` `:149`, `modifier_nav` `:151`, `orbit_pivot` `:171`, `orbit_from` `:173`, `spin` `:175`, `device_motion` `:178`, `bridge_motion` `:181`, `axis_drag` `:193`, `orbit_free` `:195`, `ss_view` `:199`.
- `navigation.rs:1529` `hand_rolled_camera_input_is_rejected`: a source-scan guard. Any new pane code that calls `cam.orbit(`, `cam.pan(`, `cam.zoom_at(`, and so on outside `navigation.rs` fails CI. This is the enforcement of the AGENTS.md "one camera path" rule.
- `lib.rs:2087-2104` `fitted` and `fitted_bounds`: use `self.viewport.layout` (`:2098-2101`) to fit the model into the drawable area. This is the only fit path and it assumes one layout.
- `lib.rs:2221-2233` `follow_view_request`: takes `session.camera` (a single request slot, see 1e) and writes `self.cam` at `:2233`.
- `lib.rs:1974-1982` `perspective_view()`: with `auto_ortho_faces` on, the projection is decided from `self.cam.aligned_standard_view(...)`. Four panes must not let this flip the Top, Front or Right pane's projection, so per-pane projection has to override it.
- `viewport.rs:368-378` `projection(app, rect)`: copies `app.cam` (`:369`), forces `fov` from `perspective_view()` unless Render is active (`:372-373`), and builds the view-projection with scene bounds (`:376`). This is the funnel for almost all picking and drawing.
- `navigation_cube.rs:123` and `:157`: the axis gizmo draws and hit-tests facets of `app.cam`.
- `dialogs_sheet.rs:1158` (`cam` field): a separate camera struct for flat pages; `viewport.rs:3877` routes flat pages to `dialogs_sheet::navigate` instead of `navigation::handle`. Pane work should leave it alone.

### 1c. Rendering

- `viewport.rs:4000-4046`: one `Frame` per frame, with one `view_proj` (`:4020`), `back` (`:4021`), `eye` (`:4034`), `size_px` (`:4022`), and one `painter.add(egui_wgpu::Callback::new_paint_callback(rect, cb))` (`:4046`).
- `viewport.rs:3971-3985`: the model scene is rebuilt only when `scene_key` changes, and stored in one `slot` (`:3976`). Sketch and highlight scenes follow the same pattern (`:3980-3998`).
- `viewport.rs:437-460` `scene_key` depends on the camera: `app.cam.up` (`:455`) and `app.cam.pitch > 0.0` (`:456`). The model scene itself does too: ground shadow is built only when `pitch > 0` (`viewport.rs:1440`). A Top pane (`pitch = +pi/2`) and a Front pane (`pitch = 0`) therefore produce different scene keys, so alternating panes would rebuild the whole model scene every frame. This is a concrete trap; see section 2.
- `viewport.rs:487-503` `sketch_scene_key` includes `projection_key(proj)` (sketch scene depends on the view).
- `sketch_display.rs:14-23` `projection_key` hashes the full view-projection and rect. `sketch_display.rs:219-226` `DisplayCache::reset` clears curves, profiles and budgets whenever the view changes. Per-pane projections would clear the cache every pane switch.
- `viewport.rs:543-600` `supersampling`, `ao_samples`, `view_settled`: adaptive supersampling and ambient occlusion wait until the view has held still for `SS_SETTLE` (0.25 s, `:591`), tracked in the single `ss_view` (`viewport.rs:199`). Alternating panes never settle, so supersampling and AO would never turn on.
- `viewport.rs:668-720` `CameraRest`, `lod_view` (`:85`), `lod_rest` (`:87`), `update_refinement` (`:687-720`): display refinement is chosen per view and waits for the camera to stop. Same problem: one slot, four views.
- `viewport.rs:2755` `axis_view_size`, `viewport.rs:5565-5570` `axis_rect`: the axis gizmo rect comes from the single `layout.axis_view`.

GPU side (`crates/gpu/src/view.rs`, glue in `crates/ui-egui/src/gpu.rs`):

- `gpu.rs:29-58` `install`: builds one `Renderer` and inserts it once into `rs.renderer.write().callback_resources` (`gpu.rs:58`). Callbacks are looked up by type, so there is one renderer.
- `gpu.rs:177-200` `ViewportCallback` implements `prepare` (`:178`) and `paint` (`:192`).
- `vendor/egui-wgpu/src/renderer.rs:1066-1077`: egui runs every callback's `prepare` in the frame before any `paint`. `paint` is called later in the render pass, at `vendor/egui-wgpu/src/renderer.rs:610`.
- `crates/gpu/src/view.rs:3281-3330` `Renderer::prepare`: writes the single shared uniform (`:3300`, `:3328`), updates the model, sketch, highlight and preview scenes (`:3297`, `:3308`, `:3318`), and culls against `cb.view_proj` (`:3327`, cull at `:1830`). The culled instance buffers and the glass-sort order (`:3111`, keyed on `back`) are per renderer, not per pane.
- `crates/gpu/src/view.rs:2030` `uniform: wgpu::Buffer`, and `:2086-2089` `ss_key`, `cached`: one supersampled picture, one cache key.
- `crates/gpu/src/view.rs:3406-3440` `paint` sets `pass.set_viewport` from the callback's rect (`:3411`) and then draws with the state left by `prepare`.

Consequence, which drives the whole GPU design: with one `Renderer` and four callbacks, every `paint` reads the uniform, culling and glass state written by the last `prepare`. All four panes would show the last pane's camera. A second callback sharing the renderer is not enough; see candidate B.

Capture and overlay paths:

- `crates/ui-egui/src/control.rs:456-461` `view_scene(app, guides)`: builds the render-side `Scene` with `&app.cam` (used by thread markings and the ground grid). `control.rs:475-478` `view_rgba` draws it through `engine::capture::rgba`.
- `crates/engine/src/capture.rs:36,41` `Renderer` type and `install`. `crates/ui-egui/src/gpu.rs:100-108` `install_picture_renderer` installs a separate GPU capturer (`crates/gpu/src/capture.rs`) and falls back to the CPU rasterizer. This is a second, independent GPU path, so thumbnails could use it without touching the main `Renderer` (candidate C).
- `control.rs:582-588` `ui.inspect` reports one `viewport` rect and one `camera`. Agents drive the UI through this surface, so it needs a pane argument.
- `viewport.rs:5385` `overlays(app, painter, proj)` takes a `Proj`, so it is easy to pass per pane. `viewport.rs:5266` `hover_highlight`, `viewport.rs:5246-5262` grid helpers, and `viewport.rs:217` `grid_plane` (one `Mutex<GridPlane>`) are per-view.
- `viewport.rs:4630` `agent_chip`, `viewport.rs:4678` `transient_messages`, `viewport.rs:4829` toast: all take the one rect and "free" rectangles from the one layout.
- `viewport.rs:5510-5550` `refresh_layout`: builds `app.viewport.layout` from the one `rect`, `shell::content_rect`, navigation bar measurement, and dock obstacles. `viewport_layout.rs:95-160` `Layout::build` reserves one axis gizmo, one navigation bar and one confirm pair per layout. Four panes need four layouts, or one layout plus per-pane sub-rects.
- `lib.rs:2615-2621` `show_workspace`: the one branch point where the modelling viewport or the Render workspace is drawn. This is the natural seam for a pane container.

### 1d. Standard views, keys and commands

- `crates/engine/src/cmd/view_navigation.rs:97-103`: `view.front`, `view.back`, `view.left`, `view.right`, `view.top`, `view.bottom`, `view.isometric` with keys Ctrl+1 to Ctrl+7. These are the natural pane presets.
- `crates/engine/src/cmd/view_navigation.rs:143-155` `turn_to`: takes an optional `camera` parameter, falls back to `s.camera` or the home camera, and writes `s.camera` (`:153`). The result is a single request slot with no pane identity.
- `crates/engine/src/lib.rs:424` `Session.camera`: "view state, not saved", a request for the viewport to move. Since it is one slot, pane targeting needs either a `pane` field next to it or a per-pane request map.
- `crates/render/src/camera.rs:61-71` `StandardView::angles`: Front `(0, 0)`, Right `(-pi/2, 0)`, Top `(0, +pi/2)`, Bottom `(0, -pi/2)`. So the Top pane has `pitch > 0` and triggers the ground-shadow and `scene_key` effect in 1c.
- `navigation.rs:362-377` (keyboard turns) and `navigation.rs:405` (`view.navigate` with the current camera): one camera.

### 1e. Preferences and saved state

- `lib.rs:334-337` `ui.perspective`, `ui.auto_ortho_faces`. `lib.rs:371-372` defaults. `lib.rs:943-953` `prefs()` JSON stores `perspective` (`:949`) and `auto_ortho_faces` (`:950`). A four-pane layout needs its own prefs keys (layout on/off, which pane is Perspective) and must not overload `perspective`.
- `viewport.rs:2755` `axis_view_size(style)`: uses `preferences.navigator_style`, so the gizmo size per pane follows one preference (fine, but the gizmo must be re-fit per pane rect).
- `crates/doc/src/document.rs:2828-2838` `NamedView` (saved camera in the document: target, yaw, pitch, distance, fov). `crates/engine/src/cmd/browser.rs:721-737` stores it; `crates/ui-egui/src/browser.rs:1741` `view.save` sends the current `app.cam`. This is document state. Decision needed: does a named view record a pane, or always the active camera?
- `crates/doc/src/render_settings.rs:21` and `crates/ui-egui/src/render_workspace_shots.rs:15` store `app.cam` into Render settings. This is Render-only and can stay on the active camera.
- `prefs.rs:1790` reads `app.cam.target` for pivoting (one call site, low risk).

### 1f. Existing precedent to copy

- `crates/ui-egui/src/render_workspace.rs:1281-1300` `with_result_camera`: temporarily swaps `app.cam`, `ui.perspective`, `cam_anim`, `pending_fit`, `session.camera`, `viewport.orbit_free` and `viewport.spin` with a second camera, runs a closure, and swaps back. This is already a working "run the navigation code against another camera" mechanism. Its field list is a good first draft of the per-pane state bundle.
- `crates/gpu/src/view.rs:949-950` `Frame.product`: the product preview has its own uploads inside the same `Renderer` (`res.product` next to `res.model`, `view.rs:3297`). That is a precedent for several scene sets, but not for several cameras.

### 1g. Tests and guards that will break or need extending

- `navigation.rs:1529` camera-routing guard (above).
- `crates/ui-egui/src/viewport_layout_tests.rs` (41 hits on the layout API): pure tests of `Layout` and `fit_bounds`. The easiest place to add four-pane layout tests.
- `crates/ui-egui/src/scenario.rs:143` `Harness`: headless UI harness used by many `*_tests.rs` files, with `app.cam` assertions everywhere.
- `crates/ui-egui/src/control.rs` `ui.inspect`, `ui.pick`, screenshot: agent-facing surfaces that must learn a pane argument.
- `crates/ui-egui/tests/*.rs` integration tests, for example `orbit_empty_space_pivot.rs` and `projection_preselection.rs`, use `projection` and camera state directly.

## 2. Candidate designs

Sizes are rough guesses from the call-site counts above. They are not estimates from a working prototype.

### Candidate A: N independent viewports, each owning its camera and sharing the scene

Shape: a `Pane` struct holds one camera, its animation, pending fit, orbit state, perspective flag and one `ViewportState`. `app.panes` holds four of them; the existing field names become "the active pane" through a swap helper modelled on `with_result_camera` (`render_workspace.rs:1281`). Each pane runs `navigation::handle` and `viewport::show` against its own rect.

GPU: give each pane its own `Renderer`. `callback_resources` is keyed by type, so one wrapper type (for example `PaneRenderers`) holding four `Renderer`s, selected by a `pane` index added to `Frame`, works without touching the renderer's internals. The uniform, culling and glass state then stop interfering, because each pane owns its own.

Size: GPU glue is small (`gpu.rs:29-58`, `:177-200`, `Frame` at `view.rs:948`). The UI state move is large: roughly 25 `ViewportState` fields (`viewport.rs:51-217`), `SolveApp.cam`, `cam_anim`, `pending_fit`, and about 116 `projection` call sites. The swap trick keeps the diff small, but `show` (about 740 lines, `viewport.rs:3867-4606`) must still be split so per-pane drawing is one call.

Costs and risks:
- Scene uploads: `Renderer::prepare` uploads model, sketch and highlight scenes per renderer (`view.rs:3297-3318`). Four renderers means four copies of the GPU scene, so roughly four times the memory and upload time. This is the main risk for large assemblies (the code already has a lowered-quality path for devices that refuse a frame's memory, `#7296`: `viewport.rs:4047-4049` and `crates/gpu/src/view.rs:178`).
- Startup: `Renderer::with_window_target` (`view.rs:2422`) builds pipelines and shader source per instance. Four instances means four pipeline sets, unless pipelines are factored out (a refactor of `view.rs`).
- CPU scene: `slot`, `sketch_slot` and `hl_slot` live in `ViewportState` (`viewport.rs:128-134`). If they stay per pane, `build_scene` (`viewport.rs:1240`) runs four times. Shared slots should move to the app level, keyed by `scene_key` (which still includes `pitch > 0`, see section 1c).
- Per-pane `scene_key` must drop the camera dependence, or it must be applied per pane (see open question 3).
- Settle timers (`ss_view`, `lod_rest`) and the sketch display cache must be per pane, or they thrash.

AGENTS.md rules: one camera path is preserved if each pane calls `navigation::handle` with its own camera, and the guard test (`navigation.rs:1529`) is extended to cover pane code. One outer container is preserved if the four panes are sub-rects inside `show_workspace` (`lib.rs:2615`), with no new egui `Panel` or `Area` (the host list in `xtask/src/ui_containers.rs:16`). Colours still come from `Tokens` (`viewport.rs:3868`). New UI strings go to `assets/locales/pending/`. The layout is a changed default, so the old single view must stay behind a preference.

Test strategy:
- Pure: `Layout` for each pane rect (extend `viewport_layout_tests.rs`), and "pane camera equals standard view" tests using `StandardView::angles`.
- Headless: `Harness` (`scenario.rs:143`) with four panes, asserting that a drag in pane 2 changes only pane 2's camera.
- Golden: a screenshot of the Top pane through `ui.screenshot` must match the single-viewport Top view (`view.top`) pixel for pixel, within a tolerance for AA.
- Perf: a frame-time counter per pane (`DrawStats`, `gpu.rs:`-era `draw_stats`) on a large model, before and after.

### Candidate B: one render pass per pane using viewport and scissor rects, into the same target

Shape: keep one `Renderer` and one scene set, but make `Renderer` hold per-pane state: one uniform buffer and bind group per pane, per-pane culled instance buffers, per-pane glass order, per-pane supersampled picture. Then one `ViewportCallback` draws all four panes: `prepare` loops over panes writing each uniform and culling; `paint` loops over panes, calls `set_viewport`/`set_scissor_rect` and the pane's bind group, and draws. Depth is cleared per pane with the existing depth-reset pipeline (`depth_reset_pipeline`, `crates/gpu/src/view.rs:2792`, used in `paint`).

Sub-variant B2 (the one I would pick if B is pursued): put all four panes in one callback, so egui's prepare-before-paint ordering (section 1c) cannot leak state between panes. Each pane's uniform lives in its own buffer, so no dynamic offsets are needed.

Size: large and concentrated. `Renderer` is `view.rs:1969` onward with about 1,500 lines of fields and draw methods, and `prepare` (`:3281-3330`) and `paint` (`:3406-3440`) need pane loops. The UI glue is medium. Estimate 1,000 to 2,000 lines in `crates/gpu/src/view.rs`, plus tests in `crates/gpu/tests/` (for example `depth.rs`, `thin_lines.rs`, `capture.rs`, which already exist as GPU test harnesses).

Costs and risks:
- Scene uploads, pipelines and shader compile happen once. This is the efficient end state.
- The supersampling and AO path (`view.rs:3297-3307`, `ss_ready`, `ss_key`, `cached` at `:2086-2089`) is one-picture-per-frame; with four panes it needs per-pane caches or must be turned off for panes that are not active.
- Depth reset and transparency (`see_through`, OIT) must be re-checked per pane.
- wgpu ordering: `queue.write_buffer` calls are applied in submission order, not interleaved with recorded encoder commands. That is my reading of the wgpu queue model, not something I tested here. It is the reason per-pane uniforms must be separate buffers rather than one buffer written four times.
- Highest chance of a subtle visual regression, and the hardest to test without a GPU in CI (the GPU tests use `offscreen_device`, `gpu.rs` test support, and skip when there is no adapter).
- Device-lost and "lowered quality" fallbacks (`crates/gpu/src/view.rs:178` and `:2093`, and `viewport.rs:4047-4049`) must cover four panes.

AGENTS.md rules: the same as A. The GPU crate change does not touch the camera path or containers. Changed default still needs a preference.

Test strategy:
- GPU: an offscreen test that renders two panes with different cameras into one target and compares each region with a single-pane render of the same camera (strict equality of pixels, within a tolerance).
- Regression: the existing `crates/gpu/tests/depth.rs` and `thin_lines.rs` must still pass with one pane.
- Perf: frames per second and draw calls per pane (`DrawStats`).

### Candidate C: one active full viewport plus three small read-only orthographic thumbnails

Shape: the main viewport stays exactly as it is, with one camera, one `Renderer` and one set of caches. The rect is split into one large area (the active pane, Perspective by default) and three small thumbnails (Top, Front, Right), all in orthographic. Thumbnails are not navigable. They are drawn through the existing headless capture path (`engine::capture::rgba`, `crates/engine/src/capture.rs:36-41`, GPU capturer `gpu.rs:100-108`) into egui images, refreshed only when `scene_key` changes or the model is idle. A click on a thumbnail calls an existing standard-view command (`view.top`, `view.front`, `view.right`, `view_navigation.rs:97-103`), so the camera change goes through the existing path and no new camera code is needed.

Size: small to medium. A thumbnails module, a cache keyed by (scene key, standard view, size), a `view_scene` variant that takes its camera (`control.rs:456-461` currently uses `&app.cam`), and a layout split in `refresh_layout` (`viewport.rs:5510-5550`) plus `Layout` sub-rects. Estimate 300 to 600 lines.

Costs and risks:
- Each thumbnail refresh builds a full CPU `Scene` (`engine/src/view.rs:571`, `scene_with_thread_markings`) and then draws it. That is three extra full scene builds whenever the model changes. Cost not measured.
- Input routing: the thumbnail rects must be excluded from `navigation::handle` (as `axis_rect` is at `navigation.rs:472`) and from picking, and must not own the pointer during drags.
- Thumbnails are a different picture from the live view (CPU or capturer output vs the window's pipelines). Visual consistency (grid, section cap, outline style) has to be checked.
- Memory: three extra egui textures per refresh.

AGENTS.md rules: no camera code is added, so the camera path is untouched. Thumbnails sit inside the same container. The stand-alone capture path already exists and is used for headless pictures, so it is an existing, tested route.

Test strategy:
- Thumbnail for Top equals `ui.screenshot` of `view.top` at the same size (pixel equality within tolerance).
- Cache test: no refresh when the model and view are unchanged (count capture calls through a fake renderer via `engine::capture::install`).
- Click test: a click on a thumbnail issues `view.top` and the active camera changes to the Top angles.
- Interaction test: a drag starting in a thumbnail does nothing to the camera.

Note: C is the cheapest honest first step toward four panes. It proves layout, input exclusion and the "click a view" workflow without touching the GPU multi-state problem. It is not the final four-equal-pane layout.

## 3. Recommended incremental plan

Each step is shippable on its own. Steps 1 to 3 change no behaviour. Step 0 is process, per the AGENTS.md contributing rule (claim with a draft PR, look at open PRs first).

0. Claim the work with a draft PR (lowest number wins, `skills/solidcraft-contributing/SKILL.md`); check open PRs for overlap with `viewport.rs`, which is a hot file.
1. Pure additions: `Layout::quad(rect) -> [Rect; 4]` and a `ViewPreset` mapping to `StandardView::angles` (`crates/render/src/camera.rs:61-71`), with tests in `viewport_layout_tests.rs`. Nothing calls them yet.
2. Parameterise the camera: add `projection_for(app, cam, perspective, rect)`; make `projection(app, rect)` a thin wrapper; key `pick_cached` by the camera it is given (`viewport.rs:2021-2061`) instead of `app.cam`. Behaviour identical; all existing tests run unchanged.
3. Pane container with one pane: introduce a `Pane` struct holding the per-camera bundle (cam, cam_anim, pending_fit, orbit_free, spin, perspective, and per-pane `ViewportState` fields listed in section 1b), with one active pane = today's behaviour. Use the swap pattern of `with_result_camera` (`render_workspace.rs:1281`) to run `show` against a pane. Guard it with the existing navigation test.
4. Thumbnails (candidate C), behind a preference that defaults off: read-only Top, Front and Right thumbnails via the capture path, cached by `scene_key`, click runs `view.top` etc. Also add `view_scene(app, cam, guides)` so the thumbnail camera is passed explicitly (`control.rs:456`). Screenshot golden test per thumbnail. Ship with a real screenshot per AGENTS.md.
5. Agent and command surface: add an optional `pane` argument to `view.front`..`view.isometric` (`view_navigation.rs:97-155`), to `ui.inspect` (`control.rs:582-588`) and to screenshot and pick requests (`control.rs:835-948`). Keep the default as the active pane. Update command schemas (`skills/solidcraft-commands/SKILL.md`). 3D mouse input goes to the active pane (`navigation.rs:757,761`).
6. Pane-aware navigation and settle state: call `navigation::handle` per pane rect with that pane's camera; move `ss_view` (`viewport.rs:199`), `lod_rest` and `lod_view` (`viewport.rs:85-87`) and the sketch display cache to per-pane. Test that a drag in pane 2 leaves pane 1's camera, hover and pick cache unchanged.
7. Four equal panes on the CPU and capture path first, then the GPU: give each pane its own `Renderer` through a `pane` field on `Frame` (`gpu.rs:177-200`, `view.rs:948`), and measure memory and frame time on a large model with `DrawStats`. Move to candidate B2 only if the measurement says the per-pane copies are too costly.
8. Default and polish: four panes as a preference, with the single view kept as the default until screenshots and a perf check pass; then switch the default with a changed-default note (AGENTS.md rule). Add locale strings in `assets/locales/pending/`.

### Step 1 done

Pure additions in `crates/ui-egui/src/viewport_layout.rs`, with no callers yet, so no behaviour changes:

- `Layout::quad(rect) -> [Rect; 4]`: splits a rect into a 2 x 2 grid, returned as [top left, top right, bottom left, bottom right]. A rect that is not finite or has no area gives four empty rects.
- `ViewPreset` (`Top`, `Front`, `Right`, `Perspective`, with `ViewPreset::ALL` in that order): `standard_view()` and `angles()` delegate to `StandardView::angles` (`crates/render/src/camera.rs`). Perspective uses the iso angles. `is_perspective()` is true only for Perspective.
- Tests in `crates/ui-egui/src/viewport_layout_tests.rs`: the quads tile the rect without overlap, empty and non-finite input is handled, and the presets match the standard-view angles.

Not changed: `SolveApp.cam`, `ViewportState` and every call site. The brief's example for step 1 (grouping per-viewport state into a struct) is the doc's step 3 and is still to do.

## 4. Open questions

1. Interaction ownership: which pane owns a drag that starts in one pane and ends in another, and does hover need to be per pane (`viewport.rs:153`) or only the pane under the pointer?
2. Sketching: `perspective_view()` already forces orthographic during a sketch (`lib.rs:1975-1977`). Should a sketch open in a single full pane, or in whichever pane is Top/Front/Right? Sketch planes are chosen from `active_view`.
3. Ground shadow and `scene_key`: the ground shadow depends on `pitch > 0` (`viewport.rs:1440`, `:456`). Should it be a model-level choice (same in every pane), or per pane with separate scenes? The model-level choice avoids the rebuild trap but changes the look of the Top pane in single-view mode unless guarded.
4. Perspective pane and `auto_ortho_faces`: should the Perspective pane ignore `auto_ortho_faces` (`lib.rs:1978-1979`) and the three ortho panes always be orthographic? Probably yes; that needs an explicit rule.
5. Named views and renders: should `view.save` (`browser.rs:1741`) record the active pane's camera, and should Render use the active pane or always the Perspective pane?
6. Layout preference and persistence: where do the layout mode and per-pane presets live (`lib.rs:943` JSON prefs vs a document field)? Document-level layout would change `.solidcraft` files, which the file-format rules govern.
7. Overlays with four panes: the navigation bar and confirm pair (`viewport_layout.rs:95-160`) are one per layout. One shared bar for the active pane, or one per pane?
8. Scene memory on the GPU: how many copies of the model scene can the target machines hold? This decides between candidate A's per-pane renderers and B2. It needs a measurement on a large model (Blender-class or the bundled examples).
9. wasm: `apps/solidcraft-web` shares `ui-egui`. Does the capture path exist on wasm (`cargo xtask wasm`)? The thumbnail design depends on it.
10. Measurement: nothing here was timed. Step 7 and the candidate costs above need real frame-time numbers before any choice is final.

## 5. What I did not verify

- No build or test was run (`cargo` was forbidden for this study).
- The wgpu ordering statement in candidate B is my reading of the wgpu queue model. The egui-wgpu ordering (prepare for all, then paint) was read from the vendored source.
- I read the navigation, viewport, GPU and capture paths, plus call-site counts by grep. I did not read every one of the roughly 116 `projection` call sites or the roughly 734 `app.cam` lines individually; the list in section 1 is representative, not exhaustive.
- `viewport.rs` is about 10.8k lines. I read its state struct, the draw entry point, picking cache, refresh and layout functions, and the scene-key functions. Some draw helpers (`build_highlight`, `box_select`, `overlays`) were only checked for their signatures.
