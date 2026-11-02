//! Experimental native four-view interaction. All panes share the same CAD Session but
//! have independently owned cameras, viewport caches, navigation gestures and pending fits.
//! This is opt-in: the previous single viewport remains unchanged until ui.quad_views is set.
//!
//! A single egui-wgpu Renderer cannot draw four independent callbacks: prepare is run for
//! all callbacks before paint, so uniforms/culling would all belong to the last pane.
//! The quad callbacks therefore use independent renderers (see gpu::QuadViewportCallback).

use egui::{Color32, Pos2, Rect, Stroke, Ui, UiBuilder, pos2, vec2};
use solidcraft_engine::geom::Aabb3;
use solidcraft_engine::render::{Camera, CameraAnim};

use crate::viewport::ViewportState;
use crate::viewport_layout::{Layout, ViewPreset};
use crate::SolveApp;

struct Pane {
    camera: Camera,
    animation: Option<CameraAnim>,
    fit: Option<(Camera, Aabb3, bool)>,
    viewport: ViewportState,
}

impl Default for Pane {
    fn default() -> Self {
        Self { camera: Camera::default(), animation: None, fit: None, viewport: ViewportState::default() }
    }
}

/// Stored for the lifetime of the window, never saved into the CAD design.
/// The active pane lives in SolveApp's established single-camera fields. Its slot in
/// `panes` is the spare staging slot until a different pane becomes active.
pub(crate) struct State {
    pub enabled: bool,
    pub active: usize,
    pub maximized: Option<usize>,
    pub rendering: Option<usize>,
    pub drag_owner: Option<usize>,
    initialized: bool,
    panes: Box<[Pane; 4]>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            enabled: false,
            active: 3,
            maximized: None,
            rendering: None,
            drag_owner: None,
            initialized: false,
            panes: Box::new(std::array::from_fn(|_| Pane::default())),
        }
    }
}

fn swap_active(app: &mut SolveApp, pane: usize) {
    if pane >= ViewPreset::ALL.len() {
        return;
    }
    let slot = &mut app.quad.panes[pane];
    std::mem::swap(&mut app.cam, &mut slot.camera);
    std::mem::swap(&mut app.cam_anim, &mut slot.animation);
    std::mem::swap(&mut app.pending_fit, &mut slot.fit);
    std::mem::swap(&mut app.viewport, &mut slot.viewport);
}

/// Selecting a pane is a swap, not a copy: the old camera and all of its UI/pick caches
/// survive, and the chosen pane owns the CAD viewport until another pane takes focus.
pub(crate) fn activate(app: &mut SolveApp, next: usize) {
    if next >= ViewPreset::ALL.len() || next == app.quad.active {
        return;
    }
    let old = app.quad.active;
    swap_active(app, old);
    swap_active(app, next);
    app.quad.active = next;
}

/// Give every axis view its own camera, initialized from the existing single-view
/// target/distance and world-up. Repeated layout toggles retain all the camera positions.
fn initialize(app: &mut SolveApp) {
    if app.quad.initialized {
        return;
    }
    for (i, preset) in ViewPreset::ALL.iter().enumerate().take(3) {
        let mut cam = app.cam;
        cam.set_view(preset.standard_view());
        cam.fov = 0.0;
        app.quad.panes[i].camera = cam;
        app.quad.panes[i].viewport.gpu = app.viewport.gpu.map(|gpu| crate::gpu::GpuTarget { tess: false, ..gpu });
    }
    app.quad.initialized = true;
}

pub(crate) fn pane_rects(stage: Rect, maximized: Option<usize>) -> [Rect; 4] {
    if let Some(i) = maximized.filter(|i| *i < 4) {
        let mut rects = [Rect::NOTHING; 4];
        rects[i] = stage;
        rects
    } else {
        Layout::quad(stage)
    }
}

/// Exclusive partition: shared edges belong to the pane on their right/bottom.
/// Pointer points outside all pane rectangles never enter drawing or picking.
pub(crate) fn pane_at(rects: &[Rect; 4], p: Pos2) -> Option<usize> {
    rects.iter().enumerate().find(|(_, r)| r.is_positive() && r.left() <= p.x && p.x < r.right() && r.top() <= p.y && p.y < r.bottom()).map(|(i, _)| i)
}

fn draw_pane(app: &mut SolveApp, ui: &mut Ui, index: usize, rect: Rect) {
    if !rect.is_positive() {
        return;
    }
    let selected = app.quad.active;
    if index != selected {
        swap_active(app, selected);
        swap_active(app, index);
    }
    app.quad.rendering = Some(index);
    let previous_perspective = app.ui.perspective;
    let previous_auto = app.ui.auto_ortho_faces;
    app.ui.perspective = ViewPreset::ALL[index].is_perspective();
    app.ui.auto_ortho_faces = false;

    // The quad GPU renderer deliberately uses the CPU-prepared scene, avoiding a
    // second tessellation pipeline tied to the main single-view Renderer.
    let previous_gpu = app.viewport.gpu;
    app.viewport.gpu = previous_gpu.map(|gpu| crate::gpu::GpuTarget { tess: false, ..gpu });
    let mut pane_ui = ui.new_child(UiBuilder::new().max_rect(rect).id_salt(("quad_view", index)));
    pane_ui.set_clip_rect(rect);
    crate::viewport::show(app, &mut pane_ui);
    app.viewport.gpu = previous_gpu;
    app.ui.perspective = previous_perspective;
    app.ui.auto_ortho_faces = previous_auto;
    app.quad.rendering = None;
    if index != selected {
        swap_active(app, index);
        swap_active(app, selected);
    }
    // Pane outline and active distinction are UI paint only; never change hit tests.
    let accent = if index == selected { crate::theme::Tokens::get().accent } else { Color32::from_gray(84) };
    let painter = ui.painter_at(rect);
    let l = rect.left();
    let r = rect.right();
    let t = rect.top();
    let b = rect.bottom();
    for (p, q) in [
        (pos2(l, t), pos2(r, t)),
        (pos2(r, t), pos2(r, b)),
        (pos2(r, b), pos2(l, b)),
        (pos2(l, b), pos2(l, t)),
    ] {
        painter.line_segment([p, q], Stroke::new(if index == selected { 2.0 } else { 1.0 }, accent));
    }
}

/// A desktop-only opt-in workspace, still using viewport::show and navigation::handle for
/// every pane. A click/drag has one captured pane; moving across a divider does not
/// reassign the active camera. No separate document or scene evaluation is created.
pub(crate) fn show(app: &mut SolveApp, ui: &mut Ui) {
    let toggle = ui.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Num4));
    if toggle {
        app.ui.quad_views = !app.ui.quad_views;
        app.quad.maximized = None;
        app.quad.drag_owner = None;
    }
    app.quad.enabled = app.ui.quad_views;
    if !app.quad.enabled {
        app.quad.rendering = None;
        crate::viewport::show(app, ui);
        return;
    }
    initialize(app);
    let whole = ui.max_rect();
    if whole.width() < 120.0 || whole.height() < 110.0 {
        crate::viewport::show(app, ui);
        return;
    }
    const BAR: f32 = 28.0;
    let stage = Rect::from_min_max(pos2(whole.left(), whole.top() + BAR), whole.max);
    let rects = pane_rects(stage, app.quad.maximized);
    let (pointer, pressed, released, down) =
        ui.input(|i| (i.pointer.hover_pos(), i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.primary_down()));
    let under = pointer.and_then(|p| pane_at(&rects, p));
    if pressed && app.quad.drag_owner.is_none() {
        app.quad.drag_owner = under;
    }
    if let Some(i) = app.quad.drag_owner.or(if down { None } else { under }) {
        activate(app, i);
    }
    for (i, rect) in rects.into_iter().enumerate() {
        draw_pane(app, ui, i, rect);
    }
    // Release belongs to the original pane for the whole render; clear ownership after
    // all four panes have had their chance to finalize their input/Undo steps.
    if released || !down {
        app.quad.drag_owner = None;
    }

    // The controls live outside all viewport rects: they can never click through into
    // a model or become a gizmo drag. No extra Window, Panel or Area is introduced.
    let origin = whole.min + vec2(6.0, 3.0);
    let single = Rect::from_min_size(origin, vec2(104.0, 22.0));
    if ui.put(single, egui::Button::new("Single view")).clicked() {
        app.ui.quad_views = false;
        app.quad.maximized = None;
    }
    let maximize = Rect::from_min_size(origin + vec2(112.0, 0.0), vec2(128.0, 22.0));
    let label = if app.quad.maximized.is_some() { "Restore 4 views" } else { "Maximize active" };
    if ui.put(maximize, egui::Button::new(label)).clicked() {
        app.quad.maximized = if app.quad.maximized.is_some() { None } else { Some(app.quad.active) };
    }
    let preset = ViewPreset::ALL[app.quad.active];
    let text = format!("{preset:?}  |  Ctrl+Shift+4");
    ui.painter_at(whole).text(origin + vec2(250.0, 11.0), egui::Align2::LEFT_CENTER, text, egui::FontId::proportional(12.0), Color32::GRAY);
}
