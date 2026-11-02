//! Viewport scale handles for the existing solid.scale feature.
//!
//! A drag only edits the open dialog (and its normal live preview). OK runs
//! the existing solid.scale command as one undo step. No transient feature
//! enters the document while dragging.
//!
//! The first press fixes the camera projection, original factors and pivot.
//! Subsequent frames use that snapshot rather than incrementally applying
//! pointer movement to already changed geometry. This avoids release jumps.

use egui::{Color32, Pos2, Rect, Sense, Stroke, vec2};
use serde_json::{Value, json};
use solidcraft_engine::Sel;
use solidcraft_engine::doc::expr::Kind as ValueKind;
use solidcraft_engine::geom::Vec3;

use crate::SolveApp;
use crate::dialogs::{Dialog, Kind};
use crate::viewport::Proj;

const AXES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];
const SCREEN_LENGTH: f64 = 105.0;
const MIN_FACTOR: f64 = 0.001;
const MAX_FACTOR: f64 = 1000.0;
const HANDLE_PIXELS: f32 = 11.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handle {
    Axis(usize),
    Uniform,
}

#[derive(Clone)]
struct DragState {
    handle: Handle,
    projection: Proj,
    pivot: Vec3,
    start_pointer: Pos2,
    // Restoring the dialog on Escape must undo only the ongoing drag, not
    // edits the user already made in the scale dialog.
    original_factor: String,
    original_factors: Option<Value>,
    original_origin: Option<Value>,
    factors: [f64; 3],
    last_ratio: f64,
}

fn from_origin(d: &Dialog) -> Option<Vec3> {
    let [x, y, z]: [f64; 3] = serde_json::from_value(d.extra.get("origin")?.clone()).ok()?;
    let v = Vec3::new(x, y, z);
    (v.is_finite() && v.x.abs().max(v.y.abs()).max(v.z.abs()) <= 1e12).then_some(v)
}

/// The bounded set of bodies selected by this dialog (never every body).
/// Edits use the model state before the edited feature as the source.
pub(crate) fn selected_centre(app: &SolveApp, d: &Dialog) -> Option<Vec3> {
    let state = match d.editing {
        Some((id, _)) => app.session.model.state_before(id),
        None => app.session.model.state(),
    };
    let mut lo: Option<Vec3> = None;
    let mut hi: Option<Vec3> = None;
    for name in d.inputs.first()?.items.iter().filter_map(|s| match s {
        Sel::Body { name } => Some(name),
        _ => None,
    }) {
        let bounds = state.body(name)?.mesh().bounds();
        if bounds.is_empty() || !bounds.min.is_finite() || !bounds.max.is_finite() {
            return None;
        }
        lo = Some(lo.map_or(bounds.min, |v| v.min(bounds.min)));
        hi = Some(hi.map_or(bounds.max, |v| v.max(bounds.max)));
    }
    Some((lo? + hi?) * 0.5)
}

fn centre(app: &SolveApp, d: &Dialog) -> Option<Vec3> {
    from_origin(d).or_else(|| selected_centre(app, d))
}

fn current_factors(app: &SolveApp, d: &Dialog) -> Option<[f64; 3]> {
    let Kind::Scale { factor } = &d.kind else { return None };
    let base = app.session.doc.eval(factor, ValueKind::Unitless).ok()?;
    if !base.is_finite() || !(MIN_FACTOR..=MAX_FACTOR).contains(&base) {
        return None;
    }
    let factors = match d.extra.get("factors") {
        Some(v) => serde_json::from_value::<[f64; 3]>(v.clone()).ok()?,
        None => [base; 3],
    };
    factors.iter().all(|v| v.is_finite() && (MIN_FACTOR..=MAX_FACTOR).contains(v)).then_some(factors)
}

fn axis_screens(proj: &Proj, pivot: Vec3) -> Option<(Pos2, [(Pos2, Pos2); 3])> {
    let c = proj.to_screen(pivot)?;
    let l = crate::gizmo::ring_radius(proj, pivot)? / 0.62;
    if !(1e-9..=1e12).contains(&l) {
        return None;
    }
    let mut lines = [(c, c); 3];
    for (i, axis) in AXES.iter().enumerate() {
        let a = proj.to_screen(pivot + *axis * (l * 0.18))?;
        let b = proj.to_screen(pivot + *axis * (l * 0.95))?;
        lines[i] = (a, b);
    }
    Some((c, lines))
}

fn eligible_axis(a: Pos2, b: Pos2) -> bool {
    a.is_finite() && b.is_finite() && a.distance(b) >= 12.0
}

fn hit(proj: &Proj, pivot: Vec3, pointer: Pos2) -> Option<Handle> {
    let (c, lines) = axis_screens(proj, pivot)?;
    if c.distance(pointer) <= HANDLE_PIXELS {
        return Some(Handle::Uniform);
    }
    lines.iter().enumerate().find_map(|(i, &(a, b))| {
        (eligible_axis(a, b) && Rect::from_center_size(b, vec2(HANDLE_PIXELS * 2.3, HANDLE_PIXELS * 2.3)).contains(pointer))
            .then_some(Handle::Axis(i))
    })
}

/// Before dynamic widgets are registered, navigation must yield to a press
/// on a scale handle. Uses the same view projection passed to show().
pub(crate) fn contains_pointer(app: &SolveApp, proj: &Proj, pointer: Pos2) -> bool {
    app.dialog.as_ref().filter(|d| matches!(d.kind, Kind::Scale { .. }))
        .and_then(|d| centre(app, d))
        .is_some_and(|pivot| hit(proj, pivot, pointer).is_some())
}

/// Smooth, positive ratio: moving one projected handle length scales by 2×.
/// Without free movement, precision snaps to 5% increments. Never allow zero
/// or negative scale; those cannot be represented by this solid.scale command.
fn ratio_from_motion(distance: f64, free: bool) -> Option<f64> {
    if !distance.is_finite() {
        return None;
    }
    let raw = 2f64.powf(distance.clamp(-20.0, 20.0)).clamp(MIN_FACTOR, MAX_FACTOR);
    let ratio = if free { raw } else { (raw * 20.0).round() / 20.0 };
    Some(ratio.clamp(MIN_FACTOR, MAX_FACTOR))
}

fn drag_ratio(state: &DragState, pointer: Pos2, free: bool) -> Option<f64> {
    if !pointer.is_finite() {
        return None;
    }
    let delta = pointer - state.start_pointer;
    let units = match state.handle {
        Handle::Uniform => f64::from(delta.x) / SCREEN_LENGTH,
        Handle::Axis(i) => {
            let (c, lines) = axis_screens(&state.projection, state.pivot)?;
            let (_, b) = *lines.get(i)?;
            let direction = b - c;
            let length = direction.length_sq();
            if length < 144.0 { return None; }
            f64::from(delta.dot(direction) / length)
        }
    };
    ratio_from_motion(units, free)
}

fn set_factors(d: &mut Dialog, state: &DragState, ratio: f64) {
    let mut factors = state.factors;
    match state.handle {
        Handle::Uniform => {
            for factor in &mut factors {
                *factor = (*factor * ratio).clamp(MIN_FACTOR, MAX_FACTOR);
            }
        }
        Handle::Axis(i) => {
            if let Some(v) = factors.get_mut(i) {
                *v = (*v * ratio).clamp(MIN_FACTOR, MAX_FACTOR);
            }
        }
    }
    // A uniform scale of a uniform feature can keep the feature's parameter
    // a single number; other cases must use the real nonuniform factor array.
    let uniform = factors.iter().all(|v| (v - factors[0]).abs() <= 1e-9 * factors[0].max(1.0));
    if uniform {
        if let Kind::Scale { factor } = &mut d.kind {
            *factor = format!("{:.8}", factors[0]);
        }
        d.extra.remove("factors");
    } else {
        // The factor field stays valid, but per-axis factors take precedence.
        d.extra.insert("factors".into(), json!(factors));
    }
    d.extra.insert("origin".into(), json!([state.pivot.x, state.pivot.y, state.pivot.z]));
    d.error = None;
}

fn restore(d: &mut Dialog, state: &DragState) {
    if let Kind::Scale { factor } = &mut d.kind {
        *factor = state.original_factor.clone();
    }
    match &state.original_factors {
        Some(value) => { d.extra.insert("factors".into(), value.clone()); }
        None => { d.extra.remove("factors"); }
    }
    match &state.original_origin {
        Some(value) => { d.extra.insert("origin".into(), value.clone()); }
        None => { d.extra.remove("origin"); }
    }
}

fn paint(painter: &egui::Painter, proj: &Proj, pivot: Vec3, active: Option<Handle>) {
    let Some((c, lines)) = axis_screens(proj, pivot) else { return };
    let theme = crate::theme::Tokens::get();
    for (i, (a, b)) in lines.into_iter().enumerate() {
        if !eligible_axis(a, b) {
            continue;
        }
        let picked = active == Some(Handle::Axis(i));
        let color = if picked { theme.accent } else { theme.axes()[i] };
        painter.line_segment([a, b], Stroke::new(if picked { 3.2 } else { 2.1 }, color));
        painter.rect_filled(Rect::from_center_size(b, vec2(HANDLE_PIXELS, HANDLE_PIXELS)), 1.5, color);
        crate::scenario::publish_handle(&format!("scale_{}", ["x", "y", "z"][i]), b);
    }
    let color = if active == Some(Handle::Uniform) { theme.accent } else { Color32::from_gray(225) };
    painter.rect_filled(Rect::from_center_size(c, vec2(HANDLE_PIXELS + 2.0, HANDLE_PIXELS + 2.0)), 2.0, color);
    crate::scenario::publish_handle("scale_uniform", c);
}

/// Show three axis squares and the uniform centre. Every frame during a drag
/// uses the original parameter snapshot. Esc restores it, release commits
/// only *dialog* values; the existing OK transaction handles model + Undo.
pub(crate) fn show(app: &SolveApp, d: &mut Dialog, ui: &mut egui::Ui, painter: &egui::Painter, proj: &Proj) -> bool {
    let id = egui::Id::new("sc_scale_triad");
    let old: Option<DragState> = ui.data(|data| data.get_temp(id));
    let Some(pivot) = old.as_ref().map(|state| state.pivot).or_else(|| centre(app, d)) else { return false };
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let pressed = ui.input(|input| input.pointer.primary_down());
    let start = ui.input(|input| input.pointer.press_origin());
    let candidate = if pressed { start.or(pointer) } else { pointer };
    let hovered = if old.is_none() { candidate.and_then(|point| hit(proj, pivot, point)) } else { None };
    let active = old.as_ref().map(|state| state.handle).or(hovered);
    paint(painter, proj, pivot, active);
    if old.is_none() && hovered.is_none() {
        return false;
    }
    let Some(at) = pointer.or_else(|| old.as_ref().map(|state| state.start_pointer)) else { return false };
    let response = ui.interact(Rect::from_center_size(at, vec2(28.0, 28.0)), id.with("hit"), Sense::drag());
    if response.hovered() || old.is_some() {
        ui.ctx().set_cursor_icon(if pressed { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
    }
    if old.is_some() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        if let Some(state) = old.as_ref() { restore(d, state); }
        ui.data_mut(|data| data.remove::<DragState>(id));
        return true;
    }
    let mut state = match old {
        Some(state) => state,
        None => {
            if !response.drag_started() {
                return false;
            }
            let Some(handle) = hovered else { return false };
            let Some(factors) = current_factors(app, d) else { return false };
            let Kind::Scale { factor } = &d.kind else { return false };
            DragState {
                handle,
                projection: *proj,
                pivot,
                start_pointer: start.unwrap_or(at),
                original_factor: factor.clone(),
                original_factors: d.extra.get("factors").cloned(),
                original_origin: d.extra.get("origin").cloned(),
                factors,
                last_ratio: 1.0,
            }
        }
    };
    crate::tool_modifiers::set_manipulator_dragging(true);
    let done = response.drag_stopped() || !pressed;
    let release = if done {
        ui.input(|input| input.events.iter().rev().find_map(|event| match event {
            egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, .. } => Some(*pos),
            _ => None,
        })).unwrap_or(at)
    } else { at };
    let free = crate::tool_modifiers::manipulator_free();
    if let Some(ratio) = drag_ratio(&state, release, free) {
        state.last_ratio = ratio;
    }
    // When a ray is temporarily undefined, keep the last valid preview,
    // including the final release frame, rather than snapping to 1:1.
    set_factors(d, &state, state.last_ratio);
    if done {
        ui.data_mut(|data| data.remove::<DragState>(id));
    } else {
        ui.data_mut(|data| data.insert_temp(id, state));
        ui.ctx().request_repaint();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_is_positive_bounded_and_is_from_initial_pointer() {
        assert_eq!(ratio_from_motion(0.0, false), Some(1.0));
        assert_eq!(ratio_from_motion(1.0, false), Some(2.0));
        assert_eq!(ratio_from_motion(-1.0, false), Some(0.5));
        assert_eq!(ratio_from_motion(f64::INFINITY, false), None);
        assert!(ratio_from_motion(-10000.0, true).unwrap() >= MIN_FACTOR);
        assert!(ratio_from_motion(10000.0, true).unwrap() <= MAX_FACTOR);
    }
}
