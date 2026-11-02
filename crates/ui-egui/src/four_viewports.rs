//! Arclet's opt-in four-pane modeling viewport.
//!
//! Keep the shared CAD document/selection and a single wgpu viewport renderer;
//! the active pane uses the normal GPU callback, while other panes have cached
//! CPU pictures underneath the *same* picking, sketch, and overlay UI. Changing
//! the active pane moves the one GPU target rather than uploading four copies
//! of the model to four renderers. Do not call this from Render or flat pages.

use std::hash::{Hash, Hasher};

use egui::{ColorImage, FontId, Pos2, Rect, Stroke, TextureHandle, TextureOptions, Ui, pos2, vec2};
use serde_json::{Value, json};
use solidcraft_engine::render::{Camera, CameraAnim};

use crate::viewport::ViewportState;
use crate::viewport_layout::{Layout, ViewPreset};
use crate::{SolveApp, viewport};

const PANES: usize = 4;
const MAX_PREVIEW_PX: f32 = 768.0;

struct Pane {
    cam: Camera,
    anim: Option<CameraAnim>,
    view: ViewportState,
    picture: Option<TextureHandle>,
    picture_key: Option<u64>,
}

impl Pane {
    fn new(preset: ViewPreset) -> Self {
        let mut cam = Camera::default();
        cam.set_view(preset.standard_view());
        cam.fov = if preset.is_perspective() { 0.6 } else { 0.0 };
        Self { cam, anim: None, view: ViewportState::default(), picture: None, picture_key: None }
    }
}

/// Four independent camera/interaction caches, one shared document and GPU target.
/// The active pane's camera and viewport state live directly in SolveApp so
/// standard engine commands, tools, hover and navigation retain their old path.
pub struct FourViewports {
    pub enabled: bool,
    pub active: usize,
    pub maximized: Option<usize>,
    panes: [Pane; PANES],
    initialized: bool,
    ever_started: bool,
    single: Option<(Camera, bool)>,
    drag_owner: Option<usize>,
}

impl Default for FourViewports {
    fn default() -> Self {
        Self {
            enabled: false,
            active: 3,
            maximized: None,
            panes: std::array::from_fn(|i| Pane::new(ViewPreset::ALL[i])),
            initialized: false,
            ever_started: false,
            single: None,
            drag_owner: None,
        }
    }
}

impl FourViewports {
    /// This is a window layout, never part of the CAD document format.
    pub fn prefs(&self, active_camera: Camera) -> String {
        let cameras: Vec<_> = (0..PANES).map(|i| if i == self.active && self.initialized { active_camera } else { self.panes[i].cam }).collect();
        json!({"enabled": self.enabled, "active": self.active, "maximized": self.maximized, "cameras": cameras}).to_string()
    }

    pub fn load_prefs(&mut self, text: &str) {
        let Ok(value) = serde_json::from_str::<Value>(text) else { return };
        self.enabled = value["enabled"].as_bool().unwrap_or(false);
        self.active = value["active"].as_u64().and_then(|i| usize::try_from(i).ok()).filter(|&i| i < PANES).unwrap_or(3);
        self.maximized = value["maximized"].as_u64().and_then(|i| usize::try_from(i).ok()).filter(|&i| i < PANES);
        if let Some(cameras) = value["cameras"].as_array().filter(|cameras| cameras.len() == PANES) {
            for (i, item) in cameras.iter().enumerate() {
                if let Ok(cam) = serde_json::from_value::<Camera>(item.clone())
                    && cam.is_valid()
                {
                    self.panes[i].cam = cam;
                }
            }
            self.ever_started = true;
        }
    }

    fn enter(&mut self, app: &mut SolveApp) {
        if self.initialized {
            return;
        }
        self.single = Some((app.cam, app.ui.perspective));
        if !self.ever_started {
            for (i, pane) in self.panes.iter_mut().enumerate() {
                pane.cam.target = app.cam.target;
                pane.cam.distance = app.cam.distance;
                pane.cam.up = app.cam.up;
                pane.cam.set_view(ViewPreset::ALL[i].standard_view());
                pane.cam.fov = if i == 3 { 0.6 } else { 0.0 };
            }
            self.ever_started = true;
        }
        app.cam = self.panes[self.active].cam;
        self.initialized = true;
    }

    /// Restore the original single camera. The four saved cameras survive this
    /// toggle and the next launch, and single-view tools stay untouched.
    pub fn leave(&mut self, app: &mut SolveApp) {
        if !self.initialized {
            return;
        }
        self.panes[self.active].cam = app.cam;
        if let Some((camera, perspective)) = self.single.take() {
            app.cam = camera;
            app.ui.perspective = perspective;
        }
        app.viewport.background_image = None;
        app.viewport.orbit_locked = false;
        self.drag_owner = None;
        self.initialized = false;
    }

    /// Swaps the active pane only on an actual pointer press, never merely on
    /// hover across pane boundaries. The drag's original pane remains the owner.
    fn activate(&mut self, app: &mut SolveApp, next: usize) {
        if next == self.active || next >= PANES {
            return;
        }
        let previous = self.active;
        let old_cam = std::mem::replace(&mut app.cam, self.panes[next].cam);
        let old_anim = std::mem::replace(&mut app.cam_anim, self.panes[next].anim.take());
        let old_view = std::mem::replace(&mut app.viewport, std::mem::take(&mut self.panes[next].view));
        self.panes[previous].cam = old_cam;
        self.panes[previous].anim = old_anim;
        self.panes[previous].view = old_view;
        // One renderer, one geometry upload set: it follows the active pane.
        app.viewport.gpu = self.panes[previous].view.gpu.take();
        app.viewport.tess_ready = self.panes[previous].view.tess_ready.take();
        app.viewport.draw_stats = self.panes[previous].view.draw_stats.take();
        self.active = next;
        app.viewport.background_image = None;
    }

    /// Render a non-GPU pane only when its camera, model, selection, display
    /// settings or size changed. CPU rasterization is a reliable device-loss
    /// fallback, not a fourth copy of the model's GPU buffers.
    fn preview(&mut self, app: &SolveApp, ui: &Ui, i: usize, rect: Rect) -> Option<TextureHandle> {
        let ppp = ui.ctx().pixels_per_point().clamp(0.25, 8.0);
        let (rw, rh) = (rect.width() * ppp, rect.height() * ppp);
        if !rw.is_finite() || !rh.is_finite() || rw < 2.0 || rh < 2.0 {
            return None;
        }
        let factor = (MAX_PREVIEW_PX / rw.max(rh)).min(1.0);
        let (w, h) = ((rw * factor).round().max(2.0) as usize, (rh * factor).round().max(2.0) as usize);
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        app.session.revision.hash(&mut hash);
        (std::sync::Arc::as_ptr(&app.session.doc) as usize).hash(&mut hash);
        app.session.active_sketch.hash(&mut hash);
        format!("{:?}", app.session.selection).hash(&mut hash);
        format!("{:?}", crate::viewport::section_plane(app)).hash(&mut hash);
        app.ui.visual_style.hash(&mut hash);
        app.ui.show_grid.hash(&mut hash);
        app.origin_visible().hash(&mut hash);
        format!("{:?}", app.preferences.color_vision).hash(&mut hash);
        app.viewport_dark().hash(&mut hash);
        w.hash(&mut hash);
        h.hash(&mut hash);
        for v in [
            app.cam.target.x, app.cam.target.y, app.cam.target.z, app.cam.yaw, app.cam.pitch,
            app.cam.roll, app.cam.distance, app.cam.fov,
        ] {
            v.to_bits().hash(&mut hash);
        }
        let key = hash.finish();
        let pane = &mut self.panes[i];
        if pane.picture_key == Some(key) {
            return pane.picture.clone();
        }
        let scene = crate::control::view_scene(app, crate::control::window_guides(app));
        let image = solidcraft_engine::render::render(&scene, &app.cam, w, h);
        let expected = w.saturating_mul(h).saturating_mul(4);
        if image.rgba.len() != expected {
            pane.picture_key = None;
            return None;
        }
        let picture = ColorImage::from_rgba_unmultiplied([w, h], &image.rgba);
        let texture = ui.ctx().load_texture(format!("arclet-four-view-{i}"), picture, TextureOptions::LINEAR);
        pane.picture = Some(texture.clone());
        pane.picture_key = Some(key);
        Some(texture)
    }

    fn rects(&self, rect: Rect) -> [Rect; PANES] {
        if let Some(i) = self.maximized {
            let mut rects = [Rect::NOTHING; PANES];
            if i < PANES {
                rects[i] = rect;
            }
            rects
        } else {
            Layout::quad(rect)
        }
    }

    fn route_press(&mut self, app: &mut SolveApp, ui: &Ui, rects: &[Rect; PANES]) {
        let (origin, down) = ui.input(|input| (input.pointer.press_origin(), input.pointer.any_down()));
        if !down {
            self.drag_owner = None;
            return;
        }
        if self.drag_owner.is_none()
            && let Some(origin) = origin
            && let Some(i) = rects.iter().position(|rect| rect.is_positive() && rect.contains(origin))
        {
            self.drag_owner = Some(i);
            self.activate(app, i);
        }
    }

    fn show_pane(&mut self, app: &mut SolveApp, ui: &mut Ui, i: usize, rect: Rect) {
        let other = i != self.active;
        if other {
            std::mem::swap(&mut app.cam, &mut self.panes[i].cam);
            std::mem::swap(&mut app.cam_anim, &mut self.panes[i].anim);
            std::mem::swap(&mut app.viewport, &mut self.panes[i].view);
        }
        let perspective = std::mem::replace(&mut app.ui.perspective, i == 3);
        let auto = std::mem::replace(&mut app.ui.auto_ortho_faces, false);
        app.cam.fov = if i == 3 { 0.6 } else { 0.0 };
        app.viewport.orbit_locked = i != 3;
        app.viewport.background_image = if app.viewport.gpu.is_none() { self.preview(app, ui, i, rect) } else { None };
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt(("arclet_viewport", i)));
        viewport::show(app, &mut child);
        app.viewport.background_image = None;
        app.ui.perspective = perspective;
        app.ui.auto_ortho_faces = auto;
        if other {
            std::mem::swap(&mut app.cam, &mut self.panes[i].cam);
            std::mem::swap(&mut app.cam_anim, &mut self.panes[i].anim);
            std::mem::swap(&mut app.viewport, &mut self.panes[i].view);
        }
    }

    fn decorate(&mut self, app: &mut SolveApp, ui: &mut Ui, rects: &[Rect; PANES]) {
        let tokens = crate::theme::Tokens::get();
        for (i, rect) in rects.iter().copied().enumerate().filter(|(_, rect)| rect.is_positive()) {
            let highlight = i == self.active;
            let color = if highlight { tokens.sketch_accent } else { tokens.text_dim.gamma_multiply(0.65) };
            let stroke = Stroke::new(if highlight { 2.0 } else { 1.0 }, color);
            let painter = ui.painter_at(rect);
            for (a, b) in [(rect.left_top(), rect.right_top()), (rect.right_top(), rect.right_bottom()),
                (rect.right_bottom(), rect.left_bottom()), (rect.left_bottom(), rect.left_top())] {
                painter.line_segment([a, b], stroke);
            }
            let name = match i {
                0 => "Top",
                1 => "Front",
                2 => "Right",
                _ => "Perspective",
            };
            let text = if highlight { format!("{name}  •") } else { name.to_owned() };
            let left = Rect::from_min_size(rect.min + vec2(7.0, 7.0), vec2(105.0, 22.0));
            painter.rect_filled(left, 3.0, tokens.app_bar);
            painter.text(left.left_center() + vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, text, FontId::proportional(12.0), tokens.text);
            let button = Rect::from_min_size(pos2(rect.right() - 32.0, rect.top() + 6.0), vec2(25.0, 23.0));
            if ui.put(button, egui::Button::new(if self.maximized == Some(i) { "❐" } else { "□" })).on_hover_text(
                if self.maximized == Some(i) { "Restore four views" } else { "Maximize viewport" }
            ).clicked() {
                self.maximized = if self.maximized == Some(i) { None } else { Some(i) };
                self.activate(app, i);
                ui.ctx().request_repaint();
            }
        }
    }

    fn show(&mut self, app: &mut SolveApp, ui: &mut Ui) {
        self.enter(app);
        let rect = ui.max_rect();
        if !rect.is_positive() || rect.width() < 64.0 || rect.height() < 64.0 {
            viewport::show(app, ui);
            return;
        }
        let rects = self.rects(rect);
        self.route_press(app, ui, &rects);
        for (i, pane_rect) in rects.iter().copied().enumerate() {
            if pane_rect.is_positive() {
                self.show_pane(app, ui, i, pane_rect);
            }
        }
        self.decorate(app, ui, &rects);
    }
}

/// Keep the active pane in SolveApp while temporarily taking the pane controller
/// to avoid overlapping mutable borrows of the app and its view state.
pub fn show(app: &mut SolveApp, ui: &mut Ui) {
    let mut four = std::mem::take(&mut app.four_viewports);
    four.show(app, ui);
    app.four_viewports = four;
}

#[cfg(test)]
mod tests {
    use super::*;
    use solidcraft_engine::render::StandardView;

    fn app() -> SolveApp {
        SolveApp::new(Default::default(), Default::default())
    }

    #[test]
    fn cameras_are_independent_and_standard_views_remain_orthographic() {
        let mut app = app();
        let mut four = FourViewports { enabled: true, ..Default::default() };
        four.enter(&mut app);
        assert_eq!(four.active, 3);
        assert!(four.panes.iter().enumerate().all(|(i, p)| p.cam.is_valid() && (i == 3 || p.cam.fov == 0.0)));
        four.activate(&mut app, 0);
        assert_eq!((app.cam.yaw, app.cam.pitch), StandardView::Top.angles());
        let mut changed = app.cam;
        changed.target.x += 11.0;
        changed.distance *= 0.75;
        app.cam = changed;
        four.activate(&mut app, 1);
        assert_eq!((app.cam.yaw, app.cam.pitch), StandardView::Front.angles());
        assert_ne!(four.panes[0].cam.target.x, app.cam.target.x);
        assert_eq!(four.panes[0].cam, changed);
        four.activate(&mut app, 2);
        assert_eq!((app.cam.yaw, app.cam.pitch), StandardView::Right.angles());
        four.activate(&mut app, 3);
        assert_eq!((app.cam.yaw, app.cam.pitch), StandardView::Iso.angles());
        four.leave(&mut app);
        assert!(!four.initialized);
        assert!(app.cam.is_valid());
    }

    #[test]
    fn quad_tiling_and_maximizing_restore_without_modifying_document() {
        let mut four = FourViewports::default();
        let full = Rect::from_min_size(Pos2::ZERO, vec2(900.0, 600.0));
        let panes = four.rects(full);
        assert_eq!(panes[0].size(), vec2(450.0, 300.0));
        assert!(panes.iter().all(|r| r.is_positive()));
        four.maximized = Some(2);
        let solo = four.rects(full);
        assert_eq!(solo[2], full);
        assert!(solo.iter().enumerate().all(|(i,r)| i == 2 || !r.is_positive()));
        four.maximized = None;
        assert_eq!(four.rects(full), panes);
    }

    #[test]
    fn pane_preferences_reject_invalid_cameras_and_indices() {
        let mut state = FourViewports::default();
        state.load_prefs(r#"{"enabled":true,"active":400,"maximized":401,"cameras":[null,null,null,null]}"#);
        assert!(state.enabled);
        assert_eq!(state.active, 3);
        assert_eq!(state.maximized, None);
        let original = state.panes[0].cam;
        state.load_prefs(r#"{"cameras":[{"distance":-1},null,null,null]}"#);
        assert_eq!(state.panes[0].cam, original);
        let encoded = state.prefs(Camera::default());
        let mut loaded = FourViewports::default();
        loaded.load_prefs(&encoded);
        assert_eq!(loaded.active, state.active);
        assert_eq!(loaded.enabled, state.enabled);
    }

    #[test]
    fn gpu_initialization_missing_does_not_create_a_second_renderer() {
        let mut app = app();
        assert!(app.viewport.gpu.is_none());
        let mut state = FourViewports::default();
        state.enter(&mut app);
        state.activate(&mut app, 0);
        assert!(app.viewport.gpu.is_none());
        state.activate(&mut app, 3);
        assert!(app.viewport.gpu.is_none());
    }
}
