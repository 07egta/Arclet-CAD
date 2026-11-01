use egui::{Align2, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use solidcraft_graph::{Node, NodeId, component};

use super::{NodeEditor, Tokens, component_label, group_label, input_label, tr};

const WIDTH: f32 = 220.0;
const HEADER: f32 = 54.0;
const ROW: f32 = 25.0;

fn card_rect(origin: Pos2, node: &Node) -> Rect {
    let rows = node.inputs.len().max(1);
    Rect::from_min_size(origin + vec2(node.at.0, node.at.1), vec2(WIDTH, HEADER + rows as f32 * ROW + 24.0))
}

fn input_at(rect: Rect, input: usize) -> Pos2 {
    pos2(rect.left(), rect.top() + HEADER + (input as f32 + 0.5) * ROW)
}

fn output_at(rect: Rect, output: usize) -> Pos2 {
    pos2(rect.right(), rect.top() + HEADER + (output as f32 + 0.5) * ROW)
}

fn wire(painter: &egui::Painter, start: Pos2, end: Pos2, color: egui::Color32) {
    let bend = ((end.x - start.x).abs() * 0.45).clamp(36.0, 180.0);
    painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
        [start, start + vec2(bend, 0.0), end - vec2(bend, 0.0), end],
        false,
        egui::Color32::TRANSPARENT,
        Stroke::new(2.0, color),
    ));
}

impl NodeEditor {
    pub(super) fn canvas(&mut self, ui: &mut Ui, height: f32) {
        let t = Tokens::get();
        let now = ui.input(|input| input.time);
        // ScrollArea owns graph scrolling. It is a document layout, not a second model camera.
        egui::ScrollArea::both().id_salt("node-canvas").auto_shrink([false, false]).max_height(height.max(160.0)).show_viewport(ui, |ui, view| {
            // Fill the visible workspace after resizing, while retaining scrollable graph extents.
            let size = self.graph.nodes.values().fold(view.size(), |size, node| {
                let rect = card_rect(Pos2::ZERO, node);
                vec2(size.x.max(rect.right() + 100.0), size.y.max(rect.bottom() + 100.0))
            });
            let (rect, background) = ui.allocate_exact_size(size, Sense::click());
            let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
            painter.rect_filled(rect, 0, t.viewport_bottom);
            let clip = painter.clip_rect();
            let step = 24.0;
            let mut x = rect.left() + ((clip.left() - rect.left()) / step).floor() * step;
            while x < clip.right() {
                let mut y = rect.top() + ((clip.top() - rect.top()) / step).floor() * step;
                while y < clip.bottom() {
                    painter.circle_filled(pos2(x, y), 0.8, t.grid);
                    y += step;
                }
                x += step;
            }
            for node in self.graph.nodes.values() {
                let destination = card_rect(rect.min, node);
                for (input, socket) in node.inputs.iter().enumerate() {
                    if let Some(start) = socket.wire.and_then(|wire| self.graph.nodes.get(&wire.from).map(|node| (wire.output, node))) {
                        let source = card_rect(rect.min, start.1);
                        wire(&painter, output_at(source, start.0), input_at(destination, input), t.accent);
                    }
                }
            }
            if let Some((id, output)) = self.wire_start
                && let Some(node) = self.graph.nodes.get(&id)
                && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
            {
                wire(&painter, output_at(card_rect(rect.min, node), output), pointer, t.text_dim);
            }
            let ids: Vec<NodeId> = self.graph.nodes.keys().copied().collect();
            for id in ids {
                self.card(ui, &painter, rect.min, id, now);
            }
            if background.clicked() {
                if let Some(name) = self.placing {
                    if let Some(point) = background.interact_pointer_pos() {
                        let at = (point.x - rect.left(), point.y - rect.top());
                        self.checkpoint();
                        match self.graph.add_node(name, at) {
                            Ok(id) => {
                                self.selected = Some(id);
                                self.placing = None;
                                self.changed(now);
                            }
                            Err(error) => self.error = Some(error.to_string()),
                        }
                    }
                } else {
                    self.selected = None;
                }
            }
            if background.secondary_clicked() || ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                self.placing = None;
                self.wire_start = None;
            }
            if let Some(name) = self.placing
                && let Some(pointer) = ui.input(|input| input.pointer.hover_pos()).filter(|p| clip.contains(*p))
            {
                painter.rect_stroke(Rect::from_min_size(pointer, vec2(WIDTH, 110.0)), 8, Stroke::new(1.5, t.accent), StrokeKind::Inside);
                painter.text(pointer + vec2(14.0, 16.0), Align2::LEFT_TOP, component_label(name), FontId::proportional(14.0), t.text);
            }
        });
    }

    fn card(&mut self, ui: &mut Ui, painter: &egui::Painter, origin: Pos2, id: NodeId, now: f64) {
        let Some(node) = self.graph.nodes.get(&id) else { return };
        let Some(c) = component(node.component) else { return };
        let rect = card_rect(origin, node);
        if self.scroll_to == Some(id) {
            ui.scroll_to_rect(rect.expand(32.0), Some(egui::Align::Center));
            self.scroll_to = None;
        }
        if !painter.clip_rect().intersects(rect.expand(12.0)) {
            return;
        }
        let t = Tokens::get();
        let error = self.evaluation().and_then(|evaluation| evaluation.errors.get(&id)).cloned();
        let selected = self.selected == Some(id);
        let border = if error.is_some() {
            t.error
        } else if selected {
            t.accent
        } else {
            t.border
        };
        painter.rect_filled(rect, 8, t.panel);
        painter.rect_stroke(rect, 8, Stroke::new(if selected || error.is_some() { 2.0 } else { 1.0 }, border), StrokeKind::Inside);
        let header = Rect::from_min_size(rect.min, vec2(WIDTH, HEADER));
        painter.text(rect.min + vec2(14.0, 13.0), Align2::LEFT_TOP, component_label(c.name), FontId::proportional(15.0), t.text);
        painter.text(rect.min + vec2(14.0, 33.0), Align2::LEFT_TOP, group_label(c.group), FontId::proportional(10.0), t.text_dim);
        painter.text(
            pos2(rect.right() - 12.0, rect.top() + 14.0),
            Align2::RIGHT_TOP,
            format!("#{}", id.0.saturating_add(1)),
            FontId::proportional(10.0),
            t.text_dim,
        );
        painter.line_segment([pos2(rect.left() + 10.0, header.bottom()), pos2(rect.right() - 10.0, header.bottom())], Stroke::new(1.0, t.border));
        let header_response = ui.interact(header, ui.make_persistent_id(("node-card", id.0)), Sense::click_and_drag());
        crate::scenario::publish_target(&format!("nodes:card:{}", id.0), header);
        header_response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, component_label(c.name)));
        if header_response.clicked() {
            self.selected = Some(id);
        }
        if header_response.drag_started() {
            self.selected = Some(id);
            self.drag = Some((id, node.at));
            self.checkpoint();
        }
        if header_response.dragged_by(egui::PointerButton::Primary) {
            let delta = ui.input(|input| input.pointer.delta());
            if let Some(node) = self.graph.nodes.get(&id) {
                let at = ((node.at.0 + delta.x).clamp(0.0, 1_000_000.0 - WIDTH), (node.at.1 + delta.y).clamp(0.0, 1_000_000.0 - 300.0));
                if let Err(error) = self.graph.move_node(id, at) {
                    self.error = Some(error.to_string());
                }
            }
        }
        if header_response.drag_stopped() {
            self.drag = None;
        }
        let mut connect_to = None;
        for (index, input) in c.inputs.iter().enumerate() {
            let at = input_at(rect, index);
            let socket = self.graph.nodes.get(&id).and_then(|node| node.inputs.get(index));
            let connected = socket.is_some_and(|socket| socket.wire.is_some());
            let fill = if connected { t.accent } else { t.control_fill };
            painter.circle_filled(at, 5.5, fill);
            painter.circle_stroke(at, 5.5, Stroke::new(1.5, if connected { t.accent } else { t.text_dim }));
            painter.text(at + vec2(13.0, 0.0), Align2::LEFT_CENTER, input_label(input.name), FontId::proportional(11.0), t.text_dim);
            let hit = Rect::from_center_size(at, vec2(22.0, 22.0));
            let response = ui.interact(hit, ui.make_persistent_id(("node-input", id.0, index)), Sense::click());
            crate::scenario::publish_target(&format!("nodes:input:{}:{index}", id.0), hit);
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{}: {}", component_label(c.name), input_label(input.name)))
            });
            if response.clicked() {
                self.selected = Some(id);
                if self.wire_start.is_some() {
                    connect_to = Some(index);
                }
            }
            response.on_hover_text(tr("ui.nodes.input_hint"));
        }
        for (output, name) in c.outputs.iter().enumerate() {
            let at = output_at(rect, output);
            painter.circle_filled(at, 5.5, t.accent);
            painter.text(at - vec2(13.0, 0.0), Align2::RIGHT_CENTER, input_label(name), FontId::proportional(11.0), t.text);
            let hit = Rect::from_center_size(at, vec2(22.0, 22.0));
            let response = ui.interact(hit, ui.make_persistent_id(("node-output", id.0, output)), Sense::click());
            crate::scenario::publish_target(&format!("nodes:output:{}:{output}", id.0), hit);
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{}: {}", component_label(c.name), input_label(name)))
            });
            if response.clicked() {
                self.wire_start = Some((id, output));
                self.placing = None;
                self.selected = Some(id);
            }
            response.on_hover_text(tr("ui.nodes.output_hint"));
        }
        if let Some(input) = connect_to
            && let Some((source, output)) = self.wire_start
        {
            let before = self.graph.clone();
            match self.graph.connect_checked(source, output, id, input) {
                Ok(()) => {
                    self.push_undo(before);
                    self.wire_start = None;
                    self.changed(now);
                }
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        let status = if error.is_some() {
            tr("ui.nodes.node_error").to_owned()
        } else if let Some(tree) = self.evaluation().and_then(|evaluation| evaluation.outputs.get(&(id, 0))) {
            format!("{} {}", tree.item_count(), tr("ui.nodes.items"))
        } else {
            tr("ui.nodes.pending").to_owned()
        };
        painter.text(
            pos2(rect.left() + 14.0, rect.bottom() - 12.0),
            Align2::LEFT_CENTER,
            status,
            FontId::proportional(10.0),
            if error.is_some() { t.error } else { t.text_dim },
        );
        if let Some(error) = error {
            header_response.on_hover_text(error);
        }
        if ui.input(|input| input.pointer.secondary_clicked()) && ui.rect_contains_pointer(rect) {
            self.wire_start = None;
            self.placing = None;
        }
    }
}
