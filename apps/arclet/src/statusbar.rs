//! The thin status bar under the command bar: working plane, cursor readout, units, component,
//! selection count and the object-snap chips. A chip is live only when a real document setting
//! backs it. Every other chip is drawn disabled with a "not available yet" tooltip.
//!
//! The state rules are plain Rust with no egui types, so the tests check them directly.

use egui::{RichText, Ui};
use serde_json::{Value, json};
use solidcraft_engine::geom::Vec3;
use solidcraft_ui_egui::SolveApp;
use solidcraft_ui_egui::i18n::tr;
use solidcraft_ui_egui::theme::Tokens;
use solidcraft_ui_egui::ui_kit::{self, Button, Container, Edge, Surface};

use crate::classic_look::chrome::{self, Density};
use solidcraft_ui_egui::viewport::Hit;

/// Shown for any value the session does not have yet.
const EMPTY: &str = "—";

/// A toggle on the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    End,
    Mid,
    Cen,
    Int,
    Perp,
    Tan,
    Quad,
    Near,
    Point,
    Knot,
    GridSnap,
    Ortho,
    Planar,
    Gumball,
}

impl Chip {
    /// The object-snap row, in the order the bar shows it.
    pub const OBJECT_SNAPS: [Chip; 10] =
        [Chip::End, Chip::Mid, Chip::Cen, Chip::Int, Chip::Perp, Chip::Tan, Chip::Quad, Chip::Near, Chip::Point, Chip::Knot];
    /// The mode row after the object snaps.
    pub const MODES: [Chip; 4] = [Chip::GridSnap, Chip::Ortho, Chip::Planar, Chip::Gumball];

    fn label_id(self) -> &'static str {
        match self {
            Chip::End => "arclet.status.snap.end",
            Chip::Mid => "arclet.status.snap.mid",
            Chip::Cen => "arclet.status.snap.cen",
            Chip::Int => "arclet.status.snap.int",
            Chip::Perp => "arclet.status.snap.perp",
            Chip::Tan => "arclet.status.snap.tan",
            Chip::Quad => "arclet.status.snap.quad",
            Chip::Near => "arclet.status.snap.near",
            Chip::Point => "arclet.status.snap.point",
            Chip::Knot => "arclet.status.snap.knot",
            Chip::GridSnap => "arclet.status.grid_snap",
            Chip::Ortho => "arclet.status.ortho",
            Chip::Planar => "arclet.status.planar",
            Chip::Gumball => "arclet.status.gumball",
        }
    }
}

/// A real document setting that a chip reads and writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backing {
    /// The active sketch's Snap option. It controls the grid rounding of a sketch point: off, the
    /// cursor point is used as it is.
    SketchGridSnap,
}

/// The setting behind a chip, if any. Only Grid Snap has one today.
pub fn backing(chip: Chip) -> Option<Backing> {
    match chip {
        Chip::GridSnap => Some(Backing::SketchGridSnap),
        _ => None,
    }
}

/// The settings the chips read, as the session has them this frame. `sketch_grid_snap` is `None`
/// when no sketch is open, since the Snap option belongs to a sketch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub sketch_grid_snap: Option<bool>,
}

/// How a chip is drawn: enabled, and whether it is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChipState {
    pub enabled: bool,
    pub on: bool,
}

pub fn chip_state(chip: Chip, settings: Settings) -> ChipState {
    match (backing(chip), settings.sketch_grid_snap) {
        (Some(Backing::SketchGridSnap), Some(on)) => ChipState { enabled: true, on },
        _ => ChipState { enabled: false, on: false },
    }
}

/// The command and parameters a click on a chip sends. `None` for a chip that is disabled.
pub fn click(chip: Chip, settings: Settings) -> Option<(&'static str, Value)> {
    let state = chip_state(chip, settings);
    if !state.enabled {
        return None;
    }
    match backing(chip)? {
        Backing::SketchGridSnap => Some(("sketch.options", json!({"snap": !state.on}))),
    }
}

/// The tooltip message id for a chip: its name when it works, the reason when it does not.
fn tooltip_id(chip: Chip, state: ChipState) -> &'static str {
    if state.enabled {
        chip.label_id()
    } else if backing(chip).is_some() {
        "arclet.status.open_sketch"
    } else {
        "arclet.status.not_available"
    }
}

/// One coordinate for the readout: three decimals, or the dash when there is no value.
pub fn coordinate(value: Option<f64>) -> String {
    match value {
        Some(v) if v.is_finite() => format!("{v:.3}"),
        _ => EMPTY.to_owned(),
    }
}

/// The world point a viewport hit stands for. Sketch hits are in sketch coordinates and have no
/// world point here, so they show the dash.
pub fn cursor_of(hit: Option<&Hit>) -> Option<[f64; 3]> {
    let point = match hit? {
        Hit::Vertex { point, .. } | Hit::Face { point, .. } | Hit::Plane { point, .. } => *point,
        Hit::Edge { mid, .. } => *mid,
        _ => return None,
    };
    let Vec3 { x, y, z } = point;
    Some([x, y, z])
}

/// Everything the bar displays, gathered from the session each frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Readout {
    /// The open sketch's feature name, which is the working plane while sketching.
    pub plane: Option<String>,
    pub cursor: Option<[f64; 3]>,
    pub units: String,
    /// The component new geometry goes into.
    pub component: Option<String>,
    pub selected: usize,
}

fn readout(app: &SolveApp) -> Readout {
    let s = &app.session;
    let plane = s.active_sketch.and_then(|id| s.doc.feature(id)).map(|f| f.name.clone());
    let component = if s.active_component == 0 {
        Some(tr("arclet.status.root").to_owned())
    } else {
        s.doc.components.iter().find(|c| c.id == s.active_component).map(|c| c.name.clone())
    };
    Readout { plane, cursor: cursor_of(app.viewport.hover.as_ref()), units: s.doc.units.clone(), component, selected: s.selection.len() }
}

/// Keep one compact strip on small laptops instead of letting disabled future
/// features wrap the status bar into several rows and steal model space.
fn visible_chips(density: Density) -> impl Iterator<Item = Chip> {
    Chip::OBJECT_SNAPS
        .into_iter()
        .chain(Chip::MODES)
        .filter(move |chip| density == Density::Full || backing(*chip).is_some())
}

fn settings(app: &SolveApp) -> Settings {
    let sketch_grid_snap = app.session.active_sketch.map(|_| !solidcraft_ui_egui::viewport::active_view(app).no_snap);
    Settings { sketch_grid_snap }
}

/// Draws the bar. It sits under the command bar, so it is added before the command bar.
pub fn show(app: &mut SolveApp, ui: &mut Ui) {
    if app.nodes.open {
        return;
    }
    let tokens = Tokens::get();
    let read = readout(app);
    let settings = settings(app);
    let mut clicked = None;
    let density = chrome::status_density(ui.ctx().viewport_rect().width());
    Container::new(ui.ctx(), Surface::ApplicationBar)
        .edge("arclet_status", Edge::Bottom)
        .frame(Container::new(ui.ctx(), Surface::ApplicationBar).edge_frame(tokens.panel).inner_margin(egui::Margin::symmetric(12, 4)))
        .show(ui, |ui| {
            // One line, even at 200% scale. An overflow scroll never moves the
            // viewport vertically when the cursor or selection readout changes.
            egui::ScrollArea::horizontal()
                .id_salt("arclet_status_overflow")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = chrome::SECTION_GAP;
                        let dim = |text: String| RichText::new(text).size(tokens.font_body).color(tokens.text_dim);
                        let cursor = read.cursor;
                        let axis = |name: &str, index: usize| format!("{name} {}", coordinate(cursor.map(|c| c[index])));
                        if density == Density::Full || read.plane.is_some() {
                            ui.label(dim(format!("{}: {}", tr("arclet.status.plane"), read.plane.as_deref().unwrap_or(EMPTY))));
                            ui.separator();
                        }
                        ui.label(dim(format!("{}  {}  {}", axis("X", 0), axis("Y", 1), axis("Z", 2))).monospace());
                        ui.separator();
                        ui.label(dim(format!("{}: {}", tr("arclet.status.units"), read.units)));
                        if density == Density::Full {
                            ui.separator();
                            ui.label(dim(format!("{}: {}", tr("arclet.status.component"), read.component.as_deref().unwrap_or(EMPTY))));
                        }
                        ui.separator();
                        ui.label(dim(format!("{}: {}", tr("arclet.status.selected"), read.selected)));
                        ui.separator();
                        for chip in visible_chips(density) {
                            let state = chip_state(chip, settings);
                            let response = ui
                                .add_enabled_ui(state.enabled, |ui| {
                                    ui_kit::button(
                                        ui,
                                        RichText::new(tr(chip.label_id())).size(tokens.font_body),
                                        if state.on { Button::SmallSelected } else { Button::Small },
                                    )
                                })
                                .inner
                                .on_hover_text(tr(tooltip_id(chip, state)));
                            if response.clicked() {
                                clicked = Some(chip);
                            }
                        }
                    });
                });
        });
    if let Some((command, params)) = clicked.and_then(|chip| click(chip, settings))
        && let Err(error) = app.run(command, params)
    {
        app.set_status(error, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_grid_snap_is_wired_and_only_while_a_sketch_is_open() {
        let closed = Settings { sketch_grid_snap: None };
        let open = Settings { sketch_grid_snap: Some(true) };
        for chip in Chip::OBJECT_SNAPS.into_iter().chain(Chip::MODES) {
            assert!(!chip_state(chip, closed).enabled, "{chip:?} is disabled with no sketch");
            if chip == Chip::GridSnap {
                assert_eq!(chip_state(chip, open), ChipState { enabled: true, on: true });
                assert_eq!(click(chip, open), Some(("sketch.options", json!({"snap": false}))));
                assert_eq!(click(chip, closed), None);
            } else {
                assert!(!chip_state(chip, open).enabled, "{chip:?} has no real setting, so it stays disabled");
                assert_eq!(click(chip, open), None, "{chip:?} sends nothing");
                assert_eq!(tooltip_id(chip, chip_state(chip, open)), "arclet.status.not_available");
            }
        }
    }

    #[test]
    fn compact_status_never_advertises_unsupported_snap_modes() {
        let compact: Vec<_> = visible_chips(Density::Compact).collect();
        assert_eq!(compact, [Chip::GridSnap]);
        let full: Vec<_> = visible_chips(Density::Full).collect();
        assert_eq!(full.len(), Chip::OBJECT_SNAPS.len() + Chip::MODES.len());
        for chip in compact {
            assert_eq!(backing(chip), Some(Backing::SketchGridSnap));
        }
    }

    #[test]
    fn readout_shows_a_dash_until_there_is_a_value() {
        assert_eq!(coordinate(None), "—");
        assert_eq!(coordinate(Some(f64::NAN)), "—");
        assert_eq!(coordinate(Some(1.23456)), "1.235");
        assert_eq!(cursor_of(None), None);
        let vertex = Hit::Vertex { body: "Body".into(), point: Vec3 { x: 1.0, y: 2.0, z: 3.0 } };
        assert_eq!(cursor_of(Some(&vertex)), Some([1.0, 2.0, 3.0]));
    }
}
