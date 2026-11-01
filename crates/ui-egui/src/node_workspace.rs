//! File and CAD-host integration for the portable node editor.
//!
//! Graphs are separate JSON documents. Importing a generated mesh is explicit and uses the
//! existing asynchronous CAD import path; a graph run never replaces the current design.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};
use solidcraft_engine::io::vfs;

use crate::SolveApp;
use crate::i18n::tr;
use crate::node_editor::{NodeAction, NodeEditor, NodeMesh};

const MAX_GRAPH_FILE: usize = solidcraft_graph::MAX_DOCUMENT_BYTES;
const MAX_EXPORT_FILE: usize = 64 * 1024 * 1024;
const MAX_PREVIEW_TRIANGLES: usize = 100_000;

#[derive(Default)]
pub struct State {
    /// Opted into only by the independent Arclet host, not the existing applications.
    pub enabled: bool,
    pub editor: NodeEditor,
    pub open: bool,
    path: Option<String>,
    job: Option<Job>,
    message: Option<String>,
    preview_path: Option<Arc<TemporaryMesh>>,
    next_preview: u64,
}

struct Job {
    result: std::sync::mpsc::Receiver<Result<Completed, String>>,
    cancel: Arc<AtomicBool>,
}

enum Work {
    Load(String),
    Save { path: String, text: String, graph: bool },
    Preview { path: String, mesh: NodeMesh, destination: Destination },
}

enum Completed {
    Loaded { path: String, text: String },
    Saved { path: String, text: String, graph: bool },
    Preview { file: Arc<TemporaryMesh>, destination: Destination },
}

/// The last owner removes the staging file, including when a completed worker result is
/// dropped before the UI polls it. The importer is kept alive by State until it has read it.
struct TemporaryMesh {
    path: String,
}

impl Drop for TemporaryMesh {
    fn drop(&mut self) {
        let _ = vfs::remove(&self.path);
    }
}

/// A prepared mesh belongs to the design the user was looking at when they chose Add to model.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Destination {
    session: u64,
    document: String,
    revision: u64,
}

impl Destination {
    fn of(app: &SolveApp) -> Self {
        Self { session: app.session.uid, document: app.session.document_id.clone(), revision: app.session.revision }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::Relaxed);
        }
    }
}

impl State {
    /// A pending file operation must finish before a host closes its workspace. In
    /// particular, dropping State cancels the worker, so a clean graph alone is not enough.
    pub fn file_busy(&self) -> bool {
        self.job.is_some() || self.preview_path.is_some()
    }

    /// Hosts already persist preferences on exit and periodically. Keep an independent graph
    /// recovery snapshot there as well; ordinary CAD files retain their existing format.
    pub(crate) fn saved_state(&self) -> Value {
        if !self.enabled {
            return Value::Null;
        }
        json!({"graph": self.editor.to_json().ok(), "path": self.path, "dirty": self.editor.is_dirty()})
    }

    pub(crate) fn restore_state(&mut self, saved: &Value) {
        let Some(text) = saved.get("graph").and_then(Value::as_str) else { return };
        self.enabled = true;
        match solidcraft_graph::Graph::from_json(text).map_err(|error| error.to_string()).and_then(|graph| self.editor.replace_graph(graph)) {
            Ok(()) => {
                if saved.get("dirty").and_then(Value::as_bool) == Some(false) {
                    self.editor.mark_saved();
                } else {
                    self.editor.mark_unsaved();
                }
                self.path = saved.get("path").and_then(Value::as_str).filter(|path| path.len() <= 4096).map(str::to_owned);
            }
            Err(error) => self.editor.report_error(error),
        }
    }

    fn start(&mut self, work: Work, ctx: &egui::Context) -> Result<(), String> {
        if self.job.is_some() {
            return Err(tr("node.workspace.file_busy").to_string());
        }
        let (send, result) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ctx = ctx.clone();
            std::thread::Builder::new()
                .name("node files".into())
                .spawn(move || {
                    let done = perform(work, &flag);
                    let _ = send.send(done);
                    ctx.request_repaint();
                })
                .map_err(|error| error.to_string())?;
        }
        #[cfg(target_arch = "wasm32")]
        {
            // The web host has already read a bounded file asynchronously into VFS. No native
            // filesystem or thread is called here. Geometry preview remains deliberately small.
            let _ = send.send(perform(work, &flag));
            ctx.request_repaint();
        }
        self.job = Some(Job { result, cancel });
        self.message = None;
        Ok(())
    }
}

impl SolveApp {
    /// A programmatic workspace switch never opens a picker or finishes an active CAD edit.
    pub fn set_nodes_open(&mut self, open: bool) -> Result<(), String> {
        if open
            && (self.session.active_sketch.is_some()
                || self.tool.is_some()
                || self.dialog.is_some()
                || self.file_picker_busy()
                || self.file_load.busy())
        {
            return Err(tr("node.workspace.finish_edit").to_string());
        }
        self.nodes.open = open;
        self.nodes.enabled |= open;
        if open {
            self.home.open = false;
            self.ui.palette_open = false;
        }
        Ok(())
    }
}

fn perform(work: Work, cancel: &AtomicBool) -> Result<Completed, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err(tr("node.workspace.cancelled").to_string());
    }
    match work {
        Work::Load(path) => {
            let text = vfs::read_to_string(&path, MAX_GRAPH_FILE).map_err(|error| error.to_string())?;
            solidcraft_graph::Graph::from_json(&text).map_err(|error| error.to_string())?;
            Ok(Completed::Loaded { path, text })
        }
        Work::Save { path, text, graph } => {
            if text.len() > MAX_EXPORT_FILE {
                return Err(tr("node.workspace.file_large").to_string());
            }
            if cancel.load(Ordering::Relaxed) {
                return Err(tr("node.workspace.cancelled").to_string());
            }
            write(&path, text.as_bytes())?;
            Ok(Completed::Saved { path, text, graph })
        }
        Work::Preview { path, mesh, destination } => {
            let bytes = mesh_stl(&mesh, cancel)?;
            if cancel.load(Ordering::Relaxed) {
                return Err(tr("node.workspace.cancelled").to_string());
            }
            write(&path, &bytes)?;
            Ok(Completed::Preview { file: Arc::new(TemporaryMesh { path }), destination })
        }
    }
}

fn write(path: &str, bytes: &[u8]) -> Result<(), String> {
    vfs::check(path).map_err(|error| error.to_string())?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        solidcraft_engine::io::write_atomic(std::path::Path::new(path), bytes, true).map_err(|error| error.to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        vfs::write(path, bytes).map_err(|error| error.to_string())
    }
}

fn mesh_stl(mesh: &[[solidcraft_engine::geom::Vec3; 3]], cancel: &AtomicBool) -> Result<Vec<u8>, String> {
    if mesh.is_empty() || mesh.len() > MAX_PREVIEW_TRIANGLES {
        return Err(tr("node.workspace.preview_limit").to_string());
    }
    let mut bytes = vec![0_u8; 80];
    let count = u32::try_from(mesh.len()).map_err(|error| error.to_string())?;
    bytes.extend_from_slice(&count.to_le_bytes());
    for triangle in mesh {
        if cancel.load(Ordering::Relaxed) {
            return Err(tr("node.workspace.cancelled").to_string());
        }
        // A zero STL normal asks the importer to calculate it from the three vertices.
        bytes.extend_from_slice(&[0_u8; 12]);
        for point in triangle {
            for value in [point.x, point.y, point.z] {
                let narrowed = value as f32;
                if !value.is_finite() || !narrowed.is_finite() {
                    return Err(tr("node.workspace.invalid_mesh").to_string());
                }
                bytes.extend_from_slice(&narrowed.to_le_bytes());
            }
        }
        bytes.extend_from_slice(&[0_u8; 2]);
    }
    Ok(bytes)
}

pub(crate) fn tick(app: &mut SolveApp, ctx: &egui::Context) {
    let finished = app.nodes.job.as_ref().and_then(|job| match job.result.try_recv() {
        Ok(done) => Some((done, job.cancel.load(Ordering::Relaxed))),
        Err(std::sync::mpsc::TryRecvError::Disconnected) => Some((Err(tr("node.workspace.worker_closed").to_string()), false)),
        Err(std::sync::mpsc::TryRecvError::Empty) => None,
    });
    if let Some((done, cancelled)) = finished {
        app.nodes.job = None;
        // A committed atomic save is already on disk. A late Cancel cannot undo the write
        // and must not tell the user that the saved data was discarded.
        if cancelled && !matches!(&done, Ok(Completed::Saved { .. })) {
            drop(done);
            app.nodes.message = Some(tr("node.workspace.cancelled").to_string());
        } else {
            match done {
                Ok(Completed::Loaded { path, text }) => match app.nodes.editor.load_json(&text) {
                    Ok(()) => {
                        app.nodes.path = Some(path);
                        app.nodes.message = Some(tr("node.workspace.loaded").to_string());
                    }
                    Err(error) => app.nodes.editor.report_error(error),
                },
                Ok(Completed::Saved { path, text, graph }) => {
                    if graph {
                        match app.nodes.editor.mark_saved_json(&text) {
                            Ok(()) => {
                                app.nodes.path = Some(path);
                                app.nodes.message = Some(tr("node.workspace.saved").to_string());
                            }
                            Err(error) => app.nodes.editor.report_error(error),
                        }
                    } else {
                        app.nodes.message = Some(tr("node.workspace.saved").to_string());
                    }
                }
                Ok(Completed::Preview { file, destination }) => {
                    if destination != Destination::of(app)
                        || app.file_load.busy()
                        || app.file_picker_busy()
                        || app.session.active_sketch.is_some()
                        || app.tool.is_some()
                        || app.dialog.is_some()
                    {
                        app.nodes.editor.report_error(tr("node.workspace.model_changed"));
                    } else {
                        app.insert_path_async(&file.path);
                        app.nodes.preview_path = Some(file);
                        app.nodes.open = false;
                    }
                }
                Err(error) => app.nodes.editor.report_error(error),
            }
        }
    }
    if app.nodes.job.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
    }
    if !app.file_load.busy() {
        app.nodes.preview_path = None;
    }
}

pub(crate) fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    ui.painter().rect_filled(ui.available_rect_before_wrap(), 0, crate::theme::Tokens::get().panel);
    let mut actions = Vec::new();
    let busy = app.nodes.job.is_some();
    if busy || app.nodes.message.is_some() {
        ui.horizontal(|ui| {
            if busy {
                ui.spinner();
                ui.label(tr("node.workspace.file_busy"));
                if crate::ui_kit::button(ui, tr("node.workspace.cancel"), crate::ui_kit::Button::Normal).clicked()
                    && let Some(job) = &app.nodes.job
                {
                    job.cancel.store(true, Ordering::Relaxed);
                }
            } else if let Some(message) = &app.nodes.message {
                ui.label(message);
            }
        });
    }
    ui.add_enabled_ui(!busy, |ui| actions = app.nodes.editor.show(ui));
    for action in actions {
        dispatch(app, ui.ctx(), action);
    }
}

fn dispatch(app: &mut SolveApp, ctx: &egui::Context, action: NodeAction) {
    match action {
        NodeAction::OpenJson => {
            let ctx = ctx.clone();
            app.pick_open_for(&["json"], move |app, path| {
                if let Some(path) = path
                    && let Err(error) = app.nodes.start(Work::Load(path), &ctx)
                {
                    app.nodes.editor.report_error(error);
                }
            });
        }
        NodeAction::SaveJson(text) => save(app, ctx, text, true),
        NodeAction::ExportGcode(text) => save(app, ctx, text, false),
        NodeAction::PreviewMesh(mesh) => {
            if app.file_load.busy() || app.file_picker_busy() {
                app.nodes.editor.report_error(tr("node.workspace.file_busy"));
                return;
            }
            app.nodes.next_preview = app.nodes.next_preview.wrapping_add(1);
            let stamp = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|time| time.as_nanos()).unwrap_or_default();
            let path = vfs::temp_dir().join(format!("arclet-nodes-{stamp}-{}.stl", app.nodes.next_preview));
            let path = path.to_string_lossy().to_string();
            let destination = Destination::of(app);
            if let Err(error) = app.nodes.start(Work::Preview { path, mesh, destination }, ctx) {
                app.nodes.editor.report_error(error);
            }
        }
    }
}

/// Use the same bounded, snapshot-aware save path as the node toolbar. A host's
/// close guard may request this without accessing the editor's private actions.
/// Cancellation or failure leaves the editor dirty; hosts must observe that state.
pub fn save_graph(app: &mut SolveApp, ctx: &egui::Context) -> Result<(), String> {
    if app.nodes.file_busy() || app.file_picker_busy() {
        return Err(tr("node.workspace.file_busy").to_string());
    }
    let text = app.nodes.editor.to_json()?;
    save(app, ctx, text, true);
    Ok(())
}

fn save(app: &mut SolveApp, ctx: &egui::Context, text: String, graph: bool) {
    let (name, extension) = if graph { ("model.nodes.json", "json") } else { ("preview-unverified.gcode", "gcode") };
    if let Some(download) = &app.services.download_design {
        match download(name, text.as_bytes()) {
            Ok(()) => {
                if graph {
                    match app.nodes.editor.mark_saved_json(&text) {
                        Ok(()) => app.nodes.message = Some(tr("node.workspace.saved").to_string()),
                        Err(error) => app.nodes.editor.report_error(error),
                    }
                } else {
                    app.nodes.message = Some(tr("node.workspace.saved").to_string());
                }
            }
            Err(error) => app.nodes.editor.report_error(error),
        }
        return;
    }
    let ctx = ctx.clone();
    app.pick_save(name, &[extension], move |app, path| {
        if let Some(path) = path
            && let Err(error) = app.nodes.start(Work::Save { path, text, graph }, &ctx)
        {
            app.nodes.editor.report_error(error);
        }
    });
}

/// Dialog-free automation for the same workspace and JSON document shown by the UI.
pub(crate) fn control(app: &mut SolveApp, params: &Value) -> Result<Value, String> {
    // Validate all input before changing workspace or document. Failed automation is atomic.
    let open = params.get("open").map(|value| value.as_bool().ok_or_else(|| tr("node.workspace.invalid_open").to_string())).transpose()?;
    let graph = params
        .get("graph")
        .map(|value| {
            let text = value.as_str().ok_or_else(|| tr("node.workspace.invalid_graph").to_string())?;
            let graph = solidcraft_graph::Graph::from_json(text).map_err(|error| error.to_string())?;
            // Also check the canonical saved-size limit before any workspace change.
            graph.to_json().map_err(|error| error.to_string())?;
            Ok::<_, String>(graph)
        })
        .transpose()?;
    if graph.is_some() && app.nodes.job.is_some() {
        return Err(tr("node.workspace.file_busy").to_string());
    }
    if let Some(open) = open {
        app.set_nodes_open(open)?;
    }
    if let Some(graph) = graph {
        app.nodes.editor.replace_graph(graph)?;
        app.nodes.editor.mark_unsaved();
        app.nodes.path = None;
        app.nodes.enabled = true;
    }
    Ok(
        json!({"open": app.nodes.open, "graph": app.nodes.editor.to_json()?, "running": app.nodes.editor.is_running(), "dirty": app.nodes.editor.is_dirty()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use solidcraft_engine::geom::Vec3;

    #[test]
    fn preview_stl_is_bounded_finite_and_cancellable() {
        let tri = [Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), Vec3::new(0.0, 10.0, 0.0)];
        let stop = AtomicBool::new(false);
        let bytes = mesh_stl(&[tri], &stop).unwrap();
        assert_eq!(bytes.len(), 134);
        assert_eq!(bytes.get(80..84), Some(1_u32.to_le_bytes().as_slice()));
        let mut bad = tri;
        bad[0].x = f64::INFINITY;
        assert!(mesh_stl(&[bad], &stop).is_err());
        stop.store(true, Ordering::Relaxed);
        assert!(mesh_stl(&[tri], &stop).is_err());
        assert!(mesh_stl(&[], &stop).is_err());
    }

    #[test]
    fn graph_recovery_preserves_dirty_state_without_changing_cad() {
        let mut app = SolveApp::new(solidcraft_engine::Session::new(Default::default()), Default::default());
        app.nodes.enabled = true;
        let before = app.session.revision;
        let graph = app.nodes.editor.to_json().unwrap();
        let preferences = app.prefs();
        let mut other = SolveApp::new(solidcraft_engine::Session::new(Default::default()), Default::default());
        other.load_prefs(&preferences);
        assert_eq!(other.nodes.editor.to_json().unwrap(), graph);
        assert!(other.nodes.editor.is_dirty());
        app.nodes.editor.mark_saved();
        other.load_prefs(&app.prefs());
        assert!(!other.nodes.editor.is_dirty());
        assert_eq!(app.session.revision, before);
    }

    #[test]
    fn invalid_control_graph_leaves_the_document_unchanged() {
        let mut app = SolveApp::new(solidcraft_engine::Session::new(Default::default()), Default::default());
        let before = app.nodes.editor.to_json().unwrap();
        assert!(control(&mut app, &json!({"open":true, "graph":"not JSON"})).is_err());
        assert!(!app.nodes.open);
        assert_eq!(app.nodes.editor.to_json().unwrap(), before);
        assert!(control(&mut app, &json!({"open":"yes"})).is_err());
        assert!(!app.nodes.open);
        control(&mut app, &json!({"open": true})).unwrap();
        assert!(app.nodes.open);
    }

    #[test]
    fn committed_save_is_reported_even_if_cancel_arrived_after_writing() {
        let mut app = SolveApp::new(Default::default(), Default::default());
        let text = app.nodes.editor.to_json().unwrap();
        let (send, result) = std::sync::mpsc::channel();
        send.send(Ok(Completed::Saved { path: "graph.json".into(), text, graph: true })).ok();
        app.nodes.job = Some(Job { result, cancel: Arc::new(AtomicBool::new(true)) });
        tick(&mut app, &egui::Context::default());
        assert!(!app.nodes.editor.is_dirty());
        assert_eq!(app.nodes.path.as_deref(), Some("graph.json"));
        assert_eq!(app.nodes.message.as_deref(), Some(tr("node.workspace.saved")));
    }

    #[test]
    fn prepared_mesh_cannot_land_in_a_different_design() {
        let mut app = SolveApp::new(Default::default(), Default::default());
        app.nodes.open = true;
        let destination = Destination::of(&app);
        app.session = Default::default();
        let path = vfs::temp_dir().join(format!("arclet-preview-test-{}.stl", app.session.uid)).to_string_lossy().into_owned();
        write(&path, b"temporary").unwrap();
        let (send, result) = std::sync::mpsc::channel();
        send.send(Ok(Completed::Preview { file: Arc::new(TemporaryMesh { path: path.clone() }), destination })).ok();
        app.nodes.job = Some(Job { result, cancel: Arc::new(AtomicBool::new(false)) });
        tick(&mut app, &egui::Context::default());
        assert!(app.nodes.open);
        assert!(!app.file_load.busy());
        assert!(vfs::read(&path, 1024).is_err());
    }

    #[test]
    fn automation_replacement_is_an_unsaved_graph_with_no_old_path() {
        let mut app = SolveApp::new(Default::default(), Default::default());
        app.nodes.path = Some("old.json".into());
        app.nodes.editor.mark_saved();
        let graph = solidcraft_graph::Graph::new().to_json().unwrap();
        let response = control(&mut app, &json!({"graph":graph})).unwrap();
        assert_eq!(response["dirty"], true);
        assert!(app.nodes.path.is_none());
    }

    #[test]
    fn an_unpolled_preview_cleans_its_file_when_the_host_closes() {
        let app = SolveApp::new(Default::default(), Default::default());
        let path = vfs::temp_dir().join(format!("arclet-unpolled-test-{}.stl", app.session.uid)).to_string_lossy().into_owned();
        write(&path, b"temporary").unwrap();
        let (send, receive) = std::sync::mpsc::channel();
        send.send(Completed::Preview { file: Arc::new(TemporaryMesh { path: path.clone() }), destination: Destination::of(&app) }).ok();
        drop(receive);
        assert!(vfs::read(&path, 1024).is_err());
    }

    #[test]
    fn inserted_node_mesh_survives_staging_cleanup_and_a_cold_document_reload() {
        let mut app = SolveApp::new(Default::default(), Default::default());
        let a = Vec3::ZERO;
        let b = Vec3::X * 10.0;
        let c = Vec3::Y * 10.0;
        let d = Vec3::Z * 10.0;
        let mesh = Arc::new(vec![[a, c, b], [a, b, d], [a, d, c], [b, c, d]]);
        let path = vfs::temp_dir().join(format!("arclet-roundtrip-test-{}.stl", app.session.uid)).to_string_lossy().into_owned();
        let work = Work::Preview { path: path.clone(), mesh, destination: Destination::of(&app) };
        let Completed::Preview { file, .. } = perform(work, &AtomicBool::new(false)).unwrap() else { panic!("a prepared mesh was required") };
        app.run("file.insert_mesh", json!({"path":path})).unwrap();
        let bytes = solidcraft_engine::io::write_design(&app.session.doc);
        drop(file);
        assert!(vfs::read(&path, 1024).is_err(), "the temporary file is gone");

        let document = solidcraft_engine::io::read_design(&bytes).unwrap();
        let reopened = solidcraft_engine::Session::new(document);
        let model = reopened.world_state();
        assert_eq!(model.bodies.len(), 1, "geometry is embedded in the document");
        let bounds = model.bodies[0].mesh().bounds();
        assert_eq!(bounds.min, Vec3::ZERO);
        assert_eq!(bounds.max, Vec3::new(10.0, 10.0, 10.0));
    }
    #[test]
    fn host_can_observe_pending_file_work_before_closing() {
        let mut app = SolveApp::new(Default::default(), Default::default());
        app.nodes.editor.mark_saved();
        assert!(!app.nodes.file_busy());
        let (_send, result) = std::sync::mpsc::channel();
        app.nodes.job = Some(Job { result, cancel: Arc::new(AtomicBool::new(false)) });
        assert!(app.nodes.file_busy());
        assert!(save_graph(&mut app, &egui::Context::default()).is_err());
        assert!(!app.nodes.editor.is_dirty());
    }

    #[test]
    fn failed_close_save_never_marks_the_graph_saved() {
        let services = crate::Services { download_design: Some(Box::new(|_, _| Err("disk unavailable".into()))), ..Default::default() };
        let mut app = SolveApp::new(Default::default(), services);
        assert!(app.nodes.editor.is_dirty());
        save_graph(&mut app, &egui::Context::default()).unwrap();
        assert!(app.nodes.editor.is_dirty());
    }

    #[test]
    fn recovered_dirty_flag_wins_even_when_contents_equal_a_saved_starter() {
        let mut state = State::default();
        state.enabled = true;
        let text = state.editor.to_json().unwrap();
        state.editor.mark_saved();
        state.restore_state(&json!({"graph":text, "path":"other.json", "dirty":true}));
        assert!(state.editor.is_dirty());
        assert_eq!(state.path.as_deref(), Some("other.json"));
    }

    #[test]
    fn adopting_the_same_graph_without_a_file_is_still_unsaved() {
        let mut app = SolveApp::new(Default::default(), Default::default());
        app.nodes.editor.mark_saved();
        let graph = app.nodes.editor.to_json().unwrap();
        control(&mut app, &json!({"graph":graph})).unwrap();
        assert!(app.nodes.editor.is_dirty());
        assert!(app.nodes.path.is_none());
    }
}
