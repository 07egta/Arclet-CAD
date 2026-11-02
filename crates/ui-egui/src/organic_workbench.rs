//! Opt-in bounded T-spline control-point workshop. This is a standalone
//! in-memory editing surface, NOT a persisted CAD feature or a SubD converter.
//! Mouse handles edit Cartesian controls; the same validated operations are
//! tested in solidcraft-geom. No engineering model is overwritten here.
use egui::{Color32, Pos2, Sense, Stroke, Vec2};
use solidcraft_engine::geom::{Vec3, tspline::{TSpline, TMesh, Cut}};

const KEY: &str = "arclet.organic.workshop.state";
const SIZE: f32 = 360.0;

#[derive(Clone)]
struct State {
    open: bool,
    spline: Option<TSpline>,
    selected: Vec<usize>,
    undo: Vec<TSpline>,
    redo: Vec<TSpline>,
    error: String,
    z_mm: f64,
}
impl Default for State {
    fn default() -> Self {
        let knots = (0..7).map(|i| i as f64).collect::<Vec<_>>();
        let spline = TMesh::tensor(knots.clone(), knots).ok().and_then(|mesh| {
            let (points, weights): (Vec<_>, Vec<_>) = mesh.vertices().iter()
                .map(|&(i,j)| (Vec3::new((i as f64 - 3.0)*10.0, (j as f64 - 3.0)*10.0, if i==3 && j==3 { 9.0 } else { 0.0 }), 1.0))
                .unzip();
            TSpline::new(mesh, [3,3], &points, &weights).ok()
        });
        Self { open: false, spline, selected: Vec::new(), undo: Vec::new(), redo: Vec::new(), error: String::new(), z_mm: 2.0 }
    }
}
impl State {
    fn apply(&mut self, next: Result<TSpline, String>) {
        match next {
            Ok(next) => {
                if let Some(current) = &self.spline {
                    if *current != next {
                        if self.undo.len() == 32 { self.undo.remove(0); }
                        self.undo.push(current.clone());
                        self.redo.clear();
                    }
                }
                self.spline = Some(next);
                self.selected.retain(|&i| self.spline.as_ref().is_some_and(|s| i < s.points().len()));
                self.error.clear();
            }
            Err(error) => self.error = error,
        }
    }
    fn history(&mut self, backwards: bool) {
        let (source, destination) = if backwards { (&mut self.undo, &mut self.redo) } else { (&mut self.redo, &mut self.undo) };
        if let Some(previous) = source.pop() {
            if let Some(current) = self.spline.replace(previous) { destination.push(current); }
            self.selected.clear();
            self.error.clear();
        }
    }
}
fn screen(x: f64, y: f64, rect: egui::Rect) -> Pos2 {
    let center = rect.center();
    center + Vec2::new(x as f32 * 3.7, -y as f32 * 3.7)
}
fn from_screen(at: Pos2, rect: egui::Rect) -> (f64, f64) {
    let c = rect.center();
    ((at.x - c.x) as f64 / 3.7, (c.y - at.y) as f64 / 3.7)
}
/// The launcher is deliberately separate from normal CAD feature/history tools.
/// Do not describe a workshop edit as mutating a saved body.
pub fn show(ui: &mut egui::Ui) {
    let id = egui::Id::new(KEY);
    let mut state: State = ui.ctx().data_mut(|data| data.remove_temp(id)).unwrap_or_default();
    if ui.small_button("Organic T-Spline Workshop").on_hover_text("Experimental control-net editor; independent of saved CAD bodies").clicked() {
        state.open = true;
    }
    if state.open {
        let mut opened = state.open;
        egui::Window::new("Organic T-Spline — experimental")
            .id(egui::Id::new("organic-tspline-window"))
            .open(&mut opened).resizable(true).show(ui.ctx(), |ui| {
                ui.label("Independent T-Spline control net, not a document body or arbitrary-topology SubD.");
                ui.label("Click points; Shift-click adds selection. Drag a handle to move selected points in XY.");
                if state.spline.is_none() { ui.label("Could not create the example spline."); return; }
                let rect = ui.allocate_exact_size(Vec2::splat(SIZE), Sense::hover()).0;
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
                let Some(current) = state.spline.clone() else { return; };
                let domain = current.domain();
                // Evaluated surface is shown as parametric isocurves; the
                // T-grid below is a control mesh, not the output surface.
                for k in 0..=16 {
                    for dir in 0..2 {
                        let mut previous = None;
                        for l in 0..=32 {
                            let u = domain[0][0] + (domain[0][1]-domain[0][0]) * (if dir == 0 {k as f64 / 16.0} else {l as f64 / 32.0});
                            let v = domain[1][0] + (domain[1][1]-domain[1][0]) * (if dir == 0 {l as f64 / 32.0} else {k as f64 / 16.0});
                            if let Ok(p) = current.eval(u,v) {
                                let point = screen(p.x,p.y,rect);
                                if let Some(last) = previous { painter.line_segment([last,point], Stroke::new(0.6,Color32::from_rgb(75,150,205))); }
                                previous = Some(point);
                            }
                        }
                    }
                }
                let points = current.points();
                for &(i,j) in current.mesh().vertices() {
                    if let Some(k) = current.mesh().vertex_at(i,j) {
                        if let Some(p) = points.get(k) {
                            for adjacent in [(i+1,j),(i,j+1)] {
                                if let Some(other) = current.mesh().vertex_at(adjacent.0,adjacent.1).and_then(|idx| points.get(idx)) {
                                    painter.line_segment([screen(p.x,p.y,rect),screen(other.x,other.y,rect)],Stroke::new(0.8,Color32::GRAY));
                                }
                            }
                        }
                    }
                }
                let mut drag = None;
                for (k,p) in points.iter().enumerate() {
                    let center = screen(p.x,p.y,rect);
                    let hit = egui::Rect::from_center_size(center,Vec2::splat(13.0));
                    let response = ui.interact(hit,ui.id().with(("organic-control",k)),Sense::click_and_drag());
                    if response.clicked() {
                        let multi = ui.input(|input| input.modifiers.shift);
                        if !multi { state.selected.clear(); }
                        if !state.selected.contains(&k) {state.selected.push(k);}
                    }
                    if response.dragged() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            let (x,y) = from_screen(pos,rect);
                            drag = Some((k,Vec3::new(x-p.x,y-p.y,0.0)));
                        }
                    }
                    painter.circle_filled(center,if state.selected.contains(&k){5.0}else{3.5},if state.selected.contains(&k){Color32::YELLOW}else{Color32::LIGHT_GRAY});
                }
                if let Some((k,delta))=drag {
                    let selection = if state.selected.contains(&k) {state.selected.clone()}else{vec![k]};
                    // Drag is a preview operation in this in-memory workshop;
                    // each frame stays undoable (bounded to 32 prior states).
                    state.apply(current.translated_controls(&selection,delta).map_err(|e|e.to_string()));
                }
                ui.horizontal(|ui| {
                    ui.label(format!("{} selected / {} controls",state.selected.len(),points.len()));
                    if ui.button("Select all").clicked(){state.selected=(0..points.len()).collect();}
                    if ui.button("Clear").clicked(){state.selected.clear();}
                });
                ui.horizontal(|ui| {
                    ui.label("Z displacement (mm)");
                    ui.add(egui::DragValue::new(&mut state.z_mm).speed(0.1).range(-100.0..=100.0));
                    if ui.button("Move selection Z").clicked() {
                        state.apply(current.translated_controls(&state.selected,Vec3::new(0.0,0.0,state.z_mm)).map_err(|e|e.to_string()));
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Undo").clicked(){state.history(true);}
                    if ui.button("Redo").clicked(){state.history(false);}
                    if ui.button("Refine center (exact)").clicked() {
                        let mid=(domain[0][0]+domain[0][1])*0.5;
                        state.apply(current.verified_refine(&[Cut::S{s:mid,t0:domain[1][0],t1:domain[1][1]}],17,1e-8)
                            .map(|(s,_)|s).map_err(|e|e.to_string()));
                    }
                });
                if !state.error.is_empty(){ui.colored_label(Color32::LIGHT_RED,&state.error);}
                ui.label("Workshop is not saved into .sc; T-mesh persistence and model integration are pending.");
            });
        state.open=opened;
    }
    ui.ctx().data_mut(|data| data.insert_temp(id,state));
}
