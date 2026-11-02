//! Arclet's right-docked panel with three tabs: Properties, Groups and Display.
//!
//! The panel is one side panel from the shared [`Container`], drawn beside the header and the
//! command bar. Each action runs an engine command through [`SolveApp::run`]. Measurements use the
//! read-only `inspect.measure` command and are kept until the selection or the design changes.
//!
//! The Groups tab is the design's browser groups (`browser.group`, `browser.rename_group`,
//! `browser.ungroup`), and its eye toggles the group through `browser.visibility`. The Layers tab
//! lists the design's layers (`layer.*` commands): each with an eye, a lock, a colour swatch and
//! a current-layer marker, plus New, Rename, Move selection here and Set current.

use std::sync::Arc;

use egui::{RichText, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};
use solidcraft_engine::doc::layers::DEFAULT;
use solidcraft_engine::{Sel, Session};
use solidcraft_ui_egui::i18n::{format as message, tr};
use solidcraft_ui_egui::theme::Tokens;
use solidcraft_ui_egui::ui_kit::{self, Button, Container, Edge, Surface};
use solidcraft_ui_egui::{SolveApp, icons, prefs};

const OPEN_WIDTH: f32 = 300.0;
const COLLAPSED_WIDTH: f32 = 28.0;

/// Visual styles in the order `view.visual_style` numbers them (its `index`).
const STYLES: [(&str, &str); 4] = [
    ("shaded_with_edges", "arclet.panel.style_edges"),
    ("shaded", "arclet.panel.style_shaded"),
    ("wireframe", "arclet.panel.style_wire"),
    ("shaded_with_hidden_edges", "arclet.panel.style_hidden"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Properties,
    Groups,
    Layers,
    Display,
}

/// The panel's own state: what is open, which tab, and what is being typed. The design and the
/// viewport settings stay in the engine and in `SolveApp`.
pub struct PanelState {
    open: bool,
    tab: Tab,
    /// The group that Rename and Ungroup act on.
    chosen_group: Option<u64>,
    /// A rename in progress: the group and the name typed so far.
    rename: Option<(u64, String)>,
    /// The layer that Rename, Move selection here and Set current act on (`None`: the default).
    chosen_layer: Option<u64>,
    /// A layer rename in progress: the layer and the name typed so far.
    layer_rename: Option<(u64, String)>,
    error: Option<String>,
    measured: Option<Measured>,
}

/// A measurement of the selected bodies, valid for one design and one body list.
struct Measured {
    design: usize,
    bodies: Vec<String>,
    result: Result<Value, String>,
}

/// A browser group as the panel needs it.
struct GroupRow {
    id: u64,
    name: String,
    component: u64,
    items: Vec<String>,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            open: true,
            tab: Tab::Properties,
            chosen_group: None,
            rename: None,
            chosen_layer: None,
            layer_rename: None,
            error: None,
            measured: None,
        }
    }
}

/// Draw the right panel. Hidden while the node workspace is open, as the other chrome is.
pub fn show(ui: &mut Ui, cad: &mut SolveApp, state: &mut PanelState) {
    if cad.nodes.open {
        return;
    }
    let tokens = Tokens::get();
    let width = if state.open { OPEN_WIDTH } else { COLLAPSED_WIDTH };
    Container::new(ui.ctx(), Surface::Panel)
        .edge("arclet_panels", Edge::Right)
        .exact_size(width)
        .frame(Container::new(ui.ctx(), Surface::Panel).edge_frame(tokens.panel).inner_margin(egui::Margin::symmetric(10, 8)))
        .show(ui, |ui| state.body(cad, ui));
}

impl PanelState {
    fn body(&mut self, cad: &mut SolveApp, ui: &mut Ui) {
        if !self.open {
            // The thin edge button: the only thing drawn while the panel is folded away.
            ui.vertical_centered(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(Tokens::get().control_height, Tokens::get().control_height), Sense::hover());
                if ui_kit::panel_disclosure_button_at(ui, rect, false, tr("arclet.panel.expand")).clicked() {
                    self.open = true;
                }
            });
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in [
                (Tab::Properties, "arclet.panel.tab.properties"),
                (Tab::Groups, "arclet.panel.tab.groups"),
                (Tab::Layers, "arclet.panel.tab.layers"),
                (Tab::Display, "arclet.panel.tab.display"),
            ] {
                let style = if self.tab == tab { Button::Primary } else { Button::Ghost };
                if ui_kit::button(ui, tr(label), style).clicked() {
                    self.tab = tab;
                }
            }
        });
        ui.separator();
        if let Some(error) = &self.error {
            ui.colored_label(Tokens::get().error, error);
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.tab {
            Tab::Properties => self.properties(cad, ui),
            Tab::Groups => self.groups(cad, ui),
            Tab::Layers => self.layers(cad, ui),
            Tab::Display => self.display(cad, ui),
        });
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(Tokens::get().control_height, Tokens::get().control_height), Sense::hover());
            if ui_kit::panel_disclosure_button_at(ui, rect, true, tr("arclet.panel.collapse")).clicked() {
                self.open = false;
            }
        });
    }

    /// Run a command through the shared host. A refusal is kept for the panel to show.
    fn exec(&mut self, cad: &mut SolveApp, id: &str, params: Value) -> Option<Value> {
        match cad.run(id, params) {
            Ok(value) => {
                self.error = None;
                Some(value)
            }
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }

    fn properties(&mut self, cad: &mut SolveApp, ui: &mut Ui) {
        let tokens = Tokens::get();
        let count = cad.session.selection.len();
        let Some(first) = cad.session.selection.first() else {
            ui.label(RichText::new(tr("arclet.panel.none")).color(tokens.text_dim));
            return;
        };
        let (kind, name) = describe(&cad.session, first);
        let bodies = selected_bodies(&cad.session.selection);
        rows(ui, "arclet_panel_props", &[("arclet.panel.name", name)]);
        ui.label(RichText::new(kind_label(kind)).color(tokens.text_dim));
        ui.label(RichText::new(message("arclet.panel.selected_count", &[("count", &count.to_string())])).color(tokens.text_dim));
        if !bodies.is_empty() {
            self.refresh_measure(cad, &bodies);
            if let Some(measured) = &self.measured {
                match &measured.result {
                    Ok(value) => measurement_rows(ui, cad, value),
                    Err(error) => {
                        ui.colored_label(tokens.error, error);
                    }
                }
            }
        }
    }

    /// Measure the selected bodies again only when the body list or the design changed.
    fn refresh_measure(&mut self, cad: &mut SolveApp, bodies: &[String]) {
        let design = Arc::as_ptr(&cad.session.doc) as usize;
        if self.measured.as_ref().is_some_and(|m| m.design == design && m.bodies == bodies) {
            return;
        }
        let result = cad.session.execute("inspect.measure", &json!({ "bodies": bodies })).map_err(|e| e.to_string());
        self.measured = Some(Measured { design, bodies: bodies.to_vec(), result });
    }

    fn groups(&mut self, cad: &mut SolveApp, ui: &mut Ui) {
        let tokens = Tokens::get();
        let groups = group_rows(&cad.session);
        let bodies = selected_bodies(&cad.session.selection);
        let target = group_target(&groups, self.chosen_group, &bodies);

        ui.horizontal_wrapped(|ui| {
            if ui_kit::button(ui, tr("arclet.panel.group_new"), if bodies.is_empty() { ui_kit::Button::Enabled(false) } else { Button::Primary })
                .clicked()
                && let Some(made) = self.exec(cad, "browser.group", json!({ "folder": "bodies", "component": 0, "items": bodies }))
            {
                self.chosen_group = made.get("group").and_then(Value::as_u64);
            }
            if ui_kit::button(ui, tr("arclet.panel.group_rename"), ui_kit::Button::Enabled(target.is_some())).clicked()
                && let Some(id) = target
                && let Some(group) = groups.iter().find(|g| g.id == id)
            {
                self.rename = Some((id, group.name.clone()));
            }
            if ui_kit::button(ui, tr("arclet.panel.group_ungroup"), ui_kit::Button::Enabled(target.is_some())).clicked()
                && let Some(id) = target
                && self.exec(cad, "browser.ungroup", json!({ "group": id })).is_some()
            {
                self.chosen_group = None;
            }
        });

        if let Some((id, mut text)) = self.rename.take() {
            let mut keep = true;
            ui.horizontal(|ui| {
                let input_id = ui.make_persistent_id(("arclet-group-rename", id));
                ui.allocate_ui(vec2(150.0, Tokens::get().control_height), |ui| ui_kit::cell_editor(ui, input_id, &mut text, "", false));
                if ui_kit::button(ui, tr("arclet.panel.group_save"), Button::Primary).clicked() {
                    let name = text.trim().to_string();
                    // A refused name keeps the editor open with the reason shown above.
                    keep = name.is_empty() || self.exec(cad, "browser.rename_group", json!({ "group": id, "name": name })).is_none();
                }
                if ui_kit::button(ui, tr("arclet.panel.group_cancel"), Button::Normal).clicked() {
                    keep = false;
                }
            });
            if keep {
                self.rename = Some((id, text));
            }
        }

        ui.add_space(6.0);
        if groups.is_empty() {
            ui.label(RichText::new(tr("arclet.panel.group_empty")).color(tokens.text_dim));
        }
        for group in &groups {
            let key = format!("c{}/g{}", group.component, group.id);
            let hidden = cad.session.folder_hidden(&key);
            ui.horizontal(|ui| {
                let (rect, response) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
                icons::paint_icon(ui.painter(), rect.shrink(3.0), "eye", if hidden { 0.35 } else { 1.0 });
                if hidden {
                    ui.painter().line_segment(
                        [pos2(rect.left() + 5.0, rect.bottom() - 5.0), pos2(rect.right() - 5.0, rect.top() + 5.0)],
                        Stroke::new(1.5, tokens.text_dim),
                    );
                }
                let toggle = if hidden { tr("arclet.panel.group_show") } else { tr("arclet.panel.group_hide") };
                if response.on_hover_text(toggle).clicked() {
                    self.exec(cad, "browser.visibility", json!({ "visible": hidden, "tree_folders": [key] }));
                }
                let label = format!("{} ({})", group.name, message("arclet.panel.group_items", &[("count", &group.items.len().to_string())]));
                if ui_kit::button(ui, label, if target == Some(group.id) { Button::SmallSelected } else { Button::Ghost }).clicked() {
                    self.chosen_group = Some(group.id);
                }
            });
        }
    }

    fn layers(&mut self, cad: &mut SolveApp, ui: &mut Ui) {
        let tokens = Tokens::get();
        let rows = layer_rows(&cad.session);
        let current = cad.session.doc.layers.current;
        let bodies = selected_bodies(&cad.session.selection);
        let target = self.chosen_layer.filter(|id| rows.iter().any(|row| row.id == *id)).unwrap_or(DEFAULT);

        ui.label(RichText::new(tr("arclet.layers.hint")).color(tokens.text_dim));
        ui.horizontal_wrapped(|ui| {
            if ui_kit::button(ui, tr("arclet.layers.new"), Button::Primary).clicked()
                && let Some(made) = self.exec(cad, "layer.new", json!({}))
            {
                self.chosen_layer = made.get("layer").and_then(Value::as_u64);
            }
            if ui_kit::button(ui, tr("arclet.layers.rename"), ui_kit::Button::Enabled(target != DEFAULT)).clicked()
                && let Some(row) = rows.iter().find(|row| row.id == target)
            {
                self.layer_rename = Some((target, row.name.clone()));
            }
            if ui_kit::button(ui, tr("arclet.layers.assign"), ui_kit::Button::Enabled(!bodies.is_empty())).clicked() {
                self.exec(cad, "layer.assign", json!({ "bodies": bodies, "layer": target }));
            }
            if ui_kit::button(ui, tr("arclet.layers.make_current"), ui_kit::Button::Enabled(target != current)).clicked() {
                self.exec(cad, "layer.set_current", json!({ "layer": target }));
            }
        });
        if let Some((id, mut text)) = self.layer_rename.take() {
            let mut keep = true;
            ui.horizontal(|ui| {
                let input_id = ui.make_persistent_id(("arclet-layer-rename", id));
                ui.allocate_ui(vec2(150.0, Tokens::get().control_height), |ui| ui_kit::cell_editor(ui, input_id, &mut text, "", false));
                if ui_kit::button(ui, tr("arclet.panel.group_save"), Button::Primary).clicked() {
                    let name = text.trim().to_string();
                    // A refused name keeps the editor open with the reason shown above.
                    keep = name.is_empty() || self.exec(cad, "layer.rename", json!({ "layer": id, "name": name })).is_none();
                }
                if ui_kit::button(ui, tr("arclet.panel.group_cancel"), Button::Normal).clicked() {
                    keep = false;
                }
            });
            if keep {
                self.layer_rename = Some((id, text));
            }
        }

        ui.add_space(6.0);
        for row in &rows {
            ui.horizontal(|ui| {
                if row.id == DEFAULT {
                    ui.add_space(52.0);
                } else {
                    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
                    icons::paint_icon(ui.painter(), rect.shrink(3.0), "eye", if row.visible { 1.0 } else { 0.35 });
                    if !row.visible {
                        ui.painter().line_segment(
                            [pos2(rect.left() + 5.0, rect.bottom() - 5.0), pos2(rect.right() - 5.0, rect.top() + 5.0)],
                            Stroke::new(1.5, tokens.text_dim),
                        );
                    }
                    let toggle = if row.visible { tr("arclet.layers.hide") } else { tr("arclet.layers.show") };
                    if response.on_hover_text(toggle).clicked() {
                        self.exec(cad, "layer.set_visible", json!({ "layer": row.id, "visible": !row.visible }));
                    }
                    let lock = if row.locked { tr("arclet.layers.unlock") } else { tr("arclet.layers.lock") };
                    let look = if row.locked { Button::SmallSelected } else { Button::Small };
                    if ui_kit::button(ui, lock, look).clicked() {
                        self.exec(cad, "layer.set_locked", json!({ "layer": row.id, "locked": !row.locked }));
                    }
                }
                let (swatch, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                ui.painter().rect_filled(swatch, 2.0, parse_colour(&row.colour, tokens.text_dim));
                let mut label = format!("{} ({})", row.name, message("arclet.layers.items", &[("count", &row.count.to_string())]));
                if row.id == current {
                    label = format!("{label} · {}", tr("arclet.layers.current"));
                }
                if ui_kit::button(ui, label, if target == row.id { Button::SmallSelected } else { Button::Ghost }).clicked() {
                    self.chosen_layer = Some(row.id);
                }
            });
        }
    }

    fn display(&mut self, cad: &mut SolveApp, ui: &mut Ui) {
        let tokens = Tokens::get();
        ui.label(RichText::new(tr("arclet.panel.display_style")).color(tokens.text_dim));
        let current = usize::from(cad.ui.visual_style);
        ui.horizontal_wrapped(|ui| {
            for (index, (style, label)) in STYLES.iter().enumerate() {
                let look = if index == current { Button::SmallSelected } else { Button::Small };
                if ui_kit::button(ui, tr(label), look).clicked() {
                    self.exec(cad, "view.visual_style", json!({ "style": style }));
                }
            }
        });
        ui.add_space(8.0);

        let mut grid = cad.ui.show_grid;
        if ui_kit::checkbox(ui, tr("arclet.panel.display_grid"), &mut grid).changed() {
            self.exec(cad, "view.grid", json!({ "on": grid }));
        }
        let mut shadow = cad.ui.ground_shadow;
        if ui_kit::checkbox(ui, tr("arclet.panel.display_shadow"), &mut shadow).changed() {
            self.exec(cad, "view.ground_shadow", json!({ "on": shadow }));
        }
        let mut occlusion = cad.preferences.ambient_occlusion;
        if ui_kit::checkbox(ui, tr("arclet.panel.display_ao"), &mut occlusion).changed() {
            self.exec(cad, "view.ambient_occlusion", json!({ "enabled": occlusion }));
        }

        ui.add_space(8.0);
        ui.label(RichText::new(tr("arclet.panel.display_preset")).color(tokens.text_dim));
        let prefs = &cad.preferences;
        let preset = graphics_preset(prefs.ambient_occlusion, &prefs.display_quality, prefs.supersampling, prefs.supersampling_adaptive);
        ui.horizontal_wrapped(|ui| {
            for (name, label) in [
                ("performance", "arclet.panel.preset_performance"),
                ("quality", "arclet.panel.preset_quality"),
                ("custom", "arclet.panel.preset_custom"),
            ] {
                let look = if name == preset { Button::SmallSelected } else { Button::Small };
                // Custom is what a manual change leaves behind; it is shown, not chosen.
                let enabled = name != "custom";
                if ui_kit::button(ui, tr(label), if enabled { look } else { ui_kit::Button::Enabled(false) }).clicked() && enabled {
                    self.exec(cad, "view.graphics_preset", json!({ "preset": name }));
                }
            }
        });
    }
}

/// The kind word for the Properties tab.
fn kind_label(kind: &str) -> &str {
    tr(match kind {
        "body" => "arclet.panel.kind.body",
        "face" => "arclet.panel.kind.face",
        "edge" => "arclet.panel.kind.edge",
        "feature" => "arclet.panel.kind.feature",
        "sketch" => "arclet.panel.kind.sketch",
        "occurrence" => "arclet.panel.kind.occurrence",
        _ => "arclet.panel.kind.other",
    })
}

/// The kind and name of one selected item, as the Properties tab shows them.
fn describe(session: &Session, sel: &Sel) -> (&'static str, String) {
    match sel {
        Sel::Body { name } => ("body", name.clone()),
        Sel::Face { body, .. } => ("face", body.clone()),
        Sel::Edge { body, .. } => ("edge", body.clone()),
        Sel::Feature { id } => ("feature", session.doc.feature(*id).map_or_else(|| id.to_string(), |f| f.name.clone())),
        Sel::SketchCurve { id, .. } => ("sketch", id.clone()),
        Sel::Occurrence { .. } => ("occurrence", String::new()),
        _ => ("other", String::new()),
    }
}

/// The bodies the selection names, once each: a body, or the body of a face or an edge.
fn selected_bodies(selection: &[Sel]) -> Vec<String> {
    let mut bodies: Vec<String> = Vec::new();
    for sel in selection {
        let name = match sel {
            Sel::Body { name } => name,
            Sel::Face { body, .. } | Sel::Edge { body, .. } => body,
            _ => continue,
        };
        if !bodies.contains(name) {
            bodies.push(name.clone());
        }
    }
    bodies
}

fn group_rows(session: &Session) -> Vec<GroupRow> {
    session.doc.browser_groups.iter().map(|g| GroupRow { id: g.id, name: g.name.clone(), component: g.component, items: g.items.clone() }).collect()
}

/// The group Rename and Ungroup act on: the one picked, or else the group that holds the
/// selected body. A picked group that no longer exists (after an undo) is not used.
fn group_target(groups: &[GroupRow], chosen: Option<u64>, bodies: &[String]) -> Option<u64> {
    if let Some(id) = chosen.filter(|id| groups.iter().any(|g| g.id == *id)) {
        return Some(id);
    }
    groups.iter().find(|g| g.items.iter().any(|item| bodies.contains(item))).map(|g| g.id)
}

/// A layer as the Layers tab shows it. The default layer comes first and has no eye or lock.
struct LayerRow {
    id: u64,
    name: String,
    colour: String,
    visible: bool,
    locked: bool,
    count: usize,
}

fn layer_rows(session: &Session) -> Vec<LayerRow> {
    let layers = &session.doc.layers;
    let mut rows = vec![LayerRow {
        id: DEFAULT,
        name: tr("arclet.layers.default").to_string(),
        colour: String::new(),
        visible: true,
        locked: false,
        count: layers.members(DEFAULT).len(),
    }];
    rows.extend(layers.items.iter().map(|l| LayerRow {
        id: l.id,
        name: l.name.clone(),
        colour: l.colour.clone(),
        visible: l.visible,
        locked: l.locked,
        count: layers.members(l.id).len(),
    }));
    rows
}

/// A `#rrggbb` colour; anything else is drawn in the given fallback.
fn parse_colour(hex: &str, fallback: egui::Color32) -> egui::Color32 {
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2).unwrap_or(""), 16).ok();
    match (channel(1), channel(3), channel(5)) {
        (Some(r), Some(g), Some(b)) if hex.len() == 7 => egui::Color32::from_rgb(r, g, b),
        _ => fallback,
    }
}

/// The preset `prefs::graphics_preset` names, from the same values. Manual edits show as custom.
fn graphics_preset(ambient_occlusion: bool, display_quality: &str, supersampling: f32, adaptive: bool) -> &'static str {
    // The presets are exact values a preset button writes, so equality is intended.
    if !ambient_occlusion && display_quality == "normal" && supersampling == 1.0 {
        return "performance";
    }
    if ambient_occlusion && display_quality == "fine" && adaptive && supersampling == 2.0 {
        return "quality";
    }
    "custom"
}

fn num(value: &Value) -> Option<f64> {
    value.as_f64()
}

/// The rows of a measurement, from the same JSON the Properties card reads (`inspect.measure`).
fn measurement_rows(ui: &mut Ui, cad: &SolveApp, value: &Value) {
    let tokens = Tokens::get();
    let total = &value["total"];
    let bodies = value["bodies"].as_array().map_or(0, Vec::len);
    let skipped = total["open_bodies_skipped"].as_u64().unwrap_or(0);
    let mut lines: Vec<(&'static str, String)> = Vec::new();
    if bodies > 0 && u64::try_from(bodies).is_ok_and(|n| n == skipped) {
        lines.push(("arclet.panel.open_surface", String::new()));
    } else {
        if let Some(mass) = num(&total["mass_g"]) {
            lines.push(("arclet.panel.mass", format!("{mass:.3} g")));
        }
        if let Some(volume) = num(&total["volume_mm3"]) {
            lines.push(("arclet.panel.volume", prefs::show_mm(cad, volume, 3)));
        }
    }
    if let Some(area) = num(&total["area_mm2"]) {
        lines.push(("arclet.panel.area", prefs::show_mm(cad, area, 2)));
    }
    // One body's own centre and box; a total for several bodies has the centre in `total`.
    let geometry = if total.get("center_of_mass").is_some() {
        Some(total)
    } else if bodies == 1 {
        value["bodies"].get(0)
    } else {
        None
    };
    if let Some(body) = geometry {
        if let Some(centre) = body["center_of_mass"].as_array() {
            let parts: Vec<String> = centre.iter().filter_map(num).map(|x| prefs::show_mm(cad, x, 2)).collect();
            lines.push(("arclet.panel.center", parts.join(", ")));
        }
        if let (Some(min), Some(max)) = (body["bbox"]["min"].as_array(), body["bbox"]["max"].as_array()) {
            let parts: Vec<String> =
                min.iter().zip(max).map(|(lo, hi)| prefs::show_mm(cad, num(hi).unwrap_or(0.0) - num(lo).unwrap_or(0.0), 2)).collect();
            lines.push(("arclet.panel.bbox", parts.join(" × ")));
        }
    }
    lines.push(("arclet.panel.counts", format!("{} / {} / {}", count(&total["faces"]), count(&total["edges"]), count(&total["vertices"]))));
    egui::Grid::new("arclet_panel_measure").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
        for (label, text) in lines {
            ui.label(RichText::new(tr(label)).color(tokens.text_dim));
            ui.label(text);
            ui.end_row();
        }
    });
}

/// Name / value rows for the selection summary.
fn rows(ui: &mut Ui, id: &str, lines: &[(&str, String)]) {
    let tokens = Tokens::get();
    egui::Grid::new(id).num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
        for (label, text) in lines {
            ui.label(RichText::new(tr(label)).color(tokens.text_dim));
            ui.label(text.as_str());
            ui.end_row();
        }
    });
}

/// A count from the measurement JSON, or a dash when the engine did not report it.
fn count(value: &Value) -> String {
    value.as_u64().map_or_else(|| "—".to_string(), |n| n.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(id: u64, items: &[&str]) -> GroupRow {
        GroupRow { id, name: format!("G{id}"), component: 0, items: items.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn presets_and_visual_styles_match_the_engine_order() {
        assert_eq!(graphics_preset(false, "normal", 1.0, true), "performance");
        assert_eq!(graphics_preset(true, "fine", 2.0, true), "quality");
        assert_eq!(graphics_preset(true, "normal", 1.0, true), "custom");
        assert_eq!(graphics_preset(true, "fine", 2.0, false), "custom");
        let names: Vec<&str> = STYLES.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["shaded_with_edges", "shaded", "wireframe", "shaded_with_hidden_edges"]);
    }

    #[test]
    fn layer_colours_parse_and_fall_back() {
        let fallback = egui::Color32::GRAY;
        assert_eq!(parse_colour("#4a90d9", fallback), egui::Color32::from_rgb(0x4a, 0x90, 0xd9));
        assert_eq!(parse_colour("", fallback), fallback);
        assert_eq!(parse_colour("#zz0000", fallback), fallback);
    }

    #[test]
    fn the_current_group_is_the_picked_one_or_the_one_holding_the_selected_body() {
        let groups = [group(1, &["Plate"]), group(2, &["Lid", "Hinge"])];
        // A picked group that exists wins, even over the selection's own group.
        assert_eq!(group_target(&groups, Some(2), &["Plate".to_string()]), Some(2));
        // A picked group that was undone away is ignored, so the selected body's group is used.
        assert_eq!(group_target(&groups, Some(9), &["Plate".to_string()]), Some(1));
        // With no pick and no grouped body, nothing is current.
        assert_eq!(group_target(&groups, None, &["Sphere".to_string()]), None);
    }
}
