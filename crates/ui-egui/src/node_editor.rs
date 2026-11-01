//! A small visual graph workspace shared by the desktop and browser hosts.
//! The host owns file pickers and mesh insertion; this module never opens a modal dialog.

use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};

use egui::{Ui, vec2};
use solidcraft_graph::{Evaluation, Graph, GraphError, Modifier, NodeId, Value};

use crate::i18n::tr;
use crate::theme::Tokens;

mod canvas;
mod inspector;
#[cfg(test)]
mod tests;

const MAX_ERROR_CHARS: usize = 512;

pub type NodeMesh = Arc<Vec<[solidcraft_engine::geom::Vec3; 3]>>;

/// Requests which the native/web host can fulfil using its existing services.
pub enum NodeAction {
    SaveJson(String),
    OpenJson,
    PreviewMesh(NodeMesh),
    ExportGcode(String),
}

#[cfg(not(target_arch = "wasm32"))]
struct Job {
    result: std::sync::mpsc::Receiver<Result<Evaluation, GraphError>>,
    cancel: Arc<AtomicBool>,
    revision: u64,
}

#[cfg(target_arch = "wasm32")]
struct Job {
    evaluation: solidcraft_graph::PreviewEvaluation,
    revision: u64,
}

/// Editable document, transient gesture state, and one active evaluation at most.
pub struct NodeEditor {
    graph: Graph,
    selected: Option<NodeId>,
    evaluation: Option<Evaluation>,
    job: Option<Job>,
    revision: u64,
    evaluated_revision: Option<u64>,
    pending_run: bool,
    auto_run: bool,
    last_change: f64,
    error: Option<String>,
    placing: Option<&'static str>,
    wire_start: Option<(NodeId, usize)>,
    drag: Option<(NodeId, (f32, f32))>,
    undo: Vec<Graph>,
    redo: Vec<Graph>,
    number_edit: Option<Graph>,
    scroll_to: Option<NodeId>,
    saved_graph: Option<Graph>,
}

impl Default for NodeEditor {
    fn default() -> Self {
        let graph = sample_graph().unwrap_or_default();
        let selected = graph.nodes.keys().next_back().copied();
        Self {
            graph,
            selected,
            evaluation: None,
            job: None,
            revision: 0,
            evaluated_revision: None,
            pending_run: true,
            auto_run: true,
            last_change: 0.0,
            error: None,
            placing: None,
            wire_start: None,
            drag: None,
            undo: Vec::new(),
            redo: Vec::new(),
            number_edit: None,
            scroll_to: None,
            saved_graph: None,
        }
    }
}

impl Drop for NodeEditor {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// A working starter: a circle and a height control connected to an extrusion.
pub fn sample_graph() -> Result<Graph, GraphError> {
    let mut graph = Graph::new();
    let circle = graph.add_node("Circle", (64.0, 72.0))?;
    let height = graph.add_node("Number", (64.0, 290.0))?;
    let extrude = graph.add_node("Extrude", (398.0, 140.0))?;
    graph.set_constant(height, 0, Value::Number(18.0))?;
    graph.connect_checked(circle, 0, extrude, 0)?;
    graph.connect_checked(height, 0, extrude, 1)?;
    Ok(graph)
}

impl NodeEditor {
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn evaluation(&self) -> Option<&Evaluation> {
        self.evaluation.as_ref().filter(|_| self.evaluated_revision == Some(self.revision))
    }

    pub fn is_running(&self) -> bool {
        self.job.is_some()
    }

    pub fn is_dirty(&self) -> bool {
        self.saved_graph.as_ref() != Some(&self.graph)
    }

    pub fn mark_saved(&mut self) {
        self.saved_graph = Some(self.graph.clone());
    }

    /// A recovery document or a newly adopted graph is explicitly unsaved, even when its
    /// contents happen to equal the host's starter graph or a previous saved snapshot.
    pub fn mark_unsaved(&mut self) {
        self.saved_graph = None;
    }

    /// Mark the exact asynchronously saved snapshot, preserving dirty status for later edits.
    pub fn mark_saved_json(&mut self, text: &str) -> Result<(), String> {
        self.saved_graph = Some(Graph::from_json(text).map_err(|error| error.to_string())?);
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.graph.to_json().map_err(|error| error.to_string())
    }

    /// An invalid file leaves the current graph, output and undo history untouched.
    pub fn load_json(&mut self, text: &str) -> Result<(), String> {
        let graph = Graph::from_json(text).map_err(|error| error.to_string())?;
        self.replace_graph(graph)?;
        self.mark_saved();
        Ok(())
    }

    pub fn replace_graph(&mut self, graph: Graph) -> Result<(), String> {
        // Round-tripping uses the same finite-value and resource rules as file loading.
        let text = graph.to_json().map_err(|error| error.to_string())?;
        let graph = Graph::from_json(&text).map_err(|error| error.to_string())?;
        self.checkpoint();
        self.graph = graph;
        self.selected = self.graph.nodes.keys().next_back().copied();
        self.placing = None;
        self.wire_start = None;
        self.drag = None;
        self.changed(0.0);
        Ok(())
    }

    pub fn report_error(&mut self, error: impl Into<String>) {
        // Parser errors can repeat an attacker-controlled field or component name. Bound the
        // retained display text before egui lays it out, without cutting a UTF-8 character.
        let error = error.into();
        let mut chars = error.chars();
        let mut display: String = chars.by_ref().take(MAX_ERROR_CHARS).collect();
        if chars.next().is_some() {
            display.pop();
            display.push('…');
        }
        self.error = Some(display);
    }

    pub fn cancel(&mut self) {
        self.pending_run = false;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.job = None;
        }
    }

    fn checkpoint(&mut self) {
        self.push_undo(self.graph.clone());
    }

    fn push_undo(&mut self, graph: Graph) {
        const MAX_UNDO: usize = 24;
        if self.undo.last() != Some(&graph) {
            if self.undo.len() >= MAX_UNDO {
                self.undo.remove(0);
            }
            self.undo.push(graph);
        }
        self.redo.clear();
    }

    fn changed(&mut self, now: f64) {
        self.cancel();
        self.revision = self.revision.wrapping_add(1);
        self.last_change = now;
        self.pending_run = self.auto_run;
        self.error = None;
    }

    fn undo(&mut self, now: f64) {
        if let Some(graph) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.graph, graph));
            self.selected = self.selected.filter(|id| self.graph.nodes.contains_key(id));
            self.changed(now);
        }
    }

    fn redo(&mut self, now: f64) {
        if let Some(graph) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.graph, graph));
            self.changed(now);
        }
    }

    /// Begin a run without blocking the host's UI. A repeated run cancels the active worker
    /// and waits for it to exit before starting the next; slider drags cannot spawn a backlog.
    pub fn run(&mut self, ctx: &egui::Context) {
        self.cancel();
        self.pending_run = true;
        self.last_change = -1.0;
        self.poll(ctx);
    }

    fn finish(&mut self, result: Result<Evaluation, GraphError>, revision: u64) {
        if revision != self.revision {
            return;
        }
        match result {
            Ok(evaluation) => {
                self.evaluation = Some(evaluation);
                self.evaluated_revision = Some(revision);
                self.error = None;
            }
            Err(GraphError::Cancelled) => {}
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn poll(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let received = self.job.as_ref().and_then(|job| match job.result.try_recv() {
                Ok(result) => Some((result, job.revision)),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => Some((Err(GraphError::Invalid("evaluation worker closed".into())), job.revision)),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
            });
            if let Some((result, revision)) = received {
                self.job = None;
                self.finish(result, revision);
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let finished = self.job.as_mut().and_then(|job| match job.evaluation.step(&|| false) {
                Ok(false) => None,
                result => Some(result),
            });
            if let Some(result) = finished
                && let Some(job) = self.job.take()
            {
                self.finish(result.and_then(|_| job.evaluation.finish()), job.revision);
            }
        }
        let now = ctx.input(|input| input.time);
        if self.job.is_none() && self.pending_run && now - self.last_change >= 0.2 {
            self.pending_run = false;
            self.error = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                let graph = self.graph.clone();
                let cancel = Arc::new(AtomicBool::new(false));
                let stop = cancel.clone();
                let ctx = ctx.clone();
                let (sender, result) = std::sync::mpsc::sync_channel(1);
                match std::thread::Builder::new().name("node-evaluation".into()).spawn(move || {
                    let evaluation = graph.evaluate_until(&|| stop.load(Ordering::Relaxed));
                    let _ = sender.send(evaluation);
                    ctx.request_repaint();
                }) {
                    Ok(_) => self.job = Some(Job { result, cancel, revision: self.revision }),
                    Err(error) => self.error = Some(error.to_string()),
                }
            }
            #[cfg(target_arch = "wasm32")]
            match solidcraft_graph::PreviewEvaluation::new(self.graph.clone()) {
                Ok(evaluation) => self.job = Some(Job { evaluation, revision: self.revision }),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        if self.job.is_some() || self.pending_run {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    /// Draw inside the host's existing outer container.
    pub fn show(&mut self, ui: &mut Ui) -> Vec<NodeAction> {
        ui.painter().rect_filled(ui.available_rect_before_wrap(), 0, Tokens::get().panel);
        self.poll(ui.ctx());
        let mut actions = Vec::new();
        let now = ui.input(|input| input.time);
        self.toolbar(ui, &mut actions, now);
        ui.separator();
        if let Some(error) = &self.error {
            ui.colored_label(Tokens::get().error, error);
        }
        let hint = if self.placing.is_some() {
            "ui.nodes.place_hint"
        } else if self.wire_start.is_some() {
            "ui.nodes.wire_hint"
        } else {
            "ui.nodes.canvas_hint"
        };
        ui.label(egui::RichText::new(tr(hint)).small().color(Tokens::get().text_dim));
        let available = ui.available_size();
        if available.x < 560.0 {
            self.canvas(ui, available.y.min(340.0));
            ui.separator();
            self.inspector(ui, &mut actions, now);
        } else {
            ui.horizontal_top(|ui| {
                let inspector_width = 250.0;
                ui.allocate_ui_with_layout(
                    vec2((available.x - inspector_width - 18.0).max(260.0), available.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.canvas(ui, available.y);
                    },
                );
                ui.separator();
                ui.allocate_ui_with_layout(vec2(inspector_width, available.y), egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::ScrollArea::vertical().id_salt("node-inspector").show(ui, |ui| self.inspector(ui, &mut actions, now));
                });
            });
        }
        actions
    }

    fn toolbar(&mut self, ui: &mut Ui, actions: &mut Vec<NodeAction>, now: f64) {
        ui.horizontal_wrapped(|ui| {
            ui.strong(tr("ui.nodes.title"));
            if self.is_dirty() {
                ui.label(egui::RichText::new(tr("ui.nodes.unsaved")).small().color(Tokens::get().text_dim));
            }
            crate::ui_kit::Menu::button(ui, tr("ui.nodes.add"), |ui| {
                for group in ["Math", "Curves", "Transform", "Pattern", "Solid", "Print"] {
                    crate::ui_kit::Menu::button(ui, group_label(group), |ui| {
                        for c in solidcraft_graph::COMPONENTS.iter().filter(|c| c.group == group) {
                            if crate::ui_kit::button(ui, component_label(c.name), crate::ui_kit::Button::Normal).clicked() {
                                self.placing = Some(c.name);
                                self.wire_start = None;
                                ui.close();
                            }
                        }
                    });
                }
            });
            if action_button(ui, "ui.nodes.run", "play", true).clicked() {
                self.run(ui.ctx());
            }
            if action_button(ui, "ui.nodes.stop", "stop", self.job.is_some() || self.pending_run).clicked() {
                self.cancel();
            }
            if crate::ui_kit::checkbox(ui, tr("ui.nodes.live"), &mut self.auto_run).changed() && self.auto_run {
                self.pending_run = true;
            }
            ui.separator();
            if action_button(ui, "ui.nodes.undo", "undo", !self.undo.is_empty()).clicked() {
                self.undo(now);
            }
            if action_button(ui, "ui.nodes.redo", "redo", !self.redo.is_empty()).clicked() {
                self.redo(now);
            }
            if action_button(ui, "ui.nodes.open", "open", true).clicked() {
                actions.push(NodeAction::OpenJson);
            }
            if action_button(ui, "ui.nodes.save", "save", true).clicked() {
                match self.to_json() {
                    Ok(text) => actions.push(NodeAction::SaveJson(text)),
                    Err(error) => self.error = Some(error),
                }
            }
            if crate::ui_kit::button(ui, tr("ui.nodes.home"), crate::ui_kit::Button::Normal).clicked() {
                self.scroll_to = self.selected.or_else(|| self.graph.nodes.keys().next().copied());
            }
            crate::ui_kit::Menu::button(ui, tr("ui.nodes.document"), |ui| {
                if crate::ui_kit::button(ui, tr("ui.nodes.example"), crate::ui_kit::Button::Normal).clicked() {
                    match sample_graph().map_err(|error| error.to_string()).and_then(|graph| self.replace_graph(graph)) {
                        Ok(()) => {}
                        Err(error) => self.error = Some(error),
                    }
                    ui.close();
                }
                if crate::ui_kit::button(ui, tr("ui.nodes.clear"), crate::ui_kit::Button::Normal).clicked() {
                    if let Err(error) = self.replace_graph(Graph::new()) {
                        self.error = Some(error);
                    }
                    ui.close();
                }
            });
            let status = if self.job.is_some() {
                "ui.nodes.running"
            } else if self.evaluated_revision != Some(self.revision) {
                "ui.nodes.changed"
            } else if self.evaluation.as_ref().is_some_and(|evaluation| !evaluation.errors.is_empty()) {
                "ui.nodes.errors"
            } else {
                "ui.nodes.ready"
            };
            ui.label(egui::RichText::new(tr(status)).small().color(Tokens::get().text_dim));
        });
    }
}

fn action_button(ui: &mut Ui, label: &str, icon: &str, enabled: bool) -> egui::Response {
    crate::ui_kit::icon_label_button(ui, tr(label), icon, enabled, false)
}

fn component_label(name: &'static str) -> &'static str {
    let key = match name {
        "Number" => "ui.nodes.component.number",
        "Add" => "ui.nodes.component.add",
        "Subtract" => "ui.nodes.component.subtract",
        "Multiply" => "ui.nodes.component.multiply",
        "Divide" => "ui.nodes.component.divide",
        "Series" => "ui.nodes.component.series",
        "Point" => "ui.nodes.component.point",
        "Circle" => "ui.nodes.component.circle",
        "Rectangle" => "ui.nodes.component.rectangle",
        "Move" => "ui.nodes.component.move",
        "Rotate" => "ui.nodes.component.rotate",
        "Linear Pattern" => "ui.nodes.component.linear_pattern",
        "Circular Pattern" => "ui.nodes.component.circular_pattern",
        "Extrude" => "ui.nodes.component.extrude",
        "Merge Solids" => "ui.nodes.component.merge_solids",
        "Slice" => "ui.nodes.component.slice",
        "G-code" => "ui.nodes.component.gcode",
        _ => return name,
    };
    tr(key)
}

fn group_label(name: &'static str) -> &'static str {
    tr(match name {
        "Math" => "ui.nodes.group.math",
        "Curves" => "ui.nodes.group.curves",
        "Transform" => "ui.nodes.group.transform",
        "Pattern" => "ui.nodes.group.pattern",
        "Solid" => "ui.nodes.group.solid",
        "Print" => "ui.nodes.group.print",
        _ => return name,
    })
}

fn input_label(name: &'static str) -> &'static str {
    tr(match name {
        "value" => "ui.nodes.input.value",
        "a" => "ui.nodes.input.a",
        "b" => "ui.nodes.input.b",
        "result" => "ui.nodes.input.result",
        "start" => "ui.nodes.input.start",
        "step" => "ui.nodes.input.step",
        "count" => "ui.nodes.input.count",
        "list" => "ui.nodes.input.list",
        "x" => "ui.nodes.input.x",
        "y" => "ui.nodes.input.y",
        "z" => "ui.nodes.input.z",
        "point" => "ui.nodes.input.point",
        "radius" => "ui.nodes.input.radius",
        "segments" => "ui.nodes.input.segments",
        "curve" => "ui.nodes.input.curve",
        "width" => "ui.nodes.input.width",
        "depth" => "ui.nodes.input.depth",
        "shape" => "ui.nodes.input.shape",
        "degrees" => "ui.nodes.input.degrees",
        "dx" => "ui.nodes.input.dx",
        "dy" => "ui.nodes.input.dy",
        "dz" => "ui.nodes.input.dz",
        "shapes" => "ui.nodes.input.shapes",
        "height" => "ui.nodes.input.height",
        "solid" => "ui.nodes.input.solid",
        "solids" => "ui.nodes.input.solids",
        "layer height" => "ui.nodes.input.layer_height",
        "layers" => "ui.nodes.input.layers",
        "line width" => "ui.nodes.input.line_width",
        "nozzle temp" => "ui.nodes.input.nozzle_temp",
        "bed temp" => "ui.nodes.input.bed_temp",
        "walls" => "ui.nodes.input.walls",
        "infill spacing" => "ui.nodes.input.infill_spacing",
        "text" => "ui.nodes.input.text",
        _ => return name,
    })
}

fn modifier_label(modifier: Modifier) -> &'static str {
    tr(match modifier {
        Modifier::None => "ui.nodes.modifier.none",
        Modifier::Flatten => "ui.nodes.modifier.flatten",
        Modifier::Graft => "ui.nodes.modifier.graft",
    })
}
