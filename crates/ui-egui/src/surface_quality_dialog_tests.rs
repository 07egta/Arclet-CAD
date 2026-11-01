//! Match/Fair are real feature forms: chosen constraints become engine parameters, survive
//! Edit Feature, and make geometry through the existing surface kernel.
use super::*;
use crate::dialogs::{Dialog, apply_commands};
use crate::scenario::Harness;
use solidcraft_engine::doc::{IDENTITY, kernel::Continuity};

/// Stop inside the kernel, after command dispatch, so the feature transaction must roll back.
struct StopAfter {
    checks: std::sync::atomic::AtomicUsize,
    progress: std::sync::Arc<solidcraft_engine::doc::progress::Progress>,
}

impl solidcraft_engine::kernel::watch::Watch for StopAfter {
    fn cancelled(&self) -> bool {
        if self.checks.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= 50 {
            self.progress.cancel();
        }
        self.progress.is_cancelled()
    }
    fn progress(&self, _: &'static str, _: usize, _: usize) {}
}

fn cancelled_command_preserves_document(app: &mut SolveApp, command: &str, parameters: &Value) {
    let before = app.session.doc.to_json();
    // The real UI cancels the document Progress, which also tells the shared model cache not
    // to retain an interrupted feature. Trigger that same signal at a deterministic checkpoint.
    let progress = std::sync::Arc::new(solidcraft_engine::doc::progress::Progress::default());
    let watcher = std::sync::Arc::new(StopAfter { checks: std::sync::atomic::AtomicUsize::new(0), progress: progress.clone() });
    let error = solidcraft_engine::doc::progress::watch(progress.clone(), || {
        solidcraft_engine::kernel::watch::watching(watcher, || app.run(command, parameters.clone()))
    })
    .unwrap_err();
    assert!(progress.is_cancelled());
    assert!(error.contains("cancelled"), "{error}");
    assert_eq!(app.session.doc.to_json(), before, "a cancelled result must not enter the document");
}

fn app() -> SolveApp {
    SolveApp::new(solidcraft_engine::Session::default(), crate::Services::default())
}

fn match_form(app: &SolveApp, order: usize) -> Dialog {
    let mut dialog = Dialog::for_command(app, "surface.match").unwrap();
    dialog.kind = Kind::Surface(Surf::Match { continuity: order });
    dialog.inputs[0].items = vec![Sel::Face { body: "Source".into(), index: 0, point: Vec3::new(0.0, 1.0, 2.0) }];
    dialog.inputs[1].items = vec![Sel::Edge { body: "Target".into(), index: 0, point: Vec3::new(0.0, 0.0, 2.0) }];
    dialog
}

#[test]
fn match_payload_preserves_source_target_and_all_three_orders() {
    let app = app();
    for (index, order) in ["g1", "g2", "g3"].into_iter().enumerate() {
        let dialog = match_form(&app, index);
        assert_eq!(dialog.command(), Some("surface.match"));
        let commands = apply_commands(&app, &dialog).unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].0, "surface.match");
        assert_eq!(commands[0].1, json!({"body":"Source", "faces":[[0.0,1.0,2.0]], "target":"Target", "edges":[[0.0,0.0,2.0]], "order":order}));
    }
    let mut dialog = match_form(&app, 3);
    assert!(apply_commands(&app, &dialog).is_err(), "an unknown order must not silently become G1");
    dialog.kind = Kind::Surface(Surf::Match { continuity: 1 });
    dialog.inputs[1].items.clear();
    assert!(apply_commands(&app, &dialog).is_err());
    dialog.inputs[1].items = vec![Sel::Edge { body: "Source".into(), index: 0, point: Vec3::ZERO }];
    assert!(apply_commands(&app, &dialog).is_err(), "the target must be a different body");
    dialog.inputs[1].items = vec![Sel::Edge { body: "Target".into(), index: 0, point: Vec3::new(f64::NAN, 0.0, 0.0) }];
    assert!(apply_commands(&app, &dialog).is_err(), "a malformed point must not disappear in JSON");
}

#[test]
fn fair_payload_keeps_units_optional_tolerance_and_validates_limits() {
    let mut app = app();
    let mut dialog = Dialog::for_command(&app, "surface.fair").unwrap();
    assert_eq!(dialog.command(), Some("surface.fair"));
    dialog.inputs[0].items = vec![Sel::Body { name: "Sheet".into() }];
    let parameters = &apply_commands(&app, &dialog).unwrap()[0].1;
    assert!(parameters.get("tolerance").is_none(), "an untouched field uses the body's adaptive tolerance");
    assert_eq!(parameters["keep_edges"], true);
    std::sync::Arc::make_mut(&mut app.session.doc).units = "in".into();
    dialog.kind = Kind::Surface(Surf::Fair { strength: 0.5, iterations: 4, tolerance: "0.02".into(), keep_edges: false });
    let parameters = &apply_commands(&app, &dialog).unwrap()[0].1;
    assert_eq!(parameters["strength"], 0.5);
    assert_eq!(parameters["iterations"], 4);
    assert_eq!(parameters["keep_edges"], false);
    assert_eq!(parameters["tolerance"], "0.02 in");
    for invalid in [
        Surf::Fair { strength: 0.0, iterations: 1, tolerance: String::new(), keep_edges: true },
        Surf::Fair { strength: f64::NAN, iterations: 1, tolerance: String::new(), keep_edges: true },
        Surf::Fair { strength: 1.1, iterations: 1, tolerance: String::new(), keep_edges: true },
        Surf::Fair { strength: 1.0, iterations: 0, tolerance: String::new(), keep_edges: true },
        Surf::Fair { strength: 1.0, iterations: 1001, tolerance: String::new(), keep_edges: true },
        Surf::Fair { strength: 1.0, iterations: 1, tolerance: "0 mm".into(), keep_edges: true },
    ] {
        dialog.kind = Kind::Surface(invalid);
        assert!(apply_commands(&app, &dialog).is_err());
    }
}

#[test]
fn match_and_fair_open_visible_forms_and_expose_their_controls() {
    let mut h = Harness::new();
    h.step(&json!({"call":"ui.start", "params":{"command":"surface.match"}})).unwrap();
    h.frames(3);
    for label in ["Source surface", "Target edges", "Continuity"] {
        assert!(h.widget(label).is_some(), "missing visible {label}");
    }
    h.step(&json!({"widget":"Curvature (G2)"})).unwrap();
    h.step(&json!({"widget":"Curvature change (G3)"})).unwrap();
    assert!(matches!(h.app.dialog.as_ref().unwrap().kind, Kind::Surface(Surf::Match { continuity: 2 })));
    h.step(&json!({"widget":"Cancel"})).unwrap();
    h.step(&json!({"call":"ui.start", "params":{"command":"surface.fair"}})).unwrap();
    h.frames(3);
    for label in ["Source surface", "Strength", "Iterations", "Tolerance", "Keep edge tangents"] {
        assert!(h.widget(label).is_some(), "missing visible {label}");
    }
}

fn matching_fixture(app: &mut SolveApp) -> Vec<Vec3> {
    app.run("sketch.create", json!({"plane":"XY", "name":"Round"})).unwrap();
    app.run("sketch.circle.center", json!({"center":[0,0], "radius":10})).unwrap();
    app.run("sketch.finish", json!({})).unwrap();
    app.run("solid.extrude", json!({"sketch":"Round", "distance":30, "body_name":"Target"})).unwrap();
    let body = app.session.world_state().body("Target").unwrap().body.clone();
    let edge = body
        .edges(body.size() * 0.01)
        .unwrap()
        .into_iter()
        .find(|edge| edge.length > 25.0 && (edge.mid.x.hypot(edge.mid.y) - 10.0).abs() < 0.05 && (5.0..25.0).contains(&edge.mid.z))
        .unwrap();
    let (a, b) = (*edge.points.first().unwrap(), *edge.points.last().unwrap());
    let picks = vec![a + (b - a) * 0.25, a + (b - a) * 0.75];
    let angle = picks[0].y.atan2(picks[0].x);
    let at = |t: f64| json!([11.0 * t.cos(), 11.0 * t.sin()]);
    app.run("sketch.create", json!({"plane":"XY", "name":"Sheet sketch"})).unwrap();
    let arc = app
        .run("sketch.arc.three_point", json!({"start":at(angle), "end":at(angle+60f64.to_radians()), "through":at(angle+30f64.to_radians())}))
        .unwrap()["created"]["curves"][0]
        .as_str()
        .unwrap()
        .to_string();
    app.run("sketch.finish", json!({})).unwrap();
    app.run("surface.extrude", json!({"sketch":"Sheet sketch", "curves":[arc], "distance":30, "body_name":"Source"})).unwrap();
    picks
}

#[test]
fn g3_match_form_builds_a_surface_and_restores_body_scoped_picks() {
    let mut app = app();
    let picks = matching_fixture(&mut app);
    let mut dialog = match_form(&app, 2);
    dialog.inputs[0].items = vec![Sel::Body { name: "Source".into() }];
    dialog.inputs[1].items = picks.iter().enumerate().map(|(index, point)| Sel::Edge { body: "Target".into(), index, point: *point }).collect();
    for (command, parameters) in apply_commands(&app, &dialog).unwrap() {
        cancelled_command_preserves_document(&mut app, &command, &parameters);
        app.run(&command, parameters).unwrap();
    }
    let state = app.session.world_state();
    let result = &state.body("Source").unwrap().body;
    let target = &state.body("Target").unwrap().body;
    let (gap, angle, order) = solidcraft_engine::kernel::seam_continuity(result, target, &picks).unwrap();
    assert!(gap < 1e-6 && angle < 0.05, "gap {gap}, angle {angle}");
    assert_eq!(order, Some(Continuity::G3));
    let feature = &app.session.doc.features.last().unwrap().kind;
    let (kind, restored, _) = for_feature(&app, feature, &IDENTITY).unwrap();
    assert!(matches!(kind, Kind::Surface(Surf::Match { continuity: 2 })));
    assert!(
        restored[1].items.iter().all(|pick| matches!(pick, Sel::Edge { body, .. } if body == "Target")),
        "coincident edges must stay on the target body"
    );
}

#[test]
fn fair_form_runs_on_a_bspline_and_keeps_its_boundary_and_edit_values() {
    let mut app = app();
    app.run("solid.sphere", json!({"radius":20, "body_name":"Ball"})).unwrap();
    app.run("surface.offset", json!({"faces":[[0,0,20]], "body":"Ball", "distance":2, "body_name":"Sheet"})).unwrap();
    let boundary = |app: &SolveApp| -> Vec<Vec3> {
        app.session.world_state().body("Sheet").unwrap().body.edges(0.1).unwrap().into_iter().flat_map(|edge| edge.points).collect()
    };
    let before = boundary(&app);
    let mut dialog = Dialog::for_command(&app, "surface.fair").unwrap();
    dialog.kind = Kind::Surface(Surf::Fair { strength: 0.75, iterations: 3, tolerance: "1 mm".into(), keep_edges: true });
    dialog.inputs[0].items = vec![Sel::Body { name: "Sheet".into() }];
    for (command, parameters) in apply_commands(&app, &dialog).unwrap() {
        cancelled_command_preserves_document(&mut app, &command, &parameters);
        app.run(&command, parameters).unwrap();
    }
    assert!(!before.is_empty());
    assert_eq!(boundary(&app), before);
    let feature = &app.session.doc.features.last().unwrap().kind;
    let (kind, restored, _) = for_feature(&app, feature, &IDENTITY).unwrap();
    let Kind::Surface(restored_kind) = kind else { panic!("restored a non-surface dialog") };
    let Kind::Surface(expected_kind) = &dialog.kind else { panic!("fixture is not a surface dialog") };
    assert_eq!(&restored_kind, expected_kind);
    assert_eq!(restored[0].items, dialog.inputs[0].items);
    assert!(matches!(feature, FeatureKind::SurfaceFair { strength: 0.75, iterations: 3, keep_edges: true, .. }));
}

#[test]
fn match_restoration_uses_the_targets_own_component_frame() {
    let mut app = app();
    let source = app.run("component.create", json!({"name":"Source part", "grounded":false})).unwrap();
    app.run("solid.box", json!({"corner":[0,0,0], "length":10, "width":10, "height":10, "body_name":"Source"})).unwrap();
    app.run("component.activate", json!({"component":"root"})).unwrap();
    let target = app.run("component.create", json!({"name":"Target part", "grounded":false})).unwrap();
    app.run("solid.box", json!({"corner":[0,0,0], "length":10, "width":10, "height":10, "body_name":"Target"})).unwrap();
    app.run("occurrence.move", json!({"occurrence":source["occurrence"], "translate":[20,0,0]})).unwrap();
    app.run("occurrence.move", json!({"occurrence":target["occurrence"], "translate":[100,0,0]})).unwrap();
    let mut source_frame = IDENTITY;
    source_frame[3][0] = 20.0;
    let feature = FeatureKind::SurfaceMatch {
        body: "Source".into(),
        faces: Vec::new(),
        target: "Target".into(),
        edges: vec![Position(Vec3::new(0.0, 0.0, 5.0))],
        order: Continuity::G2,
    };
    let (_, inputs, _) = for_feature(&app, &feature, &source_frame).unwrap();
    assert!(matches!(&inputs[1].items[0], Sel::Edge { body, point, .. } if body == "Target" && *point == Vec3::new(100.0, 0.0, 5.0)));
}
