//! Arclet's compact shell: familiar named tools, a command line, and the node workspace.
//! All geometry and model navigation use the shared Rust engine and its one viewport path.

use std::sync::mpsc::{Receiver, Sender};

use egui::{RichText, Ui, vec2};
use serde_json::{Value, json};
use solidcraft_engine::{Session, find_command};
use solidcraft_ui_egui::control::ControlRequest;
use solidcraft_ui_egui::i18n::tr;
use solidcraft_ui_egui::theme::Tokens;
use solidcraft_ui_egui::toolbar::FileAction;
use solidcraft_ui_egui::ui_kit::{self, Container, Edge, Surface};
use solidcraft_ui_egui::{Services, SolveApp};

use crate::{classic_look, cmdline, commands, platform};

const PREFS_KEY: &str = "arclet.workspace.v1";
const CURVES: &[(&str, &str)] = &[
    ("arclet.line", "sketch.line"),
    ("arclet.circle", "sketch.circle.center"),
    ("arclet.rectangle", "sketch.rectangle.two_point"),
    ("arclet.spline", "sketch.spline.control_point"),
];
const CURVE_TOOLS: &[(&str, &str)] = &[
    ("arclet.spline_fit", "sketch.spline.fit_point"),
    ("arclet.spline_five", "sketch.spline.control_point_5"),
    ("arclet.blend", "sketch.blend_curve"),
    ("arclet.rebuild", "sketch.spline.rebuild"),
    ("arclet.curve_join", "inspect.curve_continuity"),
];
const SURFACES: &[(&str, &str)] = &[
    ("arclet.loft", "surface.loft"),
    ("arclet.sweep", "surface.sweep"),
    ("arclet.patch", "surface.patch"),
    ("arclet.revolve", "surface.revolve"),
    ("arclet.blend", "surface.blend"),
    ("arclet.boundary", "surface.boundary"),
    ("arclet.match", "surface.match"),
    ("arclet.fair", "surface.fair"),
    ("arclet.offset", "surface.offset"),
    ("arclet.trim", "surface.trim"),
    ("arclet.stitch", "surface.stitch"),
    ("arclet.thicken", "surface.thicken"),
];
const TRANSFORMS: &[(&str, &str)] = &[("arclet.move_rotate", "solid.move"), ("arclet.scale", "solid.scale"), ("arclet.mirror", "solid.mirror")];
const ANALYSIS: &[(&str, &str)] = &[
    ("arclet.zebra", "inspect.zebra"),
    ("arclet.environment", "inspect.environment_map"),
    ("arclet.curvature", "inspect.curvature_map"),
    ("arclet.comb", "inspect.curvature_comb"),
    ("arclet.continuity", "inspect.continuity"),
    ("arclet.measure", "inspect.measure"),
];
const VIEWS: &[(&str, &str)] = &[
    ("arclet.fit", "view.fit"),
    ("arclet.iso", "view.isometric"),
    ("arclet.top", "view.top"),
    ("arclet.front", "view.front"),
    ("arclet.right", "view.right"),
];
const SOLIDS: &[(&str, &str)] = &[
    ("arclet.box", "solid.box"),
    ("arclet.cylinder", "solid.cylinder"),
    ("arclet.sphere", "solid.sphere"),
    ("arclet.extrude", "solid.extrude"),
    ("arclet.revolve", "solid.revolve"),
    ("arclet.loft", "solid.loft"),
    ("arclet.sweep", "solid.sweep"),
    ("arclet.fillet", "solid.fillet"),
    ("arclet.chamfer", "solid.chamfer"),
    ("arclet.combine", "solid.combine"),
    ("arclet.press_pull", "solid.press_pull"),
    ("arclet.shell", "solid.shell"),
];
const CURVE_SHAPES: &[(&str, &str)] = &[
    ("arclet.arc_three_point", "sketch.arc.three_point"),
    ("arclet.arc_center", "sketch.arc.center_point"),
    ("arclet.arc_tangent", "sketch.arc.tangent"),
    ("arclet.polygon", "sketch.polygon.inscribed"),
    ("arclet.ellipse", "sketch.ellipse"),
];
const CURVE_EDITS: &[(&str, &str)] = &[
    ("arclet.dimension", "sketch.dimension"),
    ("arclet.fillet", "sketch.fillet"),
    ("arclet.chamfer", "sketch.chamfer.equal_distance"),
    ("arclet.mirror", "sketch.mirror"),
    ("arclet.pattern_rect", "sketch.pattern.rectangular"),
    ("arclet.pattern_circular", "sketch.pattern.circular"),
];
const SURFACE_MORE: &[(&str, &str)] = &[
    ("arclet.ruled", "surface.ruled"),
    ("arclet.unstitch", "surface.unstitch"),
    ("arclet.untrim", "surface.untrim"),
    ("arclet.extend", "surface.extend"),
];
const SOLID_MORE: &[(&str, &str)] = &[
    ("arclet.cone", "solid.cone"),
    ("arclet.torus", "solid.torus"),
    ("arclet.pipe", "solid.pipe"),
    ("arclet.hole", "solid.hole"),
    ("arclet.split_body", "solid.split_body"),
];
const SOLID_PATTERNS: &[(&str, &str)] =
    &[("arclet.pattern_rect", "solid.pattern.rectangular"), ("arclet.pattern_circular", "solid.pattern.circular")];
const FILE_TOOLS: &[(&str, &str)] =
    &[("arclet.insert_step", "file.insert_step"), ("arclet.insert_mesh", "file.insert_mesh"), ("arclet.export", "file.export")];
const EDIT_TOOLS: &[(&str, &str)] =
    &[("arclet.repeat", "edit.repeat"), ("arclet.delete", "selection.delete"), ("arclet.select_objects", "select.set")];
const MESH_TOOLS: &[(&str, &str)] = &[("arclet.mesh_convert", "mesh.convert"), ("arclet.save_mesh", "file.save_mesh")];
const RENDER_TOOLS: &[(&str, &str)] = &[
    ("arclet.render_start", "render.start"),
    ("arclet.render_frame", "render.camera.fit"),
    ("arclet.render_status", "render.status"),
    ("arclet.render_cancel", "render.cancel"),
    ("arclet.render_export", "render.export"),
    ("arclet.render_browser", "view.render_browser"),
];

pub struct ArcletApp {
    cad: SolveApp,
    command: String,
    command_error: Option<String>,
    command_focus: bool,
    cmdline: cmdline::CommandLine,
    help: bool,
    controls: Sender<ControlRequest>,
    screenshot_path: Option<String>,
    screenshot_reply: Option<Receiver<Value>>,
    frame_count: u32,
    last_title: String,
    graph_close_pending: bool,
    /// A discard acknowledgement is valid for exactly one graph snapshot. Editing it
    /// while the CAD document's save prompt is open must cause a fresh graph prompt.
    acknowledged_graph: Option<solidcraft_graph::Graph>,
    awaiting_document_quit: bool,
}

impl ArcletApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::with_context(&cc.egui_ctx, platform::services(&cc.egui_ctx));
        if let Some(storage) = cc.storage
            && let Some(preferences) = storage.get_string(PREFS_KEY)
        {
            app.cad.load_prefs(&preferences);
        }
        app.cad.nodes.enabled = true;
        app.cad.embedded = true;
        app.cad.confirm_on_right_click = true;
        app.cad.home.open = false;
        if let Some(render) = &cc.wgpu_render_state {
            app.cad.set_wgpu_split(render, 24, 0, 1);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(dir) = platform::recovery_dir() {
                let minutes = app.cad.autosave_minutes;
                let interval = if minutes.is_finite() && minutes > 0.0 {
                    std::time::Duration::try_from_secs_f64(minutes.min(600.0) * 60.0).unwrap_or(std::time::Duration::MAX)
                } else {
                    std::time::Duration::MAX
                };
                solidcraft_ui_egui::recovery_ui::start(&mut app.cad, &dir, interval);
            }
            if !app.startup_arguments() && app.screenshot_path.is_none() {
                for path in solidcraft_ui_egui::documents::designs_to_reopen(&app.cad) {
                    app.cad.open_path_async(&path);
                }
            }
        }
        app
    }

    fn with_context(ctx: &egui::Context, services: Services) -> Self {
        // Register the named font families before the first shell/header is drawn.
        // Changing fonts in SolveApp::logic is too late for that same frame.
        solidcraft_ui_egui::fonts::setup_bundled_fonts(ctx);
        solidcraft_ui_egui::fonts::load_system_fonts_in_background(ctx);
        Self::with_services(services)
    }

    fn with_services(mut services: Services) -> Self {
        // A test or an embedding host may omit filesystem services. Arclet still must not
        // silently use the shared core application's recovery, crash or plugin folders.
        services.paths.get_or_insert_with(Default::default);
        let (controls, receive) = std::sync::mpsc::channel();
        let mut cad = SolveApp::new(Session::default(), services).with_control(receive);
        cad.embedded = true;
        cad.confirm_on_right_click = true;
        cad.home.open = false;
        cad.nodes.enabled = true;
        // The built-in starter is not a user edit. Persisted dirty graphs are restored
        // later by load_prefs and keep their dirty state; user changes are still guarded.
        cad.nodes.editor.mark_saved();
        #[cfg(not(target_arch = "wasm32"))]
        {
            cad.apply_off_thread = Some(std::time::Duration::ZERO);
        }
        // Arclet has its own settings. These defaults do not alter another application's files.
        cad.preferences.language = "en".into();
        cad.preferences.nav = "blender".into();
        cad.preferences.check_updates = false;
        // Arclet starts in the classic light-grey theme (see classic_look); saved prefs still win.
        classic_look::apply_default(&mut cad.preferences);
        Self {
            cad,
            command: String::new(),
            command_error: None,
            command_focus: false,
            cmdline: cmdline::CommandLine::default(),
            help: false,
            controls,
            screenshot_path: None,
            screenshot_reply: None,
            frame_count: 0,
            last_title: String::new(),
            graph_close_pending: false,
            acknowledged_graph: None,
            awaiting_document_quit: false,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn startup_arguments(&mut self) -> bool {
        let mut args = std::env::args().skip(1);
        let mut opened_model = false;
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--nodes" => self.cad.nodes.open = true,
                "--screenshot" => self.screenshot_path = args.next(),
                "--sample" => {
                    opened_model = true;
                    if let Err(error) = self.cad.run("solid.torus", json!({"major": 28, "minor": 8, "name": "Sample", "body_name": "Ring"})) {
                        self.command_error = Some(error);
                    }
                    self.cad.fit_view();
                    self.cad.session.mark_saved();
                }
                "--curve-sample" => {
                    opened_model = true;
                    if let Err(error) = self.curve_sample() {
                        self.command_error = Some(error);
                    }
                }
                "--zebra" => self.start("inspect.zebra"),
                path if !path.starts_with('-') => {
                    opened_model = true;
                    self.cad.open_path_async(path);
                }
                _ => {}
            }
        }
        opened_model
    }

    #[cfg(any(test, not(target_arch = "wasm32")))]
    fn curve_sample(&mut self) -> Result<(), String> {
        self.cad.run("sketch.create", json!({"plane":"XY", "name":"Curve study"}))?;
        let made = self.cad.run("sketch.spline.control_point", json!({"points":[[-40,0],[-25,35],[-5,5],[12,32],[25,-8],[42,0]]}))?;
        let curve = made
            .get("curves")
            .and_then(Value::as_array)
            .and_then(|curves| curves.first())
            .and_then(Value::as_str)
            .ok_or_else(|| "The sample did not create a curve".to_owned())?;
        self.cad.run("select.set", json!({"items":[{"type":"sketch_curve", "id":curve, "sketch":self.cad.session.active_sketch}]}))?;
        self.cad.fit_view();
        self.cad.start("sketch.spline.rebuild");
        self.cad.session.mark_saved();
        Ok(())
    }

    fn switch_workspace(&mut self, nodes: bool) {
        if let Err(error) = self.cad.set_nodes_open(nodes) {
            self.command_error = Some(error);
        }
    }

    fn start(&mut self, id: &str) {
        self.command_error = None;
        if matches!(id, "sketch.spline.rebuild" | "inspect.curve_continuity") && self.cad.session.active_sketch.is_none() {
            self.command_error = Some(tr("curve_quality.edit_sketch").to_string());
            return;
        }
        if cfg!(target_arch = "wasm32") && matches!(id, "sketch.spline.rebuild" | "inspect.curve_continuity" | "surface.match" | "surface.fair") {
            self.command_error = Some(tr("curve_quality.desktop_required").to_string());
            return;
        }
        self.cad.start(id);
    }

    fn header(&mut self, ui: &mut Ui) {
        let tokens = Tokens::get();
        Container::new(ui.ctx(), Surface::ApplicationBar)
            .edge("arclet_header", Edge::Top)
            .exact_size(50.0)
            .frame(Container::new(ui.ctx(), Surface::ApplicationBar).edge_frame(tokens.app_bar).inner_margin(egui::Margin::symmetric(16, 8)))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("Arclet").size(21.0).strong().color(tokens.text));
                    ui.label(RichText::new("CAD").size(11.0).color(tokens.text_dim));
                    ui.add_space(14.0);
                    for (label, nodes) in [("arclet.model", false), ("arclet.nodes", true)] {
                        let response =
                            ui_kit::button(ui, tr(label), if self.cad.nodes.open == nodes { ui_kit::Button::Primary } else { ui_kit::Button::Ghost });
                        solidcraft_ui_egui::scenario::publish_handle(if nodes { "arclet:nodes" } else { "arclet:model" }, response.rect.center());
                        if response.clicked() {
                            self.switch_workspace(nodes);
                        }
                    }
                    ui.separator();
                    ui.add_enabled_ui(!self.cad.nodes.open, |ui| self.menus(ui));
                    ui_kit::Menu::button(ui, tr("arclet.menu.help"), |ui| {
                        if ui_kit::button(ui, tr("arclet.help"), ui_kit::Button::Normal).clicked() {
                            self.help = !self.help;
                            ui.close();
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui_kit::button(ui, tr("arclet.settings"), ui_kit::Button::Ghost).clicked() {
                            FileAction::Settings.run(&mut self.cad);
                        }
                        if ui.available_width() > 200.0 {
                            let (name, dirty) = if self.cad.nodes.open {
                                (tr("arclet.graph_document"), self.cad.nodes.editor.is_dirty())
                            } else {
                                (self.cad.session.doc.name.as_str(), self.cad.session.is_dirty())
                            };
                            ui.label(RichText::new(format!("{name}{}", if dirty { " *" } else { "" })).color(tokens.text_dim));
                        }
                    });
                });
            });
    }

    /// The Rhino-style menu row, in the same order as Rhino's top bar. Each menu draws from
    /// the command tables above, so every entry is one engine command id.
    fn menus(&mut self, ui: &mut Ui) {
        ui_kit::Menu::button(ui, tr("arclet.menu.file"), |ui| {
            for (label, action) in [
                ("arclet.new", FileAction::New),
                ("arclet.open", FileAction::Open),
                ("arclet.save", FileAction::Save),
                ("arclet.save_as", FileAction::SaveAs),
            ] {
                if ui_kit::button(ui, tr(label), ui_kit::Button::Enabled(action.enabled(&self.cad))).clicked() {
                    action.run(&mut self.cad);
                    ui.close();
                }
            }
            ui.separator();
            for (label, extension) in [("arclet.export_stl", "stl"), ("arclet.export_3mf", "3mf"), ("arclet.export_step", "step")] {
                if ui_kit::button(ui, tr(label), ui_kit::Button::Normal).clicked() {
                    if extension == "step" {
                        FileAction::Export(extension).run(&mut self.cad);
                    } else {
                        FileAction::ExportMesh(extension).run(&mut self.cad);
                    }
                    ui.close();
                }
            }
            ui.separator();
            self.tool_items(ui, &[FILE_TOOLS]);
        });
        ui_kit::Menu::button(ui, tr("arclet.menu.edit"), |ui| {
            for (label, action) in [("arclet.undo", FileAction::Undo), ("arclet.redo", FileAction::Redo)] {
                if ui_kit::button(ui, tr(label), ui_kit::Button::Enabled(action.enabled(&self.cad))).clicked() {
                    action.run(&mut self.cad);
                    ui.close();
                }
            }
            ui.separator();
            self.tool_items(ui, &[EDIT_TOOLS]);
        });
        ui_kit::Menu::button(ui, tr("arclet.menu.view"), |ui| {
            self.tool_items(ui, &[VIEWS]);
            ui.separator();
            let isolate = solidcraft_ui_egui::context_menu::items(&self.cad, &solidcraft_ui_egui::context_menu::Target::Viewport)
                .into_iter()
                .find(|item| item.id == "ui.isolate");
            if ui_kit::button(ui, tr("arclet.isolate"), ui_kit::Button::Enabled(isolate.as_ref().is_some_and(|item| item.enabled))).clicked()
                && let Some(item) = isolate
            {
                solidcraft_ui_egui::context_menu::run_item(&mut self.cad, &item, ui.max_rect().center());
                ui.close();
            }
            if ui_kit::button(ui, tr("arclet.show_all"), ui_kit::Button::Normal).clicked() {
                if let Err(error) = self.cad.run("browser.visibility", json!({"folder": "all", "visible": true})) {
                    self.command_error = Some(error);
                }
                ui.close();
            }
        });
        self.tool_menu(ui, "arclet.menu.curve", &[CURVES, CURVE_SHAPES, CURVE_EDITS, CURVE_TOOLS]);
        self.tool_menu(ui, "arclet.menu.surface", &[SURFACES, SURFACE_MORE]);
        self.tool_menu(ui, "arclet.menu.solid", &[SOLIDS, SOLID_MORE, SOLID_PATTERNS]);
        self.tool_menu(ui, "arclet.menu.mesh", &[MESH_TOOLS]);
        self.tool_menu(ui, "arclet.menu.transform", &[TRANSFORMS]);
        self.tool_menu(ui, "arclet.menu.analyze", &[ANALYSIS]);
        self.tool_menu(ui, "arclet.menu.render", &[RENDER_TOOLS]);
        ui_kit::Menu::button(ui, tr("arclet.menu.tools"), |ui| {
            if ui_kit::button(ui, tr("arclet.all_tools"), ui_kit::Button::Normal).clicked() {
                self.start("ui.palette");
                ui.close();
            }
            ui.separator();
            solidcraft_ui_egui::toolbar::selection_filter(&mut self.cad, ui);
            ui.separator();
            if ui_kit::button(ui, tr("arclet.clear_selection"), ui_kit::Button::Normal).clicked() {
                self.start("select.clear");
                ui.close();
            }
        });
    }

    fn tools(&mut self, ui: &mut Ui) {
        if self.cad.nodes.open {
            return;
        }
        let tokens = Tokens::get();
        Container::new(ui.ctx(), Surface::ApplicationBar)
            .edge("arclet_tools", Edge::Top)
            .frame(Container::new(ui.ctx(), Surface::ApplicationBar).edge_frame(tokens.panel).inner_margin(egui::Margin::symmetric(12, 6)))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for &(label, id) in CURVES {
                        if tool_button(ui, label, id).clicked() {
                            self.start(id);
                        }
                    }
                    ui.separator();
                    if self.cad.session.active_sketch.is_some() {
                        for (label, id) in
                            [("arclet.trim", "sketch.trim"), ("arclet.offset", "sketch.offset"), ("arclet.point", "sketch.spline.insert_point")]
                        {
                            if tool_button(ui, label, id).clicked() {
                                self.start(id);
                            }
                        }
                        if ui_kit::button(ui, tr("arclet.finish_sketch"), ui_kit::Button::Primary).clicked() {
                            self.cad.finish_sketch();
                        }
                    }
                    if ui_kit::button(ui, tr("arclet.all_tools"), ui_kit::Button::Ghost).clicked() {
                        self.start("ui.palette");
                    }
                });
            });
    }

    /// Command rows for an open menu. A rule separates the groups, so one table can supply
    /// a whole section of a menu without copying its entries.
    fn tool_items(&mut self, ui: &mut Ui, groups: &[&[(&str, &str)]]) {
        for (index, group) in groups.iter().enumerate() {
            if index > 0 {
                ui.separator();
            }
            for &(label, id) in group.iter() {
                if tool_button(ui, label, id).clicked() {
                    self.start(id);
                    ui.close();
                }
            }
        }
    }

    fn tool_menu(&mut self, ui: &mut Ui, title: &str, groups: &[&[(&str, &str)]]) {
        ui_kit::Menu::button(ui, tr(title), |ui| self.tool_items(ui, groups));
    }

    /// Run the command bar's text. An empty line repeats the last command, like pressing
    /// Enter at an empty command line, so a repeated operation is one keystroke.
    fn run_command_text(&mut self) {
        if self.command.trim().is_empty() {
            match self.cad.last_command.clone() {
                Some((id, _)) => {
                    self.cmdline.repeated(&id);
                    self.start_logged(&id);
                }
                None => {
                    let error = tr("arclet.command.empty").to_string();
                    self.cmdline.failed("", &error);
                    self.command_error = Some(error);
                }
            }
            return;
        }
        self.cmdline.entered(&self.command);
        match commands::resolve(&self.command) {
            Ok(id) => {
                self.start_logged(id);
                self.command.clear();
            }
            Err(error) => {
                self.cmdline.failed(&self.command, &error);
                self.command_error = Some(error);
            }
        }
    }

    /// Start `id` from the command line and log whether it started.
    fn start_logged(&mut self, id: &str) {
        self.start(id);
        match self.command_error.clone() {
            Some(error) => self.cmdline.failed(id, &error),
            None => self.cmdline.started(id),
        }
    }

    fn command_bar(&mut self, ui: &mut Ui) {
        if self.cad.nodes.open {
            return;
        }
        let tokens = Tokens::get();
        Container::new(ui.ctx(), Surface::ApplicationBar)
            .edge("arclet_command", Edge::Bottom)
            .frame(Container::new(ui.ctx(), Surface::ApplicationBar).edge_frame(tokens.panel).inner_margin(egui::Margin::symmetric(16, 8)))
            .show(ui, |ui| {
                if let Some(error) = &self.command_error {
                    ui.label(RichText::new(error).color(tokens.error));
                }
                let invalid = self.command_error.is_some();
                match cmdline::show(ui, &mut self.cmdline, &mut self.command, &mut self.command_focus, invalid) {
                    cmdline::Action::Nothing => {}
                    cmdline::Action::Run => self.run_command_text(),
                    cmdline::Action::Suggested { id, label } => {
                        self.cmdline.entered(label);
                        self.start_logged(id);
                        self.command.clear();
                    }
                    cmdline::Action::Cancel => {
                        self.command_error = None;
                        cmdline::cancel_active(&mut self.cad);
                    }
                    cmdline::Action::Finish => solidcraft_ui_egui::tools::confirm_current(&mut self.cad),
                }
            });
    }

    fn help(&mut self, ctx: &egui::Context) {
        if !self.help {
            return;
        }
        let closed = ui_kit::Window::new(egui::Id::new("arclet_help"), "help", tr("arclet.help_title")).cards(1.5).show_body(ctx, |ui| {
            ui.set_max_width(400.0);
            for key in ["arclet.help_create", "arclet.help_edit", "arclet.help_nodes", "arclet.help_surfaces", "arclet.help_files"] {
                ui.label(tr(key));
                ui.add_space(8.0);
            }
        });
        self.help &= !closed;
    }

    fn frame(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.frame_count = self.frame_count.saturating_add(1);
        self.cad.logic(&ctx);
        for error in platform::poll(&ctx) {
            if self.cad.nodes.open {
                self.cad.nodes.editor.report_error(error.clone());
            }
            self.command_error = Some(error);
        }
        if platform::has_unsaved_downloads() && !self.cad.session.is_dirty() {
            self.cad.session.mark_unsaved();
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::K)) && !self.cad.nodes.open {
            self.command_focus = true;
        }
        solidcraft_ui_egui::scenario::clear_handles();
        self.header(ui);
        self.tools(ui);
        if cmdline::route_letters(&ctx, &self.cad, &mut self.command) {
            self.command_focus = true;
        }
        // Added before the command bar so the status bar sits below it.
        crate::statusbar::show(&mut self.cad, ui);
        self.command_bar(ui);
        self.cad.ui_after_host(ui);
        self.help(&ctx);
        let unsaved = solidcraft_ui_egui::documents::any_dirty(&self.cad) || self.cad.nodes.editor.is_dirty() || platform::has_unsaved_downloads();
        if let Err(error) = platform::set_unsaved(unsaved) {
            self.command_error = Some(error);
        }
        // The shared document quit workflow covers CAD tabs, not the separate graph.
        // Guard both the OS close event and quit requests originating inside that workflow.
        if self.awaiting_document_quit && !self.cad.quit_requested && self.cad.docs.quitting.is_none() && self.cad.docs.closing.is_none() {
            self.awaiting_document_quit = false;
            self.acknowledged_graph = None;
        }
        if ctx.input(|input| input.viewport().close_requested()) && !self.cad.quit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request_close();
        }
        if self.cad.quit_requested && (!self.graph_allows_close() || self.cad.file_picker_busy()) {
            self.cad.quit_requested = false;
            self.graph_close_pending = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        self.graph_close_dialog(&ctx);
        if self.cad.quit_requested && self.graph_allows_close() && !self.cad.file_picker_busy() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let (name, dirty) = if self.cad.nodes.open {
            (tr("arclet.graph_document"), self.cad.nodes.editor.is_dirty())
        } else {
            (self.cad.session.doc.name.as_str(), self.cad.session.is_dirty())
        };
        let title = format!("Arclet CAD — {name}{}", if dirty { " *" } else { "" });
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        self.capture(&ctx);
    }

    fn graph_allows_close(&self) -> bool {
        if self.cad.nodes.worker_busy() {
            return false;
        }
        if !self.cad.nodes.editor.is_dirty() {
            return true;
        }
        // Borrowed graph equality avoids serializing up to 4 MiB on every frame while
        // a CAD save prompt is open. Snapshot values are immutable or copy-on-write.
        self.acknowledged_graph.as_ref() == Some(self.cad.nodes.editor.graph())
    }

    fn request_close(&mut self) {
        if self.cad.file_picker_busy() || !self.graph_allows_close() {
            self.graph_close_pending = true;
            return;
        }
        self.graph_close_pending = false;
        solidcraft_ui_egui::documents::request_quit(&mut self.cad);
        self.awaiting_document_quit = self.cad.quit_requested || self.cad.docs.quitting.is_some();
    }

    fn cancel_graph_close(&mut self) {
        self.graph_close_pending = false;
        self.acknowledged_graph = None;
        self.awaiting_document_quit = false;
    }

    fn acknowledge_graph_close(&mut self) -> Result<(), String> {
        if self.cad.nodes.file_busy() || self.cad.file_picker_busy() {
            return Err(tr("arclet.close.busy").to_string());
        }
        self.acknowledged_graph = Some(self.cad.nodes.editor.graph().clone());
        self.request_close();
        Ok(())
    }

    fn graph_close_dialog(&mut self, ctx: &egui::Context) {
        if !self.graph_close_pending {
            return;
        }
        if self.graph_allows_close() && !self.cad.file_picker_busy() {
            self.request_close();
            return;
        }
        // A native save picker owns the user's choice until it resolves. A cancelled
        // picker returns here with the graph still dirty, rather than silently quitting.
        if self.cad.file_picker_busy() {
            return;
        }
        let busy = self.cad.nodes.worker_busy();
        let mut action = 0;
        let closed = ui_kit::Window::new(egui::Id::new("arclet_graph_close"), "save", tr("arclet.close.title")).cards(1.5).show_body(ctx, |ui| {
            ui.set_max_width(440.0);
            ui.label(tr(if busy { "arclet.close.busy" } else { "arclet.close.message" }));
            if let Some(error) = self.cad.nodes.editor.last_error() {
                ui.add_space(6.0);
                ui.colored_label(Tokens::get().error, error);
            }
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                if ui_kit::button(ui, tr("arclet.close.keep"), ui_kit::Button::Normal).clicked() {
                    action = 1;
                }
                if ui_kit::button(ui, tr("arclet.close.save"), ui_kit::Button::Enabled(!busy)).clicked() {
                    action = 2;
                }
                if ui_kit::button(ui, tr("arclet.close.without_save"), ui_kit::Button::Enabled(!busy)).clicked() {
                    action = 3;
                }
            });
        });
        match action {
            1 => self.cancel_graph_close(),
            2 => {
                self.acknowledged_graph = None;
                if let Err(error) = solidcraft_ui_egui::node_workspace::save_graph(&mut self.cad, ctx) {
                    self.command_error = Some(error);
                }
                // Completion is observed on a later frame. Only an actually clean graph
                // (not a clicked button, cancelled dialog or failed write) may continue.
            }
            3 => {
                if let Err(error) = self.acknowledge_graph_close() {
                    self.command_error = Some(error);
                }
            }
            _ if closed => self.cancel_graph_close(),
            _ => {}
        }
    }

    fn capture(&mut self, ctx: &egui::Context) {
        if self.frame_count >= 12
            && !self.cad.nodes.editor.is_running()
            && !self.cad.preview.busy
            && let Some(path) = self.screenshot_path.take()
        {
            let (request, reply) = ControlRequest::new("ui.screenshot", json!({"path": path}));
            if self.controls.send(request).is_ok() {
                self.screenshot_reply = Some(reply);
            }
        }
        if self.screenshot_path.is_some() || self.screenshot_reply.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(20));
        }
        if let Some(reply) = &self.screenshot_reply
            && let Ok(result) = reply.try_recv()
        {
            if result.get("ok").and_then(Value::as_bool) != Some(true) {
                log::error!("screenshot: {result}");
            }
            self.screenshot_reply = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn tool_button(ui: &mut Ui, label: &str, command: &str) -> egui::Response {
    let tokens = Tokens::get();
    let text = tr(label);
    let (rect, response) = ui.allocate_exact_size(vec2((text.len() as f32 * 7.0 + 44.0).max(72.0), 32.0), egui::Sense::click());
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, tokens.radius_sm, tokens.hover);
    }
    if let Some(spec) = find_command(command) {
        solidcraft_ui_egui::icons::paint_icon(
            ui.painter(),
            egui::Rect::from_center_size(egui::pos2(rect.left() + 16.0, rect.center().y), vec2(22.0, 22.0)),
            spec.icon,
            1.0,
        );
    }
    ui.painter().text(
        egui::pos2(rect.left() + 33.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(13.0),
        tokens.text,
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text));
    solidcraft_ui_egui::scenario::publish_handle(&format!("arclet:tool:{command}"), rect.center());
    response.on_hover_text(text)
}

impl eframe::App for ArcletApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(PREFS_KEY, self.cad.prefs());
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        self.cad.raw_input_hook(input);
    }

    fn on_exit(&mut self) {
        self.cad.nodes.editor.cancel();
        solidcraft_ui_egui::save_job::tick(&mut self.cad, true);
        solidcraft_ui_egui::recovery_ui::close(&mut self.cad);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solidcraft_ui_egui::scenario;

    /// Drive the complete Arclet shell rather than drawing the shared CAD widget alone.
    /// Separate press/release frames exercise the same hit testing as a real pointer.
    struct AppFrames {
        app: ArcletApp,
        ctx: egui::Context,
        output: egui::PlatformOutput,
        time: f64,
    }

    impl AppFrames {
        fn new() -> Self {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            // Exercise the runtime's font setup too. Installing fonts only in this test
            // harness used to hide a panic in the native application's first frame.
            let mut app = ArcletApp::with_context(&ctx, Default::default());
            // These checks exercise layout and clicks, not background geometry evaluation.
            app.cad.nodes.editor.cancel();
            let mut frames = Self { app, ctx, output: Default::default(), time: 0.0 };
            frames.idle(3);
            frames
        }

        fn frame(&mut self, events: Vec<egui::Event>) {
            self.time += 1.0 / 60.0;
            let mut raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1440.0, 900.0))),
                focused: true,
                time: Some(self.time),
                events,
                ..Default::default()
            };
            self.app.cad.raw_input_hook(&mut raw);
            let app = &mut self.app;
            let mut output = self.ctx.run_ui(raw, |ui| app.frame(ui));
            output.textures_delta.clear();
            self.output = output.platform_output;
        }

        fn idle(&mut self, count: usize) {
            for _ in 0..count {
                self.frame(Vec::new());
            }
        }

        fn click_at(&mut self, at: egui::Pos2) {
            assert!(at.is_finite());
            for pressed in [true, false] {
                self.frame(vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
                ]);
            }
        }

        fn click_handle(&mut self, name: &str) {
            let at = scenario::handle_at(name).unwrap_or_else(|| panic!("missing visible handle {name}"));
            self.click_at(at);
        }

        fn click_button(&mut self, label: &str) {
            let at = self.output.accesskit_update.as_ref().and_then(|tree| {
                tree.nodes.iter().rev().find_map(|(_, node)| {
                    if node.role() != egui::accesskit::Role::Button || node.label() != Some(label) {
                        return None;
                    }
                    node.bounds()
                        .filter(|bounds| bounds.x1 > bounds.x0 && bounds.y1 > bounds.y0)
                        .map(|bounds| egui::pos2(((bounds.x0 + bounds.x1) / 2.0) as f32, ((bounds.y0 + bounds.y1) / 2.0) as f32))
                })
            });
            self.click_at(at.unwrap_or_else(|| panic!("missing visible button {label}")));
        }
    }

    #[test]
    fn full_frames_keep_header_command_and_tool_handles() {
        let mut frames = AppFrames::new();
        for _ in 0..2 {
            frames.idle(1);
            for name in ["arclet:model", "arclet:nodes", "arclet:command", "arclet:tool:sketch.line"] {
                let at = scenario::handle_at(name).unwrap_or_else(|| panic!("the shared CAD frame erased {name}"));
                assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1440.0, 900.0)).contains(at), "{name}: {at:?}");
            }
        }
    }

    #[test]
    fn clicking_workspace_buttons_preserves_the_model_and_updates_visible_controls() {
        let mut frames = AppFrames::new();
        frames.app.cad.run("solid.box", json!({"length": 20, "width": 30, "height": 10})).unwrap();
        let document = frames.app.cad.session.doc.to_json();
        let revision = frames.app.cad.session.revision;
        frames.idle(2);

        frames.click_handle("arclet:nodes");
        assert!(frames.app.cad.nodes.open);
        assert_eq!(frames.app.cad.session.doc.to_json(), document);
        assert!(scenario::handle_at("arclet:model").is_some(), "the Model button remains reachable from Nodes");
        assert!(scenario::handle_at("arclet:command").is_none(), "hidden Model controls must not leave stale handles");

        frames.click_handle("arclet:model");
        assert!(!frames.app.cad.nodes.open);
        assert_eq!(frames.app.cad.session.doc.to_json(), document);
        assert_eq!(frames.app.cad.session.revision, revision);
        assert!(scenario::handle_at("arclet:command").is_some());
        assert!(scenario::handle_at("arclet:tool:sketch.line").is_some());
    }

    #[test]
    fn settings_opens_and_closes_visibly_while_nodes_stays_active() {
        let mut frames = AppFrames::new();
        frames.click_handle("arclet:nodes");
        frames.idle(2);
        assert!(frames.app.cad.nodes.open);
        assert!(!frames.app.cad.prefs_window.open);

        frames.click_button("Settings");
        frames.idle(3);
        assert!(frames.app.cad.nodes.open);
        assert!(frames.app.cad.prefs_window.open);
        let window = scenario::rect_of("prefs:window").expect("Settings must be drawn over Nodes, not merely marked open");
        assert!(window.is_finite() && window.is_positive());
        assert!(scenario::handle_at("prefs:page:General").is_some());

        frames.click_handle("prefs:cancel");
        frames.idle(2);
        assert!(!frames.app.cad.prefs_window.open);
        assert!(frames.app.cad.nodes.open);
        assert!(scenario::rect_of("prefs:window").is_none());
    }

    #[test]
    fn curated_geometry_tools_exist_in_the_engine() {
        let tables = CURVES
            .iter()
            .chain(CURVE_TOOLS)
            .chain(CURVE_SHAPES)
            .chain(CURVE_EDITS)
            .chain(SURFACES)
            .chain(SURFACE_MORE)
            .chain(SOLIDS)
            .chain(SOLID_MORE)
            .chain(SOLID_PATTERNS)
            .chain(TRANSFORMS)
            .chain(ANALYSIS)
            .chain(VIEWS)
            .chain(FILE_TOOLS)
            .chain(EDIT_TOOLS)
            .chain(MESH_TOOLS)
            .chain(RENDER_TOOLS);
        for &(_, id) in tables {
            assert!(find_command(id).is_some(), "{id}");
        }
        assert_eq!(commands::resolve("Spline"), Ok("sketch.spline.control_point"));
    }

    #[test]
    fn an_empty_command_line_repeats_the_last_command() {
        let mut app = ArcletApp::with_services(Default::default());
        app.command.clear();
        app.run_command_text();
        assert!(app.command_error.is_some(), "nothing to repeat yet");
        app.command = "front".into();
        app.run_command_text();
        assert!(app.command_error.is_none(), "a known name starts without an error");
        assert!(app.command.is_empty());
        app.command_error = None;
        app.cad.last_command = Some(("view.top".into(), "Top".into()));
        app.command.clear();
        app.run_command_text();
        assert!(app.command_error.is_none());
        assert_eq!(app.cad.last_command.as_ref().map(|(id, _)| id.as_str()), Some("view.top"));
    }

    #[test]
    fn curve_example_opens_rebuild_with_a_real_selected_source() {
        let mut app = ArcletApp::with_services(Default::default());
        app.curve_sample().unwrap();
        assert!(app.cad.session.active_sketch.is_some());
        let dialog = app.cad.dialog.as_ref().unwrap();
        assert_eq!(app.cad.last_command.as_ref().map(|(id, _)| id.as_str()), Some("sketch.spline.rebuild"));
        assert_eq!(dialog.inputs[0].items.len(), 1);
        assert!(!app.cad.session.is_dirty());
    }

    #[test]
    fn workspace_switch_keeps_the_model_and_refuses_an_active_sketch() {
        let mut app = ArcletApp::with_services(Default::default());
        app.cad.run("solid.box", json!({"length": 20, "width": 30, "height": 10})).unwrap();
        let revision = app.cad.session.revision;
        app.switch_workspace(true);
        assert!(app.cad.nodes.open);
        app.switch_workspace(false);
        assert_eq!(app.cad.session.revision, revision);
        app.cad.run("sketch.create", json!({"plane":"XY"})).unwrap();
        app.switch_workspace(true);
        assert!(!app.cad.nodes.open);
        assert!(app.command_error.is_some());
    }
    #[test]
    fn an_untouched_starter_graph_does_not_count_as_user_work() {
        let app = ArcletApp::with_services(Default::default());
        assert!(!app.cad.nodes.editor.is_dirty());
        assert!(app.graph_allows_close());
    }

    fn change_graph(app: &mut ArcletApp, x: f32) {
        let mut graph = app.cad.nodes.editor.graph().clone();
        graph.add_node("Number", (x, 100.0)).unwrap();
        app.cad.nodes.editor.replace_graph(graph).unwrap();
    }

    #[test]
    fn a_dirty_graph_blocks_quit_even_with_a_saved_model() {
        let mut app = ArcletApp::with_services(Default::default());
        app.cad.session.mark_saved();
        change_graph(&mut app, 100.0);
        app.request_close();
        assert!(app.graph_close_pending);
        assert!(!app.cad.quit_requested);
        assert!(app.cad.docs.quitting.is_none());
    }

    #[test]
    fn cancelling_graph_close_keeps_edits_and_does_not_grant_later_discard() {
        let mut app = ArcletApp::with_services(Default::default());
        change_graph(&mut app, 100.0);
        let text = app.cad.nodes.editor.to_json().unwrap();
        app.request_close();
        app.cancel_graph_close();
        assert!(!app.graph_close_pending);
        assert!(!app.cad.quit_requested);
        assert_eq!(app.cad.nodes.editor.to_json().unwrap(), text);
        assert!(app.cad.nodes.editor.is_dirty());
        assert!(!app.graph_allows_close());
    }

    #[test]
    fn graph_discard_acknowledges_only_the_exact_snapshot() {
        let mut app = ArcletApp::with_services(Default::default());
        app.cad.session.mark_saved();
        change_graph(&mut app, 100.0);
        app.acknowledge_graph_close().unwrap();
        assert!(app.cad.quit_requested);
        assert!(app.graph_allows_close());
        change_graph(&mut app, 200.0);
        assert!(!app.graph_allows_close(), "later edits require another user decision");
    }

    #[test]
    fn a_completed_graph_save_continues_the_existing_model_quit_workflow() {
        let mut app = ArcletApp::with_services(Default::default());
        app.cad.session.mark_saved();
        change_graph(&mut app, 100.0);
        app.request_close();
        let written = app.cad.nodes.editor.to_json().unwrap();
        app.cad.nodes.editor.mark_saved_json(&written).unwrap();
        app.graph_close_dialog(&egui::Context::default());
        assert!(!app.graph_close_pending);
        assert!(app.cad.quit_requested);
    }

    #[test]
    fn graph_acknowledgement_does_not_discard_an_unsaved_cad_design() {
        let mut app = ArcletApp::with_services(Default::default());
        app.cad.run("solid.box", json!({"length":20,"width":30,"height":10})).unwrap();
        change_graph(&mut app, 100.0);
        app.acknowledge_graph_close().unwrap();
        assert!(!app.cad.quit_requested);
        assert!(app.cad.docs.quitting.is_some());
        assert!(app.cad.docs.closing.is_some());
        assert!(app.cad.session.is_dirty());
    }
}
