//! Native desktop shell. Commands, file operations and OCCT execute on one owned worker.
pub mod camera;
mod renderer;
mod worker;
use crate::{command::HELP, Error, Result};
use camera::{ray_box, ray_triangle, Camera, ViewKind};
use eframe::{
    egui::{self, Color32, Key, PointerButton, Pos2, Rect, RichText, Stroke, Vec2},
    egui_glow, glow,
};
use glam::DVec3;
use renderer::{DisplayMode, PaintOptions, Renderer};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        mpsc::{Receiver, SyncSender, TryRecvError},
        Arc,
    },
    time::{Duration, Instant},
};
use worker::{Event, Request, Scene};

const ACCENT: Color32 = Color32::from_rgb(102, 205, 191);
#[derive(Clone)]
enum AfterSave {
    Nothing,
    Close,
    Command(String),
}
#[derive(Clone)]
struct FileDialog {
    verb: String,
    path: String,
    after: AfterSave,
}
struct LogLine {
    text: String,
    error: bool,
}
struct App {
    renderer: Arc<egui::mutex::Mutex<Renderer>>,
    tx: SyncSender<Request>,
    rx: Receiver<Event>,
    scene: Arc<Scene>,
    busy: bool,
    started: Instant,
    pending: String,
    after_save: AfterSave,
    log: VecDeque<LogLine>,
    last_error: Option<String>,
    cameras: [Camera; 4],
    four: bool,
    active: usize,
    fit_pending: bool,
    display: DisplayMode,
    grid: bool,
    selected: Vec<u64>,
    command: String,
    history: Vec<String>,
    history_index: Option<usize>,
    focus_command: bool,
    project_path: Option<String>,
    file_dialog: Option<FileDialog>,
    replace_dialog: Option<String>,
    close_dialog: bool,
    allow_close: bool,
    help: bool,
    help_filter: String,
    layer_name: String,
    rename: String,
    radius: f64,
    translation: [f64; 3],
    keep_inputs: bool,
    drawing: bool,
    draw_closed: bool,
    draw_points: Vec<DVec3>,
    snap: bool,
    snap_step: f64,
    telemetry: Option<PathBuf>,
    last_telemetry: String,
    frames: u64,
    viewport_rects: [Rect; 4],
}
fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\\\""))
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>, demo: bool) -> Result<Self> {
        let gl = cc
            .gl
            .as_ref()
            .ok_or_else(|| Error::new("OpenGL context is unavailable"))?;
        let renderer = Renderer::new(gl).map_err(Error::new)?;
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = Color32::from_rgb(27, 31, 38);
        style.visuals.window_fill = Color32::from_rgb(31, 36, 44);
        style.visuals.extreme_bg_color = Color32::from_rgb(19, 23, 29);
        style.visuals.selection.bg_fill = Color32::from_rgb(42, 93, 91);
        style.visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
        style.spacing.item_spacing = Vec2::new(8.0, 7.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, egui::FontId::monospace(12.0));
        cc.egui_ctx.set_style(style);
        let (tx, rx) = worker::start(cc.egui_ctx.clone());
        let mut app = Self {
            renderer: Arc::new(egui::mutex::Mutex::new(renderer)),
            tx,
            rx,
            scene: Arc::new(Scene::default()),
            busy: false,
            started: Instant::now(),
            pending: String::new(),
            after_save: AfterSave::Nothing,
            log: VecDeque::new(),
            last_error: None,
            cameras: [
                Camera::new(ViewKind::Perspective),
                Camera::new(ViewKind::Top),
                Camera::new(ViewKind::Front),
                Camera::new(ViewKind::Right),
            ],
            four: true,
            active: 0,
            fit_pending: true,
            display: DisplayMode::Edges,
            grid: true,
            selected: vec![],
            command: String::new(),
            history: vec![],
            history_index: None,
            focus_command: false,
            project_path: None,
            file_dialog: None,
            replace_dialog: None,
            close_dialog: false,
            allow_close: false,
            help: false,
            help_filter: String::new(),
            layer_name: String::new(),
            rename: String::new(),
            radius: 1.0,
            translation: [10.0, 0.0, 0.0],
            keep_inputs: true,
            drawing: false,
            draw_closed: false,
            draw_points: vec![],
            snap: true,
            snap_step: 1.0,
            telemetry: std::env::var_os("ARCLET_GUI_STATE").map(PathBuf::from),
            last_telemetry: String::new(),
            frames: 0,
            viewport_rects: [Rect::NOTHING; 4],
        };
        app.record("Ready. Click an object to select; Shift-click adds. Right-drag orbits, middle-drag pans.".into(),false);
        app.send_now(if demo { "Demo".into() } else { "List".into() });
        Ok(app)
    }
    fn record(&mut self, text: String, error: bool) {
        let text = if text.chars().count() > 2400 {
            format!("{} ...", text.chars().take(2400).collect::<String>())
        } else {
            text
        };
        self.log.push_back(LogLine { text, error });
        while self.log.len() > 120 {
            self.log.pop_front();
        }
    }
    fn error(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.last_error = Some(text.clone());
        self.record(text, true);
    }
    fn send_now(&mut self, line: String) {
        if self.busy {
            self.error("A geometry operation is already running. The camera remains available.");
            return;
        }
        match self.tx.try_send(Request::Command(line.clone())) {
            Ok(()) => {
                self.busy = true;
                self.started = Instant::now();
                self.pending = line.clone();
                self.last_error = None;
                self.record(format!("> {line}"), false);
                if self.history.last() != Some(&line) {
                    self.history.push(line);
                    if self.history.len() > 128 {
                        self.history.remove(0);
                    }
                }
                self.history_index = None;
            }
            Err(e) => self.error(format!("Kernel worker unavailable: {e}")),
        }
    }
    fn request(&mut self, line: String) {
        let verb = line
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_start_matches('_')
            .to_ascii_lowercase();
        if self.scene.dirty && matches!(verb.as_str(), "new" | "open") {
            self.replace_dialog = Some(line);
        } else {
            self.send_now(line);
        }
    }
    fn batch(&mut self, lines: Vec<String>) {
        if lines.is_empty() {
            self.error("Select one or more objects first.");
            return;
        }
        if self.busy {
            return;
        }
        match self.tx.try_send(Request::Batch(lines.clone())) {
            Ok(()) => {
                self.busy = true;
                self.started = Instant::now();
                self.pending = "Batch".into();
                self.last_error = None;
                self.record(format!("> {}", lines.join(" ; ")), false);
            }
            Err(e) => self.error(format!("Kernel worker unavailable: {e}")),
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        loop {
            match self.rx.try_recv() {
                Ok(event) => {
                    self.busy = false;
                    let first = self.scene.objects.is_empty() && !event.scene.objects.is_empty();
                    self.scene = event.scene;
                    self.selected
                        .retain(|id| self.scene.objects.iter().any(|o| o.id == *id));
                    match event.result {
                        Ok(outcome) => {
                            if !outcome.created.is_empty() {
                                self.selected = outcome.created;
                            }
                            let tokens =
                                crate::command::tokenize(&self.pending).unwrap_or_default();
                            let verb = tokens
                                .first()
                                .map(|s| s.trim_start_matches('_').to_ascii_lowercase())
                                .unwrap_or_default();
                            if verb == "save" || verb == "open" {
                                self.project_path = tokens.get(1).cloned();
                            }
                            if verb == "new" {
                                self.project_path = None;
                                self.selected.clear();
                            }
                            if first
                                || matches!(verb.as_str(), "demo" | "open" | "new" | "importstep")
                            {
                                self.fit_pending = true;
                            }
                            if verb != "list" {
                                self.record(
                                    format!("{}  [{} ms]", outcome.message, event.elapsed_ms),
                                    false,
                                );
                            }
                            if verb == "save" {
                                match std::mem::replace(&mut self.after_save, AfterSave::Nothing) {
                                    AfterSave::Close => {
                                        self.allow_close = true;
                                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                    }
                                    AfterSave::Command(command) => self.send_now(command),
                                    AfterSave::Nothing => {}
                                }
                            }
                        }
                        Err(e) => {
                            self.after_save = AfterSave::Nothing;
                            self.error(e.to_string());
                        }
                    }
                    for warning in self.scene.warnings.clone() {
                        self.record(warning, true);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if self.busy {
                        self.busy = false;
                        self.error("Kernel worker stopped. Save/export is unavailable; existing on-disk files have not been replaced.");
                    }
                    break;
                }
            }
        }
    }
    fn file(&mut self, verb: &str, after: AfterSave) {
        let default = match verb {
            "Save" => self
                .project_path
                .clone()
                .unwrap_or_else(|| "model.arcocct".into()),
            "ExportStep" => "model.step".into(),
            "ExportStl" => "model.stl".into(),
            _ => String::new(),
        };
        self.file_dialog = Some(FileDialog {
            verb: verb.into(),
            path: default,
            after,
        });
    }
    fn preset(&mut self, verb: &str) {
        let first = self.selected.first().copied();
        let two = self.selected.get(1).copied();
        let command = match verb {
            "Box" => "Box 0,0,0 30 20 10".into(),
            "Sphere" => "Sphere 0,0,0 10".into(),
            "Cylinder" => "Cylinder 0,0,0 8 20".into(),
            "Cone" => "Cone 0,0,0 12 5 24".into(),
            "Torus" => "Torus 0,0,0 15 4".into(),
            "Circle" => "Circle 0,0,0 12".into(),
            "Curve" => "Curve 0,0,0 10,20,0 30,10,0 40,30,0".into(),
            "NurbsCurve" => "NurbsCurve 2 10,0,0,1 10,10,0,0.7071067811865476 0,10,0,1".into(),
            "Polyline" => {
                self.drawing = true;
                self.draw_points.clear();
                return;
            }
            "Loft" => {
                if self.selected.len() < 2 {
                    self.error("Select at least two wire sections in order, then Loft.");
                    return;
                }
                format!(
                    "Loft solid {}",
                    self.selected
                        .iter()
                        .map(u64::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
            "BooleanUnion" | "BooleanDifference" | "BooleanIntersection" | "Section" => {
                let (Some(a), Some(b)) = (first, two) else {
                    self.error("Select two different objects, in target/tool order.");
                    return;
                };
                format!(
                    "{verb} {a} {b}{}",
                    if self.keep_inputs && verb != "Section" {
                        " --keep"
                    } else {
                        ""
                    }
                )
            }
            _ => {
                let Some(id) = first else {
                    self.error("Select an object first.");
                    return;
                };
                match verb {
                    "FilletEdge" => format!("FilletEdge {id} {}", self.radius),
                    "ChamferEdge" => format!("ChamferEdge {id} {}", self.radius),
                    "ExtrudeCrv" => format!("ExtrudeCrv {id} 0,0,20"),
                    "Revolve" => format!("Revolve {id} 0,0,0 0,0,1 360"),
                    "Move" => format!("Move {id} 10,0,0"),
                    "Copy" => format!("Copy {id} 20,0,0"),
                    "Rotate" => format!("Rotate {id} 0,0,0 0,0,1 45"),
                    "Scale" => format!("Scale {id} 0,0,0 1.5"),
                    "Mirror" => format!("Mirror {id} 0,0,0 1,0,0"),
                    "ArrayLinear" => format!("ArrayLinear {id} 5 20,0,0"),
                    "ArrayPolar" => format!("ArrayPolar {id} 6 0,0,0 360"),
                    _ => return,
                }
            }
        };
        self.command = command;
        self.focus_command = true;
    }
    fn menus(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("application-menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("ARCLET").strong().size(19.0).color(ACCENT));
                ui.label(RichText::new("OCCT STUDIO").size(12.0));
                ui.separator();
                ui.label(
                    RichText::new("EXPERIMENTAL")
                        .small()
                        .color(Color32::from_rgb(216, 167, 91)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(if self.scene.dirty {
                        "Modified *"
                    } else {
                        "Saved / unchanged"
                    });
                    if self.busy {
                        ui.spinner();
                        ui.label(format!(
                            "Kernel {:.1}s",
                            self.started.elapsed().as_secs_f32()
                        ));
                    }
                });
            });
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    ui.add_enabled_ui(!self.busy, |ui| {
                        for (label, verb) in [
                            ("Save project...  Ctrl+S", "Save"),
                            ("Open project...  Ctrl+O", "Open"),
                            ("Import STEP...", "ImportStep"),
                            ("Export STEP...", "ExportStep"),
                            ("Export STL...", "ExportStl"),
                        ] {
                            if ui.button(label).clicked() {
                                self.file(verb, AfterSave::Nothing);
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button("New document").clicked() {
                            self.request("New".into());
                            ui.close_menu();
                        }
                    });
                });
                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(
                            !self.busy && self.scene.undo > 0,
                            egui::Button::new("Undo  Ctrl+Z"),
                        )
                        .clicked()
                    {
                        self.send_now("Undo".into());
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            !self.busy && self.scene.redo > 0,
                            egui::Button::new("Redo  Ctrl+Shift+Z"),
                        )
                        .clicked()
                    {
                        self.send_now("Redo".into());
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            !self.busy && !self.selected.is_empty(),
                            egui::Button::new("Delete selected"),
                        )
                        .clicked()
                    {
                        self.delete_selected();
                        ui.close_menu();
                    }
                });
                for (title, verbs) in [
                    (
                        "Create",
                        &["Box", "Sphere", "Cylinder", "Cone", "Torus"][..],
                    ),
                    ("Curve", &["Polyline", "Circle", "Curve", "NurbsCurve"][..]),
                    ("Surface", &["ExtrudeCrv", "Revolve", "Loft", "Section"][..]),
                    (
                        "Solid",
                        &[
                            "BooleanUnion",
                            "BooleanDifference",
                            "BooleanIntersection",
                            "FilletEdge",
                            "ChamferEdge",
                        ][..],
                    ),
                    (
                        "Transform",
                        &[
                            "Move",
                            "Copy",
                            "Rotate",
                            "Scale",
                            "Mirror",
                            "ArrayLinear",
                            "ArrayPolar",
                        ][..],
                    ),
                ] {
                    ui.menu_button(title, |ui| {
                        ui.add_enabled_ui(!self.busy, |ui| {
                            for verb in verbs {
                                if ui.button(*verb).clicked() {
                                    self.preset(verb);
                                    ui.close_menu();
                                }
                            }
                        });
                    });
                }
                ui.menu_button("Help", |ui| {
                    if ui.button("Implemented commands  F1").clicked() {
                        self.help = true;
                        ui.close_menu();
                    }
                    ui.label("Geometry is OCCT B-rep; display is tessellated.");
                    ui.label("No Rhino / Grasshopper / 3DM compatibility is claimed.");
                });
            });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.busy && self.scene.undo > 0, egui::Button::new("Undo"))
                    .clicked()
                {
                    self.send_now("Undo".into());
                }
                if ui
                    .add_enabled(!self.busy && self.scene.redo > 0, egui::Button::new("Redo"))
                    .clicked()
                {
                    self.send_now("Redo".into());
                }
                ui.separator();
                if ui.selectable_label(self.four, "4 views").clicked() {
                    self.four = !self.four;
                    self.fit_pending = true;
                }
                if ui.button("Fit all").clicked() {
                    self.fit_pending = true;
                }
                if ui.button("Fit selection").clicked() {
                    if let Some(bounds) = self.scene.bounds(&self.selected) {
                        for camera in &mut self.cameras {
                            camera.fit(bounds, 1.0);
                        }
                    }
                }
                egui::ComboBox::from_id_salt("display-mode")
                    .selected_text(self.display.name())
                    .show_ui(ui, |ui| {
                        for mode in [
                            DisplayMode::Shaded,
                            DisplayMode::Edges,
                            DisplayMode::Wire,
                            DisplayMode::Zebra,
                        ] {
                            ui.selectable_value(&mut self.display, mode, mode.name());
                        }
                    });
                ui.checkbox(&mut self.grid, "Grid");
                ui.separator();
                ui.label(RichText::new("mm").color(ACCENT));
                if self.scene.objects.is_empty()
                    && ui
                        .add_enabled(!self.busy, egui::Button::new("Build showcase"))
                        .clicked()
                {
                    self.send_now("Demo".into());
                }
            });
        });
    }
    fn delete_selected(&mut self) {
        if !self.selected.is_empty() {
            self.send_now(format!(
                "Delete {}",
                self.selected
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
    }
    fn tools(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("tool-shelf")
            .resizable(false)
            .exact_width(108.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("MODEL").small().color(ACCENT));
                ui.add_enabled_ui(!self.busy, |ui| {
                    for verb in ["Box", "Sphere", "Cylinder", "Circle", "Polyline"] {
                        if ui
                            .add_sized([88.0, 30.0], egui::Button::new(verb))
                            .clicked()
                        {
                            self.preset(verb);
                        }
                    }
                    ui.separator();
                    for verb in ["ExtrudeCrv", "Loft", "FilletEdge", "Move", "Copy"] {
                        if ui
                            .add_sized([88.0, 30.0], egui::Button::new(verb))
                            .clicked()
                        {
                            self.preset(verb);
                        }
                    }
                });
                ui.separator();
                ui.label(RichText::new("NAVIGATE").small().color(ACCENT));
                ui.small("Right drag\nOrbit");
                ui.small("Middle drag\nPan");
                ui.small("Wheel\nZoom");
                ui.small("Shift + click\nAdd selection");
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.small("Exact B-rep\nOCCT backend");
                });
            });
    }
    fn inspector(&mut self, ctx: &egui::Context) {
        let scene = self.scene.clone();
        egui::SidePanel::right("inspector")
            .default_width(265.0)
            .min_width(230.0)
            .max_width(400.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Objects");
                        ui.label(RichText::new(format!("{}", scene.objects.len())).color(ACCENT));
                    });
                    egui::ScrollArea::vertical()
                        .id_salt("objects-list")
                        .max_height(190.0)
                        .show(ui, |ui| {
                            for o in &scene.objects {
                                ui.horizontal(|ui| {
                                    let selected = self.selected.contains(&o.id);
                                    let text = format!("#{}  {}", o.id, o.name);
                                    if ui
                                        .selectable_label(selected, text)
                                        .on_hover_text(format!(
                                            "Layer #{} | {} faces | {}",
                                            o.layer,
                                            o.stats.faces,
                                            if o.locked { "locked" } else { "editable" }
                                        ))
                                        .clicked()
                                    {
                                        if ctx.input(|i| i.modifiers.shift) {
                                            if selected {
                                                self.selected.retain(|id| *id != o.id);
                                            } else {
                                                self.selected.push(o.id);
                                            }
                                        } else {
                                            self.selected = vec![o.id];
                                        }
                                        self.rename = o.name.clone();
                                    }
                                    if !o.visible {
                                        ui.label(RichText::new("H").small());
                                    }
                                    if o.locked {
                                        ui.label(RichText::new("L").small().color(Color32::YELLOW));
                                    }
                                });
                            }
                        });
                    ui.add_enabled_ui(!self.busy && !self.selected.is_empty(), |ui| {
                        ui.horizontal(|ui| {
                            for verb in ["Hide", "Show", "Lock", "Unlock"] {
                                if ui.small_button(verb).clicked() {
                                    self.send_now(format!(
                                        "{verb} {}",
                                        self.selected
                                            .iter()
                                            .map(u64::to_string)
                                            .collect::<Vec<_>>()
                                            .join(" ")
                                    ));
                                }
                            }
                        });
                    });
                    ui.separator();
                    egui::CollapsingHeader::new("Layers")
                        .default_open(true)
                        .show(ui, |ui| {
                            for layer in &scene.layers {
                                ui.horizontal(|ui| {
                                    let mut visible = layer.visible;
                                    let mut locked = layer.locked;
                                    let mut color = layer.color;
                                    ui.add_enabled_ui(!self.busy, |ui| {
                                        if ui
                                            .checkbox(&mut visible, "")
                                            .on_hover_text("Layer visibility")
                                            .changed()
                                        {
                                            self.send_now(format!(
                                                "LayerVisible {} {}",
                                                layer.id, visible
                                            ));
                                        }
                                        if ui
                                            .checkbox(&mut locked, "")
                                            .on_hover_text("Layer geometry lock")
                                            .changed()
                                        {
                                            self.send_now(format!(
                                                "LayerLock {} {}",
                                                layer.id, locked
                                            ));
                                        }
                                        if ui.color_edit_button_srgb(&mut color).changed() {
                                            self.send_now(format!(
                                                "LayerColor {} {} {} {}",
                                                layer.id, color[0], color[1], color[2]
                                            ));
                                        }
                                        if ui
                                            .selectable_label(
                                                layer.id == scene.current_layer,
                                                format!("{}  #{}", layer.name, layer.id),
                                            )
                                            .on_hover_text(
                                                "Make this the current construction layer",
                                            )
                                            .clicked()
                                        {
                                            self.send_now(format!("CurrentLayer {}", layer.id));
                                        }
                                    });
                                });
                            }
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.layer_name)
                                        .hint_text("New layer")
                                        .desired_width(155.0),
                                );
                                if ui
                                    .add_enabled(
                                        !self.busy && !self.layer_name.trim().is_empty(),
                                        egui::Button::new("Add"),
                                    )
                                    .clicked()
                                {
                                    self.send_now(format!("Layer {}", quote(&self.layer_name)));
                                    self.layer_name.clear();
                                }
                            });
                            if !self.selected.is_empty()
                                && ui
                                    .add_enabled(
                                        !self.busy,
                                        egui::Button::new("Assign selection to current layer"),
                                    )
                                    .clicked()
                            {
                                self.batch(
                                    self.selected
                                        .iter()
                                        .map(|id| format!("SetLayer {id} {}", scene.current_layer))
                                        .collect(),
                                );
                            }
                        });
                    ui.separator();
                    ui.heading("Selection");
                    if self.selected.is_empty() {
                        ui.weak("Select a solid, surface or curve.");
                    } else {
                        ui.label(
                            self.selected
                                .iter()
                                .map(|id| format!("#{id}"))
                                .collect::<Vec<_>>()
                                .join("  "),
                        );
                        let objects = scene
                            .objects
                            .iter()
                            .filter(|o| self.selected.contains(&o.id))
                            .collect::<Vec<_>>();
                        let volume: f64 = objects.iter().map(|o| o.stats.volume).sum();
                        let area: f64 = objects.iter().map(|o| o.stats.area).sum();
                        ui.label(format!("Volume  {volume:.4} mm^3"));
                        ui.label(format!("Area       {area:.4} mm^2"));
                        if let Some(bounds) = scene.bounds(&self.selected) {
                            ui.label(format!(
                                "Size  {:.3} x {:.3} x {:.3} mm",
                                bounds[3] - bounds[0],
                                bounds[4] - bounds[1],
                                bounds[5] - bounds[2]
                            ));
                        }
                        if objects.len() == 1 {
                            let o = objects[0];
                            ui.small(format!(
                                "{} faces / {} edges / {} solids",
                                o.stats.faces, o.stats.edges, o.stats.solids
                            ));
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.rename)
                                        .hint_text(&o.name)
                                        .desired_width(160.0),
                                );
                                if ui
                                    .add_enabled(
                                        !self.busy && !self.rename.trim().is_empty(),
                                        egui::Button::new("Rename"),
                                    )
                                    .clicked()
                                {
                                    self.send_now(format!(
                                        "Rename {} {}",
                                        o.id,
                                        quote(&self.rename)
                                    ));
                                }
                            });
                        }
                        ui.add_space(6.0);
                        ui.add_enabled_ui(!self.busy, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Radius");
                                ui.add(
                                    egui::DragValue::new(&mut self.radius)
                                        .range(0.0001..=1e7)
                                        .speed(0.1),
                                );
                            });
                            ui.horizontal(|ui| {
                                for verb in ["FilletEdge", "ChamferEdge"] {
                                    if ui.button(verb).clicked() {
                                        self.batch(
                                            self.selected
                                                .iter()
                                                .map(|id| format!("{verb} {id} {}", self.radius))
                                                .collect(),
                                        );
                                    }
                                }
                            });
                            ui.small(
                                "Applies to all edges. Use the command line for edge indices.",
                            );
                            ui.horizontal(|ui| {
                                for (i, name) in ["X", "Y", "Z"].iter().enumerate() {
                                    ui.label(*name);
                                    ui.add(
                                        egui::DragValue::new(&mut self.translation[i])
                                            .speed(0.25)
                                            .range(-1e8..=1e8),
                                    );
                                }
                            });
                            ui.horizontal(|ui| {
                                for verb in ["Move", "Copy"] {
                                    if ui.button(verb).clicked() {
                                        let v = self.translation;
                                        self.batch(
                                            self.selected
                                                .iter()
                                                .map(|id| {
                                                    format!(
                                                        "{verb} {id} {},{},{}",
                                                        v[0], v[1], v[2]
                                                    )
                                                })
                                                .collect(),
                                        );
                                    }
                                }
                            });
                            if self.selected.len() == 2 {
                                ui.separator();
                                ui.small("Boolean target/tool order follows selection order.");
                                ui.checkbox(&mut self.keep_inputs, "Keep inputs");
                                ui.horizontal_wrapped(|ui| {
                                    for (label, verb) in [
                                        ("Union", "BooleanUnion"),
                                        ("Subtract", "BooleanDifference"),
                                        ("Intersect", "BooleanIntersection"),
                                    ] {
                                        if ui.button(label).clicked() {
                                            self.send_now(format!(
                                                "{verb} {} {}{}",
                                                self.selected[0],
                                                self.selected[1],
                                                if self.keep_inputs { " --keep" } else { "" }
                                            ));
                                        }
                                    }
                                });
                            }
                        });
                    }
                });
            });
    }
    fn command_dock(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(28.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if self.busy { "COMPUTING" } else { "READY" }).color(ACCENT),
                    );
                    ui.separator();
                    ui.small(format!(
                        "{} objects  |  {} display triangles  |  revision {}",
                        self.scene.objects.len(),
                        self.scene.triangles(),
                        self.scene.revision
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.small("Development build  /  Save important work regularly");
                    });
                });
            });
        egui::TopBottomPanel::bottom("command-dock")
            .resizable(true)
            .default_height(145.0)
            .min_height(100.0)
            .show(ctx, |ui| {
                let height = (ui.available_height() - 55.0).max(30.0);
                egui::ScrollArea::vertical()
                    .id_salt("command-log")
                    .max_height(height)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for entry in &self.log {
                            ui.label(RichText::new(&entry.text).monospace().size(11.5).color(
                                if entry.error {
                                    Color32::from_rgb(249, 151, 129)
                                } else {
                                    Color32::from_rgb(165, 180, 193)
                                },
                            ));
                        }
                    });
                if let Some(error) = &self.last_error {
                    ui.label(
                        RichText::new(error)
                            .small()
                            .color(Color32::from_rgb(249, 151, 129)),
                    );
                }
                let mut submit = false;
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Command >").color(ACCENT).monospace());
                    let input = ui.add_enabled(
                        !self.busy,
                        egui::TextEdit::singleline(&mut self.command)
                            .id_salt("command-entry")
                            .font(egui::TextStyle::Monospace)
                            .desired_width((ui.available_width() - 65.0).max(100.0))
                            .hint_text("Box 0,0,0 30 20 10   |   Ctrl+L focuses here"),
                    );
                    if self.focus_command {
                        input.request_focus();
                        self.focus_command = false;
                    }
                    if input.has_focus() {
                        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) && !self.history.is_empty() {
                            let index = self
                                .history_index
                                .unwrap_or(self.history.len())
                                .saturating_sub(1);
                            self.command = self.history[index].clone();
                            self.history_index = Some(index);
                        }
                        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                            if let Some(index) = self.history_index {
                                if index + 1 < self.history.len() {
                                    self.command = self.history[index + 1].clone();
                                    self.history_index = Some(index + 1);
                                } else {
                                    self.command.clear();
                                    self.history_index = None;
                                }
                            }
                        }
                    }
                    if input.lost_focus() && ctx.input(|i| i.key_pressed(Key::Enter)) {
                        submit = true;
                        input.request_focus();
                    }
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("Run"))
                        .clicked()
                    {
                        submit = true;
                    }
                });
                let word = self.command.trim().to_ascii_lowercase();
                if !word.is_empty() && !word.contains(' ') {
                    ui.horizontal_wrapped(|ui| {
                        for item in HELP
                            .iter()
                            .filter(|h| h.0.to_ascii_lowercase().starts_with(&word))
                            .take(6)
                        {
                            if ui.small_button(item.0).on_hover_text(item.1).clicked() {
                                self.command = format!("{} ", item.0);
                                self.focus_command = true;
                            }
                        }
                    });
                }
                if submit {
                    let line = if self.command.trim().is_empty() {
                        self.history.last().cloned().unwrap_or_default()
                    } else {
                        self.command.trim().to_owned()
                    };
                    if !line.is_empty() {
                        self.command.clear();
                        self.request(line);
                    }
                }
            });
    }
    fn fit_views(&mut self, aspect: f64) {
        if let Some(bounds) = self.scene.bounds(&[]) {
            for camera in &mut self.cameras {
                camera.fit(bounds, aspect);
            }
        }
        self.fit_pending = false;
    }
    fn pick(&self, index: usize, rect: Rect, pos: Pos2) -> Option<u64> {
        let camera = &self.cameras[index];
        let aspect = rect.width() as f64 / rect.height() as f64;
        let x = 2.0 * ((pos.x - rect.left()) / rect.width()) as f64 - 1.0;
        let y = 1.0 - 2.0 * ((pos.y - rect.top()) / rect.height()) as f64;
        let (origin, direction) = camera.ray(x, y, aspect)?;
        let mut best = None;
        let mut depth = f64::INFINITY;
        for o in self.scene.objects.iter().filter(|o| o.visible) {
            let Some(mesh) = &o.mesh else {
                continue;
            };
            if !mesh.indices.is_empty() && ray_box(origin, direction, o.stats.bounds) {
                for triangle in mesh.indices.chunks_exact(3) {
                    let a = DVec3::from_array(mesh.positions[triangle[0] as usize]);
                    let b = DVec3::from_array(mesh.positions[triangle[1] as usize]);
                    let c = DVec3::from_array(mesh.positions[triangle[2] as usize]);
                    if let Some(distance) = ray_triangle(origin, direction, a, b, c) {
                        if distance < depth {
                            depth = distance;
                            best = Some(o.id);
                        }
                    }
                }
            } else if mesh.indices.is_empty() {
                for line in &mesh.lines {
                    let (Some(a), Some(b)) = (
                        camera.project(DVec3::from_array(line[0]), aspect),
                        camera.project(DVec3::from_array(line[1]), aspect),
                    ) else {
                        continue;
                    };
                    let a = Vec2::new(
                        a.x as f32 * rect.width() * 0.5,
                        -a.y as f32 * rect.height() * 0.5,
                    );
                    let b = Vec2::new(
                        b.x as f32 * rect.width() * 0.5,
                        -b.y as f32 * rect.height() * 0.5,
                    );
                    let p = pos - rect.center();
                    let d = b - a;
                    let t = if d.length_sq() > 1e-12 {
                        ((p - a).dot(d) / d.length_sq()).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    if (p - (a + d * t)).length() < 7.0 {
                        let point =
                            DVec3::from_array(line[0]).lerp(DVec3::from_array(line[1]), t as f64);
                        let distance = (point - origin).dot(direction);
                        if distance >= 0.0 && distance < depth {
                            depth = distance;
                            best = Some(o.id);
                        }
                    }
                }
            }
        }
        best
    }
    fn finish_drawing(&mut self) {
        if self.draw_points.len() < 2 || (self.draw_closed && self.draw_points.len() < 3) {
            self.error("A polyline needs at least two points, or three for a closed loop.");
            return;
        }
        let points = self
            .draw_points
            .iter()
            .map(|p| format!("{:.9},{:.9},{:.9}", p.x, p.y, p.z))
            .collect::<Vec<_>>()
            .join(" ");
        self.send_now(format!(
            "Polyline {}{points}",
            if self.draw_closed { "closed " } else { "" }
        ));
        self.drawing = false;
        self.draw_points.clear();
    }
    fn viewport(&mut self, ui: &mut egui::Ui, index: usize, rect: Rect) {
        self.viewport_rects[index] = rect;
        let response = ui.interact(
            rect,
            ui.id().with(("viewport", index)),
            egui::Sense::click_and_drag(),
        );
        if response.hovered() {
            let delta = ui.input(|i| i.pointer.delta());
            let pan = response.dragged_by(PointerButton::Middle)
                || (response.dragged_by(PointerButton::Secondary)
                    && ui.input(|i| i.modifiers.shift));
            if pan {
                self.cameras[index].pan(delta.x as f64, delta.y as f64, rect.height() as f64);
            } else if response.dragged_by(PointerButton::Secondary) {
                self.cameras[index].orbit(delta.x as f64, delta.y as f64);
            }
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.cameras[index].zoom(scroll as f64);
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            self.active = index;
            if let Some(pos) = response.interact_pointer_pos() {
                if self.drawing && !self.busy {
                    let x = 2.0 * ((pos.x - rect.left()) / rect.width()) as f64 - 1.0;
                    let y = 1.0 - 2.0 * ((pos.y - rect.top()) / rect.height()) as f64;
                    if let Some(mut point) = self.cameras[index].construction_point(
                        x,
                        y,
                        rect.width() as f64 / rect.height() as f64,
                    ) {
                        if self.snap {
                            point = (point / self.snap_step).round() * self.snap_step;
                        }
                        if self
                            .draw_points
                            .last()
                            .is_none_or(|p| p.distance(point) > 1e-7)
                            && self.draw_points.len() < 256
                        {
                            self.draw_points.push(point);
                        }
                    }
                } else {
                    let hit = self.pick(index, rect, pos);
                    if ui.input(|i| i.modifiers.shift) {
                        if let Some(id) = hit {
                            if self.selected.contains(&id) {
                                self.selected.retain(|old| *old != id);
                            } else {
                                self.selected.push(id);
                            }
                        }
                    } else {
                        self.selected = hit.into_iter().collect();
                    }
                    if let Some(id) = self.selected.first() {
                        if let Some(o) = self.scene.objects.iter().find(|o| o.id == *id) {
                            self.rename = o.name.clone();
                        }
                    }
                }
            }
        }
        if response.double_clicked_by(PointerButton::Primary) {
            if self.drawing {
                self.finish_drawing();
            } else if response
                .interact_pointer_pos()
                .is_some_and(|p| p.y < rect.top() + 28.0)
            {
                self.active = index;
                self.four = !self.four;
                self.fit_pending = true;
            }
        }
        let renderer = self.renderer.clone();
        let scene = self.scene.clone();
        let camera = self.cameras[index].clone();
        let selected = self.selected.clone();
        let mode = self.display;
        let grid = self.grid;
        ui.painter().add(egui::PaintCallback {
            rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
                renderer.lock().paint(
                    painter.gl(),
                    info,
                    &scene,
                    &camera,
                    PaintOptions {
                        selected: &selected,
                        display: mode,
                        grid,
                    },
                );
            })),
        });
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_stroke(
            rect.shrink(1.0),
            0.0,
            Stroke::new(
                if self.active == index {
                    1.5_f32
                } else {
                    1.0_f32
                },
                if self.active == index {
                    ACCENT
                } else {
                    Color32::from_rgb(54, 63, 74)
                },
            ),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.left_top() + Vec2::new(12.0, 11.0),
            egui::Align2::LEFT_TOP,
            self.cameras[index].kind.name(),
            egui::FontId::proportional(13.0),
            Color32::from_rgb(190, 208, 217),
        );
        painter.text(
            rect.right_bottom() - Vec2::new(12.0, 10.0),
            egui::Align2::RIGHT_BOTTOM,
            match self.cameras[index].kind {
                ViewKind::Top => "XY",
                ViewKind::Front => "XZ",
                ViewKind::Right => "YZ",
                ViewKind::Perspective => "Z up",
            },
            egui::FontId::monospace(10.0),
            Color32::from_rgb(123, 146, 161),
        );
        if self.drawing {
            let camera = &self.cameras[index];
            let aspect = rect.width() as f64 / rect.height() as f64;
            let project = |p: DVec3| {
                camera.project(p, aspect).map(|p| {
                    Pos2::new(
                        rect.center().x + p.x as f32 * rect.width() * 0.5,
                        rect.center().y - p.y as f32 * rect.height() * 0.5,
                    )
                })
            };
            for segment in self.draw_points.windows(2) {
                if let (Some(a), Some(b)) = (project(segment[0]), project(segment[1])) {
                    painter.line_segment([a, b], Stroke::new(2.0_f32, ACCENT));
                }
            }
            if self.draw_closed && self.draw_points.len() > 2 {
                if let (Some(a), Some(b)) = (
                    self.draw_points.first().and_then(|p| project(*p)),
                    self.draw_points.last().and_then(|p| project(*p)),
                ) {
                    painter.line_segment([a, b], Stroke::new(1.0_f32, ACCENT));
                }
            }
            for p in &self.draw_points {
                if let Some(p) = project(*p) {
                    painter.circle_filled(p, 3.5, ACCENT);
                }
            }
        }
        if self.scene.objects.is_empty() && !self.busy {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Create a primitive, draw a polyline,\nor choose Build showcase.",
                egui::FontId::proportional(15.0),
                Color32::from_rgb(140, 159, 174),
            );
        }
    }
    fn canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                if self.drawing {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("DRAW POLYLINE").color(ACCENT));
                        ui.label(format!("{} points", self.draw_points.len()));
                        ui.checkbox(&mut self.draw_closed, "Closed");
                        ui.checkbox(&mut self.snap, "Grid snap");
                        ui.add(
                            egui::DragValue::new(&mut self.snap_step)
                                .range(0.0001..=1e6)
                                .speed(0.1)
                                .suffix(" mm"),
                        );
                        if ui.button("Finish / Enter").clicked() {
                            self.finish_drawing();
                        }
                        if ui.button("Cancel / Esc").clicked() {
                            self.drawing = false;
                            self.draw_points.clear();
                        }
                    });
                }
                let rect = ui.available_rect_before_wrap().shrink(3.0);
                if rect.width() < 10.0 || rect.height() < 10.0 {
                    return;
                }
                if self.fit_pending {
                    self.fit_views(rect.width() as f64 / rect.height() as f64);
                }
                if self.four {
                    let gap = 3.0;
                    let size = Vec2::new((rect.width() - gap) * 0.5, (rect.height() - gap) * 0.5);
                    for (position, index) in [1, 0, 2, 3].into_iter().enumerate() {
                        let origin = rect.min
                            + Vec2::new(
                                (position % 2) as f32 * (size.x + gap),
                                (position / 2) as f32 * (size.y + gap),
                            );
                        self.viewport(ui, index, Rect::from_min_size(origin, size));
                    }
                } else {
                    self.viewport(ui, self.active, rect);
                }
            });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.help {
            let mut open = true;
            egui::Window::new("Command reference").open(&mut open).default_width(650.0).default_height(520.0).show(ctx,|ui|{
            ui.label("These commands are implemented. The kernel reports failures; unsupported operations are not simulated.");ui.text_edit_singleline(&mut self.help_filter);let filter=self.help_filter.to_ascii_lowercase();
            egui::ScrollArea::vertical().show(ui,|ui|{for h in HELP.iter().filter(|h|h.0.to_ascii_lowercase().contains(&filter)||h.2.to_ascii_lowercase().contains(&filter)){ui.label(RichText::new(h.1).monospace().color(ACCENT));ui.small(h.2);ui.add_space(7.0);}});
        });
            self.help = open;
        }
        if let Some(mut dialog) = self.file_dialog.take() {
            let mut open = true;
            let mut submit = false;
            egui::Window::new(format!("{} - file path", dialog.verb))
                .id(egui::Id::new("file-path-dialog"))
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_width(560.0)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(
                        "Enter a full path or a path relative to the current working directory.",
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut dialog.path).desired_width(f32::INFINITY),
                    );
                    ui.small(format!(
                        "Current directory: {}",
                        std::env::current_dir()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    ));
                    if matches!(dialog.verb.as_str(), "Save" | "ExportStep" | "ExportStl")
                        && PathBuf::from(&dialog.path).exists()
                    {
                        ui.colored_label(
                            Color32::from_rgb(235, 184, 103),
                            "This existing file will be replaced only after a successful write.",
                        );
                    }
                    if ui
                        .add_enabled(
                            !self.busy && !dialog.path.trim().is_empty(),
                            egui::Button::new(&dialog.verb),
                        )
                        .clicked()
                    {
                        submit = true;
                    }
                });
            if submit {
                let line = format!("{} {}", dialog.verb, quote(dialog.path.trim()));
                if dialog.verb == "Save" {
                    self.after_save = dialog.after;
                    self.send_now(line);
                } else {
                    self.request(line);
                }
            } else if open {
                self.file_dialog = Some(dialog);
            }
        }
        if let Some(command) = self.replace_dialog.clone() {
            let mut choice = 0;
            egui::Window::new("Unsaved project")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("The current document has unsaved changes.");
                    ui.label("Save it before replacing the document?");
                    ui.horizontal(|ui| {
                        if ui.button("Save first").clicked() {
                            choice = 1;
                        }
                        if ui.button("Replace without saving").clicked() {
                            choice = 2;
                        }
                        if ui.button("Cancel").clicked() {
                            choice = 3;
                        }
                    });
                });
            match choice {
                1 => {
                    self.replace_dialog = None;
                    self.file("Save", AfterSave::Command(command));
                }
                2 => {
                    self.replace_dialog = None;
                    self.send_now(command);
                }
                3 => self.replace_dialog = None,
                _ => {}
            }
        }
        if self.close_dialog && self.file_dialog.is_none() {
            let mut choice = 0;
            egui::Window::new("Close Arclet OCCT Studio?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    if self.busy {
                        ui.label(
                            "A kernel operation is running. Closing is blocked until it finishes.",
                        );
                        ui.label(
                            "This prototype cannot forcibly cancel a native OCCT operation safely.",
                        );
                    } else {
                        ui.label("The document contains unsaved changes.");
                        ui.horizontal(|ui| {
                            if ui.button("Save and close").clicked() {
                                choice = 1;
                            }
                            if ui.button("Discard and close").clicked() {
                                choice = 2;
                            }
                        });
                    }
                    if ui.button("Keep working").clicked() {
                        choice = 3;
                    }
                });
            match choice {
                1 => self.file("Save", AfterSave::Close),
                2 => {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                3 => self.close_dialog = false,
                _ => {}
            }
        }
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(Key::Escape))
            && (self.file_dialog.is_some() || self.replace_dialog.is_some() || self.close_dialog)
        {
            self.file_dialog = None;
            self.replace_dialog = None;
            self.close_dialog = false;
            return;
        }
        if self.file_dialog.is_some() || self.replace_dialog.is_some() || self.close_dialog {
            return;
        }
        if ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                Key::L,
            ))
        }) {
            self.focus_command = true;
        }
        if !self.busy
            && ctx.input_mut(|i| {
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    Key::S,
                ))
            })
        {
            self.file("Save", AfterSave::Nothing);
        }
        if !self.busy
            && ctx.input_mut(|i| {
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    Key::O,
                ))
            })
        {
            self.file("Open", AfterSave::Nothing);
        }
        if ctx.input(|i| i.key_pressed(Key::F1)) {
            self.help = true;
        }
        if self.drawing && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.drawing = false;
            self.draw_points.clear();
        }
        if !ctx.wants_keyboard_input() {
            if self.drawing && ctx.input(|i| i.key_pressed(Key::Enter)) {
                self.finish_drawing();
            }
            if ctx.input(|i| i.key_pressed(Key::F) || i.key_pressed(Key::Home)) {
                self.fit_pending = true;
            }
            if !self.busy && ctx.input(|i| i.key_pressed(Key::Delete)) {
                self.delete_selected();
            }
            if !self.busy {
                if ctx.input_mut(|i| {
                    i.consume_shortcut(&egui::KeyboardShortcut::new(
                        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                        Key::Z,
                    ))
                }) {
                    self.send_now("Redo".into());
                } else if ctx.input_mut(|i| {
                    i.consume_shortcut(&egui::KeyboardShortcut::new(
                        egui::Modifiers::COMMAND,
                        Key::Z,
                    ))
                }) {
                    self.send_now("Undo".into());
                }
            }
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !self.busy {
            if let Some(path) = dropped.first().and_then(|f| f.path.as_ref()) {
                let extension = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let verb = match extension.as_str() {
                    "arcocct" => Some("Open"),
                    "step" | "stp" => Some("ImportStep"),
                    _ => None,
                };
                if let Some(verb) = verb {
                    self.request(format!("{verb} {}", quote(&path.to_string_lossy())));
                } else {
                    self.error("Drop an .arcocct project or a .step/.stp file.");
                }
            }
        }
    }
    fn telemetry(&mut self, ctx: &egui::Context) {
        let Some(path) = &self.telemetry else {
            return;
        };
        // Explicit opt-in local UI-test hook, never a network service or file poller.
        let json = serde_json::json!({"ready":self.frames>2&&!self.busy,"busy":self.busy,"dirty":self.scene.dirty,"revision":self.scene.revision,"command":self.pending,"error":self.last_error,"render_error":self.renderer.lock().error,"four_views":self.four,"view":self.cameras[self.active].kind.name(),"selected":self.selected,"triangles":self.scene.triangles(),"close_guard":self.close_dialog,"replace_guard":self.replace_dialog.is_some(),"pixels_per_point":ctx.pixels_per_point(),"viewports":self.viewport_rects.iter().map(|r|[r.min.x,r.min.y,r.max.x,r.max.y]).collect::<Vec<_>>(),"cameras":self.cameras.iter().map(|c|serde_json::json!({"target":c.target.to_array(),"eye":c.eye().to_array(),"span":c.span})).collect::<Vec<_>>(),"file_dialog":self.file_dialog.is_some(),"objects":self.scene.objects.iter().map(|o|serde_json::json!({"id":o.id,"name":o.name,"visible":o.visible,"locked":o.locked,"layer":o.layer,"stats":o.stats})).collect::<Vec<_>>()});
        if let Ok(text) = serde_json::to_string_pretty(&json) {
            if text != self.last_telemetry && std::fs::write(path, &text).is_ok() {
                self.last_telemetry = text;
            }
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames = self.frames.saturating_add(1);
        self.poll(ctx);
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && (self.scene.dirty || self.busy)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_dialog = true;
        }
        self.shortcuts(ctx);
        self.menus(ctx);
        self.command_dock(ctx);
        self.tools(ctx);
        self.inspector(ctx);
        self.canvas(ctx);
        self.dialogs(ctx);
        let error = self.renderer.lock().error.clone();
        if let Some(error) = error {
            egui::Window::new("Display error").show(ctx, |ui| {
                ui.colored_label(Color32::RED, error);
                ui.label(
                    "Exact geometry is still retained in the document. Try saving and reopening.",
                );
            });
        }
        self.telemetry(ctx);
        if self.busy || self.frames < 5 {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            self.renderer.lock().destroy(gl);
        }
    }
}
pub fn run(demo: bool) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 960.0])
            .with_min_inner_size([980.0, 680.0]),
        renderer: eframe::Renderer::Glow,
        depth_buffer: 24,
        multisampling: 0,
        ..Default::default()
    };
    eframe::run_native(
        "Arclet OCCT Studio",
        options,
        Box::new(move |cc| {
            App::new(cc, demo)
                .map(|app| Box::new(app) as Box<dyn eframe::App>)
                .map_err(|e| {
                    Box::new(std::io::Error::other(e.to_string()))
                        as Box<dyn std::error::Error + Send + Sync>
                })
        }),
    )
    .map_err(|e| Error::new(e.to_string()))
}
