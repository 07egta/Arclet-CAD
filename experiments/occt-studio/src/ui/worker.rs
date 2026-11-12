//! OCCT handles never leave this thread. UI messages contain copied numbers and mesh Arcs only.
use crate::{
    kernel::{Mesh, Shape, Stats},
    model::{Layer, Outcome, Session},
    Result,
};
use eframe::egui;
use std::{
    collections::BTreeMap,
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    time::Instant,
};

#[derive(Clone)]
pub struct DisplayObject {
    pub id: u64,
    pub name: String,
    pub layer: u64,
    pub visible: bool,
    pub locked: bool,
    pub color: [u8; 3],
    pub stats: Stats,
    pub mesh: Option<Arc<Mesh>>,
}
#[derive(Clone, Default)]
pub struct Scene {
    pub revision: u64,
    pub dirty: bool,
    pub undo: usize,
    pub redo: usize,
    pub current_layer: u64,
    pub objects: Vec<DisplayObject>,
    pub layers: Vec<Layer>,
    pub warnings: Vec<String>,
}
impl Scene {
    pub fn bounds(&self, ids: &[u64]) -> Option<[f64; 6]> {
        let mut bounds: Option<[f64; 6]> = None;
        for o in self
            .objects
            .iter()
            .filter(|o| o.visible && (ids.is_empty() || ids.contains(&o.id)))
        {
            if let Some(b) = &mut bounds {
                for i in 0..3 {
                    b[i] = b[i].min(o.stats.bounds[i]);
                    b[i + 3] = b[i + 3].max(o.stats.bounds[i + 3]);
                }
            } else {
                bounds = Some(o.stats.bounds);
            }
        }
        bounds
    }
    pub fn triangles(&self) -> usize {
        self.objects
            .iter()
            .filter(|o| o.visible)
            .filter_map(|o| o.mesh.as_ref())
            .map(|m| m.indices.len() / 3)
            .sum()
    }
}
pub enum Request {
    Command(String),
    Batch(Vec<String>),
}
pub struct Event {
    pub scene: Arc<Scene>,
    pub result: Result<Outcome>,
    pub elapsed_ms: u128,
}
struct Cached {
    shape: Shape,
    stats: Stats,
    mesh: Arc<Mesh>,
}
fn scene(session: &Session, cache: &mut BTreeMap<u64, Cached>) -> Scene {
    cache.retain(|id, _| session.state().objects.contains_key(id));
    let state = session.state();
    let mut result = Scene {
        revision: session.revision(),
        dirty: session.dirty(),
        undo: session.undo_len(),
        redo: session.redo_len(),
        current_layer: state.current_layer,
        layers: state.layers.values().cloned().collect(),
        ..Default::default()
    };
    let mut vertices = 0usize;
    let mut triangles = 0usize;
    let mut segments = 0usize;
    for o in state.objects.values() {
        let stats = match cache
            .get(&o.id)
            .filter(|c| c.shape.shares_geometry(&o.shape))
        {
            Some(c) => c.stats,
            None => match o.shape.stats() {
                Ok(s) => s,
                Err(e) => {
                    result
                        .warnings
                        .push(format!("Object #{} measurement failed: {e}", o.id));
                    Stats::default()
                }
            },
        };
        let visible = state.visible(o);
        let mut display = DisplayObject {
            id: o.id,
            name: o.name.clone(),
            layer: o.layer,
            visible,
            locked: state.locked(o),
            color: state
                .layers
                .get(&o.layer)
                .map_or([150, 170, 190], |l| l.color),
            stats,
            mesh: None,
        };
        if visible {
            if !cache
                .get(&o.id)
                .is_some_and(|c| c.shape.shares_geometry(&o.shape))
            {
                let diagonal = ((stats.bounds[3] - stats.bounds[0]).powi(2)
                    + (stats.bounds[4] - stats.bounds[1]).powi(2)
                    + (stats.bounds[5] - stats.bounds[2]).powi(2))
                .sqrt();
                match o.shape.mesh((diagonal / 700.0).clamp(0.005, 1000.0)) {
                    Ok(mesh) => {
                        cache.insert(
                            o.id,
                            Cached {
                                shape: o.shape.clone(),
                                stats,
                                mesh: Arc::new(mesh),
                            },
                        );
                    }
                    Err(e) => {
                        result.warnings.push(format!(
                            "Object #{} display failed; exact geometry retained: {e}",
                            o.id
                        ));
                        cache.remove(&o.id);
                    }
                }
            }
            if let Some(c) = cache.get(&o.id) {
                let m = &c.mesh;
                if vertices + m.positions.len() <= 1_000_000
                    && triangles + m.indices.len() / 3 <= 1_000_000
                    && segments + m.lines.len() <= 500_000
                {
                    vertices += m.positions.len();
                    triangles += m.indices.len() / 3;
                    segments += m.lines.len();
                    display.mesh = Some(m.clone());
                } else {
                    result.warnings.push(format!("Object #{} is not displayed: scene display budget reached. Its B-rep is retained.", o.id));
                }
            }
        }
        // A rejected mesh must not remain in the cache, otherwise many oversized
        // objects could defeat the total display-memory budget.
        if display.mesh.is_none() {
            cache.remove(&o.id);
        }
        result.objects.push(display);
    }
    // Do not retain meshes for hidden objects: the document, not the display cache, owns geometry.
    cache.retain(|id, _| state.objects.get(id).is_some_and(|o| state.visible(o)));
    result
}
pub fn start(context: egui::Context) -> (SyncSender<Request>, Receiver<Event>) {
    let (tx, rx) = mpsc::sync_channel(1);
    let (events, incoming) = mpsc::channel();
    std::thread::spawn(move || {
        let mut session = Session::default();
        let mut cache = BTreeMap::new();
        while let Ok(request) = rx.recv() {
            let started = Instant::now();
            let result = match request {
                Request::Command(line) => session.execute(&line),
                Request::Batch(lines) => session.batch(&lines),
            };
            let snapshot = Arc::new(scene(&session, &mut cache));
            if events
                .send(Event {
                    scene: snapshot,
                    result,
                    elapsed_ms: started.elapsed().as_millis(),
                })
                .is_err()
            {
                break;
            }
            context.request_repaint();
        }
    });
    (tx, incoming)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worker_recovers_from_invalid_command_and_preserves_geometry() {
        let (tx, rx) = start(egui::Context::default());
        tx.send(Request::Command("Box 0,0,0 10 10 10".into()))
            .unwrap();
        let first = rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap();
        assert!(first.result.is_ok());
        assert_eq!(first.scene.objects.len(), 1);
        assert!(first.scene.objects[0].mesh.is_some());
        tx.send(Request::Command("FilletEdge 1 100000".into()))
            .unwrap();
        let bad = rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap();
        assert!(bad.result.is_err());
        assert_eq!(bad.scene.revision, first.scene.revision);
        assert!(Arc::ptr_eq(
            first.scene.objects[0].mesh.as_ref().unwrap(),
            bad.scene.objects[0].mesh.as_ref().unwrap()
        ));
        tx.send(Request::Command("Move 1 10,0,0".into())).unwrap();
        let moved = rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap();
        assert!(moved.result.is_ok());
        assert!(moved.scene.objects[0].stats.bounds[0] > 9.9);
    }
    #[test]
    fn hidden_objects_keep_metadata_but_not_display_meshes() {
        let mut session = Session::default();
        session.execute("Box 0,0,0 10 10 10").unwrap();
        let mut cache = BTreeMap::new();
        assert!(scene(&session, &mut cache).objects[0].mesh.is_some());
        session.execute("Hide 1").unwrap();
        let view = scene(&session, &mut cache);
        assert!(!view.objects[0].visible);
        assert!(view.objects[0].mesh.is_none());
        assert!(cache.is_empty());
        assert_eq!(session.state().objects.len(), 1);
    }
}
