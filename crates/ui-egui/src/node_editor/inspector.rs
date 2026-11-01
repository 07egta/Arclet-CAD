use egui::Ui;
use solidcraft_graph::{Modifier, NodeId, Value, component};

use super::{NodeAction, NodeEditor, Tokens, action_button, component_label, input_label, modifier_label, tr};

/// A frame owns only values it can edit. Geometry and large text stay in the graph; selecting a
/// node must not copy megabytes of mesh data on every pointer repaint.
enum InputValue {
    Number(f64),
    Bool(bool),
    Text(String),
    Summary(String),
}

impl InputValue {
    fn of(value: &Value) -> Self {
        match value {
            Value::Number(number) => Self::Number(*number),
            Value::Bool(value) => Self::Bool(*value),
            Value::Text(text) if text.len() <= 4096 => Self::Text(text.clone()),
            other => Self::Summary(value_summary(other)),
        }
    }
}

impl NodeEditor {
    pub(super) fn inspector(&mut self, ui: &mut Ui, actions: &mut Vec<NodeAction>, now: f64) {
        let t = Tokens::get();
        let Some(id) = self.selected else {
            ui.heading(tr("ui.nodes.inspector"));
            ui.label(tr("ui.nodes.select_hint"));
            return;
        };
        let Some(node) = self.graph.nodes.get(&id) else { return };
        let Some(c) = component(node.component) else { return };
        let inputs: Vec<_> = node.inputs.iter().map(|socket| (socket.wire, socket.modifier, InputValue::of(&socket.constant))).collect();
        ui.heading(component_label(c.name));
        ui.label(egui::RichText::new(format!("#{}", id.0.saturating_add(1))).small().color(t.text_dim));
        ui.add_space(8.0);
        for (input, (wire, current_modifier, value)) in inputs.into_iter().enumerate() {
            let Some(spec) = c.inputs.get(input) else { continue };
            ui.push_id(("node-inspect", id.0, input), |ui| {
                ui.strong(input_label(spec.name));
                if let Some(wire) = wire {
                    ui.horizontal_wrapped(|ui| {
                        let source =
                            self.graph.nodes.get(&wire.from).map(|source| component_label(source.component)).unwrap_or(tr("ui.nodes.missing"));
                        ui.label(format!("{source} #{}", wire.from.0.saturating_add(1)));
                        if crate::ui_kit::button(ui, tr("ui.nodes.disconnect"), crate::ui_kit::Button::Small).clicked() {
                            self.checkpoint();
                            if let Err(error) = self.graph.disconnect(id, input) {
                                self.error = Some(error.to_string());
                            } else {
                                self.changed(now);
                            }
                        }
                    });
                } else if matches!(spec.name, "shape" | "curve" | "solid" | "solids" | "layers") && matches!(&value, InputValue::Number(_)) {
                    ui.label(egui::RichText::new(tr("ui.nodes.connect_shape")).small().color(t.text_dim));
                } else {
                    match value {
                        InputValue::Number(value) => self.number_input(ui, id, input, spec.name, value, now),
                        InputValue::Bool(mut value) => {
                            if crate::ui_kit::checkbox(ui, tr("ui.nodes.enabled"), &mut value).changed() {
                                self.checkpoint();
                                self.set_input(id, input, Value::Bool(value), now);
                            }
                        }
                        InputValue::Text(mut text) => {
                            if crate::ui_kit::text_area(ui, input_label(spec.name), ui.make_persistent_id("text"), &mut text, 3, 4096).changed() {
                                self.checkpoint();
                                self.set_input(id, input, Value::Text(text), now);
                            }
                        }
                        InputValue::Summary(text) => {
                            ui.label(text);
                        }
                    }
                }
                let choices = [Modifier::None, Modifier::Flatten, Modifier::Graft];
                let mut choice = choices.iter().position(|choice| *choice == current_modifier).unwrap_or(0);
                let labels = choices.map(modifier_label);
                crate::ui_kit::dropdown(ui, "modifier", &mut choice, &labels);
                let modifier = choices.get(choice).copied().unwrap_or(Modifier::None);
                if modifier != current_modifier {
                    self.checkpoint();
                    match self.graph.set_modifier(id, input, modifier) {
                        Ok(()) => self.changed(now),
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
                ui.add_space(10.0);
            });
        }
        ui.separator();
        ui.strong(tr("ui.nodes.output"));
        if let Some(error) = self.evaluation().and_then(|evaluation| evaluation.errors.get(&id)) {
            ui.colored_label(t.error, error);
        } else if let Some(evaluation) = self.evaluation() {
            if let Some(tree) = evaluation.outputs.get(&(id, 0)) {
                ui.label(format!("{} {} / {} {}", tree.item_count(), tr("ui.nodes.items"), tree.branches.len(), tr("ui.nodes.branches")));
                let first = tree.branches.values().flatten().next();
                if let Some(value) = first {
                    ui.label(value_summary(value));
                }
                let mut mesh_count = 0usize;
                for value in tree.branches.values().flatten() {
                    if let Value::Solid(triangles) = value {
                        mesh_count = mesh_count.saturating_add(triangles.len());
                    }
                }
                if mesh_count > 0 && mesh_count <= 100_000 {
                    if action_button(ui, "ui.nodes.use_mesh", "import", true).clicked() {
                        let mut mesh = Vec::with_capacity(mesh_count);
                        for value in tree.branches.values().flatten() {
                            if let Value::Solid(triangles) = value {
                                mesh.extend(triangles.iter().copied());
                            }
                        }
                        actions.push(NodeAction::PreviewMesh(std::sync::Arc::new(mesh)));
                    }
                } else if mesh_count > 100_000 {
                    ui.colored_label(t.warning, tr("ui.nodes.mesh_too_large"));
                }
                if let Some(Value::Text(text)) = first {
                    if c.name == "G-code" {
                        ui.colored_label(t.warning, tr("ui.nodes.print_unverified"));
                        if action_button(ui, "ui.nodes.export_gcode", "export", true).clicked() {
                            actions.push(NodeAction::ExportGcode(text.clone()));
                        }
                    } else {
                        let preview: String = text.chars().take(400).collect();
                        ui.monospace(preview);
                    }
                }
            } else {
                ui.label(tr("ui.nodes.pending"));
            }
        } else {
            ui.label(tr("ui.nodes.run_hint"));
        }
        ui.add_space(12.0);
        if action_button(ui, "ui.nodes.delete", "delete", true).clicked() {
            self.checkpoint();
            self.graph.remove_node(id);
            self.selected = None;
            self.wire_start = self.wire_start.filter(|(source, _)| *source != id);
            self.changed(now);
        }
    }

    fn set_input(&mut self, id: NodeId, input: usize, value: Value, now: f64) {
        match self.graph.set_constant(id, input, value) {
            Ok(()) => self.changed(now),
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn number_input(&mut self, ui: &mut Ui, id: NodeId, input: usize, name: &str, value: f64, now: f64) {
        let (range, integer) = number_range(name);
        let range = range.start().min(value)..=range.end().max(value);
        let mut number = value;
        let width = ui.available_width();
        let response = ui
            .horizontal(|ui| {
                let slider = ui
                    .allocate_ui_with_layout(egui::vec2((width - 80.0).max(70.0), 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        crate::ui_kit::slider(ui, &mut number, range, crate::ui_kit::Slider { integer, show_value: false, ..Default::default() })
                    })
                    .inner;
                let exact = crate::ui_kit::number_field(
                    ui,
                    &mut number,
                    value.min(-1_000_000.0)..=value.max(1_000_000.0),
                    if integer { 1.0 } else { 0.1 },
                    "",
                );
                slider.union(exact)
            })
            .inner;
        crate::scenario::publish_target(&format!("nodes:value:{}:{input}", id.0), response.rect);
        if response.drag_started() || response.gained_focus() {
            self.number_edit = Some(self.graph.clone());
        }
        if response.changed() {
            if self.number_edit.is_none() {
                self.number_edit = Some(self.graph.clone());
            }
            if number.is_finite() && number.abs() <= 1_000_000.0 {
                self.set_input(id, input, Value::Number(number), now);
            } else {
                self.error = Some(tr("ui.nodes.invalid_number").into());
            }
        }
        if (response.drag_stopped()
            || response.lost_focus()
            || (response.changed() && !response.dragged())
            || ui.input(|input| input.pointer.secondary_clicked()))
            && let Some(before) = self.number_edit.take()
            && before != self.graph
        {
            self.push_undo(before);
        }
    }
}

fn number_range(name: &str) -> (std::ops::RangeInclusive<f64>, bool) {
    match name {
        "segments" => (3.0..=128.0, true),
        "count" => (1.0..=64.0, true),
        "walls" => (0.0..=8.0, true),
        "layer height" => (0.04..=0.6, false),
        "line width" => (0.1..=1.0, false),
        "nozzle temp" => (0.0..=300.0, true),
        "bed temp" => (0.0..=120.0, true),
        "radius" | "width" | "depth" | "height" => (0.1..=100.0, false),
        "degrees" => (-180.0..=180.0, false),
        "infill spacing" => (0.0..=20.0, false),
        _ => (-100.0..=100.0, false),
    }
}

fn value_summary(value: &Value) -> String {
    match value {
        Value::Number(number) => format!("{number:.5}"),
        Value::Bool(value) => tr(if *value { "ui.nodes.true" } else { "ui.nodes.false" }).into(),
        Value::Text(text) => format!("{} {}", text.len(), tr("ui.nodes.bytes")),
        Value::Point(point) => format!("({:.3}, {:.3}, {:.3})", point.x, point.y, point.z),
        Value::Poly(poly) => format!("{} {}", poly.pts.len(), tr("ui.nodes.points")),
        Value::Solid(triangles) => format!("{} {}", triangles.len(), tr("ui.nodes.triangles")),
        Value::Layers(layers) => format!("{} {}", layers.layers.len(), tr("ui.nodes.layers")),
        Value::List(values) => format!("{} {}", values.len(), tr("ui.nodes.items")),
    }
}
