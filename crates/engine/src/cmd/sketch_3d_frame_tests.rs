use crate::Session;
use serde_json::{Value, json};
use solidcraft_geom::Vec3;
use solidcraft_sketch::WireSpace;

fn run(s: &mut Session, cmd: &str, params: Value) -> Value {
    s.execute(cmd, &params).unwrap_or_else(|e| panic!("{cmd}: {e}"))
}

fn near(actual: Vec3, expected: Vec3) {
    assert!(actual.dist(expected) < 1e-7, "{actual:?} != {expected:?}");
}

fn second_frame() -> (Session, u64, u64, u64) {
    let mut s = Session::default();
    let c = run(&mut s, "component.create", json!({"name":"Wire owner"}));
    let component = c["component"].as_u64().unwrap();
    let first = c["occurrence"].as_u64().unwrap();
    let second = run(&mut s, "occurrence.copy", json!({"occurrence":first}))["occurrence"].as_u64().unwrap();
    // One rigid transform: (x,y,z) -> (20-y,30+x,40+z), independent of implementation math.
    run(&mut s, "occurrence.move", json!({"occurrence":second,"axis":[0,0,1],"angle":"90 deg","translate":[20,30,40]}));
    run(&mut s, "component.activate", json!({"component":component,"occurrence":second,"path":[second]}));
    run(&mut s, "sketch.create", json!({"plane":"XY","name":"Space path"}));
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    (s, component, first, second)
}

#[test]
fn wire_frame_second_occurrence_world_inputs_move_and_reopen() {
    let (mut s, component, first, second) = second_frame();
    let sketch = s.active_sketch.unwrap();
    let r = run(&mut s, "sketch.line3d", json!({"points":[[17,32,45],[17,32,55],[14,32,55]]}));
    let a = r["curves"][0].as_str().unwrap().to_string();
    let b = r["curves"][1].as_str().unwrap().to_string();
    let spline = run(&mut s, "sketch.spline3d", json!({"points":[[14,32,55],[12,34,56],[10,35,58]]}))["curves"][0].as_str().unwrap().to_string();
    let point = run(&mut s, "sketch.point3d", json!({"point":[17,32,45]}))["point"].as_str().unwrap().to_string();
    let sk = s.doc.sketch(sketch).unwrap();
    for w in &sk.wires {
        assert_eq!(w.space, WireSpace::Component);
    }
    near(sk.wires[sk.wire_index(&a).unwrap()].fit[0], Vec3::new(2.0, 3.0, 5.0));
    near(sk.wires[sk.wire_index(&point).unwrap()].fit[0], Vec3::new(2.0, 3.0, 5.0));
    let world = s.world_state();
    let ws = &world.sketch(sketch).unwrap().sketch;
    let sw = &ws.wires[ws.wire_index(&spline).unwrap()];
    near(sw.fit[0], Vec3::new(14.0, 32.0, 55.0));
    near(sw.pts[0], sw.fit[0]);
    near(*sw.pts.last().unwrap(), Vec3::new(10.0, 35.0, 58.0));
    let before = s.doc.to_json();
    let moved = run(&mut s, "sketch.move3d", json!({"wire":a,"index":1,"by":[1,0,0]}));
    assert_eq!(moved["moved"], 2);
    let sk = s.doc.sketch(sketch).unwrap();
    near(sk.wires[sk.wire_index(&a).unwrap()].fit[1], Vec3::new(2.0, 2.0, 15.0));
    near(sk.wires[sk.wire_index(&b).unwrap()].fit[0], Vec3::new(2.0, 2.0, 15.0));
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.to_json(), before);
    run(&mut s, "edit.redo", json!({}));
    run(&mut s, "sketch.move3d", json!({"wire":point,"to":[18,33,46]}));
    near(s.doc.sketch(sketch).unwrap().wires.iter().find(|w| w.id == point).unwrap().fit[0], Vec3::new(3.0, 2.0, 6.0));
    let restored = solidcraft_doc::format::read(&s.doc.to_json()).unwrap();
    assert_eq!(restored.sketch(sketch).unwrap().wires, s.doc.sketch(sketch).unwrap().wires);
    // Same definition at the first placement has the local geometry; selected second is exact.
    run(&mut s, "component.activate", json!({"component":component,"occurrence":first,"path":[first]}));
    near(s.world_state().sketch(sketch).unwrap().sketch.wires.iter().find(|w| w.id == point).unwrap().fit[0], Vec3::new(3.0, 2.0, 6.0));
    run(&mut s, "component.activate", json!({"component":component,"occurrence":second,"path":[second]}));
    near(s.world_state().sketch(sketch).unwrap().sketch.wires.iter().find(|w| w.id == point).unwrap().fit[0], Vec3::new(18.0, 33.0, 46.0));
}

#[test]
fn wire_frame_pending_moves_other_owner_and_legacy_world_do_not_join() {
    let (mut s, _, _, second) = second_frame();
    let sketch = s.active_sketch.unwrap();
    let legacy = s.doc_mut().sketch_mut(sketch).unwrap().add_drawn_wire(&[Vec3::new(2.0, 3.0, 5.0)]).unwrap();
    let local = run(&mut s, "sketch.point3d", json!({"point":[17,32,45]}))["point"].as_str().unwrap().to_string();
    assert_eq!(run(&mut s, "sketch.move3d", json!({"wire":local,"by":[1,0,0]}))["moved"], 1);
    near(s.doc.sketch(sketch).unwrap().wires.iter().find(|w| w.id == legacy).unwrap().fit[0], Vec3::new(2.0, 3.0, 5.0));
    run(&mut s, "occurrence.move", json!({"occurrence":second,"translate":[5,0,0],"capture":false}));
    let point = run(&mut s, "sketch.point3d", json!({"point":[22,32,45]}))["point"].as_str().unwrap().to_string();
    near(s.doc.sketch(sketch).unwrap().wires.iter().find(|w| w.id == point).unwrap().fit[0], Vec3::new(2.0, 3.0, 5.0));
    near(s.world_state().sketch(sketch).unwrap().sketch.wires.iter().find(|w| w.id == legacy).unwrap().fit[0], Vec3::new(2.0, 3.0, 5.0));
    near(s.world_state().sketch(sketch).unwrap().sketch.wires.iter().find(|w| w.id == point).unwrap().fit[0], Vec3::new(22.0, 32.0, 45.0));
    run(&mut s, "component.capture_position", json!({}));
    near(s.world_state().sketch(sketch).unwrap().sketch.wires.iter().find(|w| w.id == point).unwrap().fit[0], Vec3::new(22.0, 32.0, 45.0));
    // Explicit target in root must not inherit the unrelated active component frame.
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "component.activate", json!({"component":0}));
    let root = run(&mut s, "sketch.create", json!({"plane":"XY","name":"Root path"}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let unrelated = run(&mut s, "component.create", json!({"name":"Unrelated"}))["occurrence"].as_u64().unwrap();
    run(&mut s, "occurrence.move", json!({"occurrence":unrelated,"translate":[100,200,300]}));
    run(&mut s, "sketch.point3d", json!({"sketch":root,"point":[9,8,7]}));
    near(s.doc.sketch(root).unwrap().wires[0].fit[0], Vec3::new(9.0, 8.0, 7.0));
}

#[test]
fn wire_frame_hostile_inputs_rollback_full_session() {
    let (mut s, _, _, second) = second_frame();
    let id = run(&mut s, "sketch.point3d", json!({"point":[17,32,45]}))["point"].as_str().unwrap().to_string();
    run(&mut s, "occurrence.move", json!({"occurrence":second,"translate":[1,0,0],"capture":false}));
    run(&mut s, "select.set", json!({"items":[{"type":"occurrence","path":[second]}]}));
    let doc = s.doc.to_json();
    let pending = s.pending_moves.clone();
    let selection = s.selection.clone();
    let history = (s.undo.len(), s.redo.len());
    for (cmd, p) in [
        ("sketch.move3d", json!({"wire":id,"index":u64::MAX,"to":[0,0,0]})),
        ("sketch.move3d", json!({"wire":id,"index":-1,"to":[0,0,0]})),
        ("sketch.move3d", json!({"wire":id,"index":"0","to":[0,0,0]})),
        ("sketch.move3d", json!({"wire":id,"to":[null,0,0],"by":[1,0,0]})),
        ("sketch.point3d", json!({"point":[1e100,0,0]})),
        ("sketch.line3d", json!({"points":[[17,32,45],[17,32,45]]})),
        ("sketch.spline3d", json!({"points":vec![[0,0,0];501]})),
    ] {
        assert!(s.execute(cmd, &p).is_err(), "{cmd}: {p}");
        assert_eq!(s.doc.to_json(), doc);
        assert_eq!(s.pending_moves, pending);
        assert_eq!(s.selection, selection);
        assert_eq!((s.undo.len(), s.redo.len()), history);
    }
}

#[test]
fn wire_frame_include_other_component_refresh_break_lost_and_reopen() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10}));
    let component = run(&mut s, "component.create", json!({"name":"Receiver","grounded":false}));
    let occurrence = component["occurrence"].as_u64().unwrap();
    run(&mut s, "occurrence.move", json!({"occurrence":occurrence,"translate":[20,0,0]}));
    run(&mut s, "sketch.create", json!({"plane":"XY","name":"Include target"}));
    let sketch = s.active_sketch.unwrap();
    run(&mut s, "sketch.include_3d", json!({"refs":[{"edge":[10,10,5],"body":"Body1"}]}));
    let initial = s.doc.sketch(sketch).unwrap().wires[0].clone();
    assert_eq!(initial.space, WireSpace::Component);
    assert!(initial.pts.iter().all(|p| (p.x + 10.0).abs() < 1e-7 && (p.y - 10.0).abs() < 1e-7));
    let link = initial.link.clone().unwrap();
    // A real sketch edit exercises engine refresh, not only document evaluation.
    run(&mut s, "sketch.line", json!({"points":[[0,0],[1,0]]}));
    let sk = s.doc.sketch(sketch).unwrap();
    assert_eq!(sk.wires[0], initial);
    assert!(!sk.link(&link).unwrap().lost);
    let world = s.world_state();
    assert!(world.sketch(sketch).unwrap().sketch.wires[0].pts.iter().all(|p| (p.x - 10.0).abs() < 1e-7));
    let before = s.doc.to_json();
    run(&mut s, "sketch.break_link", json!({"link":link}));
    assert_eq!(s.doc.sketch(sketch).unwrap().wires[0].space, WireSpace::Component);
    assert!(s.doc.sketch(sketch).unwrap().wires[0].link.is_none());
    let restored = solidcraft_doc::format::read(&s.doc.to_json()).unwrap();
    assert_eq!(restored.sketch(sketch).unwrap().wires[0], s.doc.sketch(sketch).unwrap().wires[0]);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.to_json(), before);
    // Lost references retain the last coordinates and explicit marker, with no guessing.
    let sk = s.doc_mut().sketch_mut(sketch).unwrap();
    let entry = sk.links.iter_mut().find(|l| l.id == link).unwrap();
    entry.source = solidcraft_sketch::LinkSource::Edge { body: "Missing".into(), at: Vec3::ZERO };
    // The link is the edge it names; without the name it is the edge nearest its point.
    entry.edge_name = None;
    run(&mut s, "sketch.line", json!({"points":[[2,0],[3,0]]}));
    let sk = s.doc.sketch(sketch).unwrap();
    assert!(sk.link(&link).unwrap().lost);
    assert_eq!(sk.wires[0], initial);
    let restored = solidcraft_doc::format::read(&s.doc.to_json()).unwrap();
    assert_eq!(restored.sketch(sketch).unwrap().wires[0], initial);
}

#[test]
fn wire_frame_pipe_component_and_world_paths_have_analytic_volume_and_position() {
    for space in [WireSpace::Component, WireSpace::World] {
        let (mut s, _, first, second) = second_frame();
        if space == WireSpace::World {
            // A single placed owner lets an immutable world path stay physically fixed;
            // replicated solids still follow their occurrence placements by design.
            run(&mut s, "selection.delete", json!({"items":[{"type":"occurrence","path":[first]}]}));
        }
        let sketch = s.active_sketch.unwrap();
        let wire = if space == WireSpace::Component {
            run(&mut s, "sketch.line3d", json!({"points":[[17,32,45],[17,32,55]]}))["curves"][0].as_str().unwrap().to_string()
        } else {
            s.doc_mut().sketch_mut(sketch).unwrap().add_drawn_wire(&[Vec3::new(17.0, 32.0, 45.0), Vec3::new(17.0, 32.0, 55.0)]).unwrap()
        };
        run(&mut s, "sketch.finish", json!({}));
        run(&mut s, "solid.pipe", json!({"path_sketch":sketch,"path":[wire],"diameter":4}));
        let world = s.world_state();
        let body = world.bodies.last().unwrap();
        // Property measurement uses fine adaptive tessellation; the viewport mesh alone
        // undershoots a small circular profile by about 0.64% at its display chord tolerance.
        let measure = solidcraft_kernel::measure(&body.body).unwrap();
        assert!((measure.volume - 40.0 * std::f64::consts::PI).abs() < 0.05, "{}", measure.volume);
        let bounds = body.mesh().bounds();
        near((bounds.min + bounds.max) / 2.0, Vec3::new(17.0, 32.0, 50.0));
        run(&mut s, "occurrence.move", json!({"occurrence":second,"translate":[5,0,0]}));
        let world = s.world_state();
        let body = world.bodies.last().unwrap();
        let bounds = body.mesh().bounds();
        near((bounds.min + bounds.max) / 2.0, Vec3::new(if space == WireSpace::Component { 22.0 } else { 17.0 }, 32.0, 50.0));
    }
}

#[test]
fn wire_frame_nonactive_repeated_owner_refuses_ambiguous_world_input() {
    let (mut s, _, _, _) = second_frame();
    let target = s.active_sketch.unwrap();
    run(&mut s, "component.activate", json!({"component":0}));
    let before = s.doc.to_json();
    let error = s.execute("sketch.point3d", &json!({"sketch":target,"point":[0,0,5]})).unwrap_err();
    assert!(error.to_string().contains("activate the desired sketch owner occurrence"));
    assert_eq!(s.doc.to_json(), before);
}

#[test]
fn wire_frame_wide_owner_graph_has_bounded_indexed_traversal() {
    let mut s = Session::default();
    run(&mut s, "component.create", json!({"name":"Target"}));
    let target = run(&mut s, "sketch.create", json!({"plane":"XY"}))["sketch"].as_u64().unwrap();
    run(&mut s, "component.activate", json!({"component":0}));
    // A legal wide assembly used to force one whole occurrence scan for each leaf.
    let doc = s.doc_mut();
    for i in 0..9998u64 {
        doc.components.push(solidcraft_doc::Component { id: 10_000 + i, name: format!("Sibling{i}"), parent: 0, ..Default::default() });
        doc.occurrences.push(solidcraft_doc::Occurrence {
            id: 100_000 + i,
            component: 10_000 + i,
            parent: 0,
            name: format!("Sibling{i}"),
            transform: solidcraft_doc::IDENTITY,
            grounded: false,
        });
    }
    assert_eq!(super::inverse_owner_frame(&s, &json!({"sketch":target}), "sketch.point3d").unwrap(), solidcraft_doc::IDENTITY);
}

#[test]
fn wire_frame_cross_owner_sweep_follows_world_path_edit_undo_and_reopen() {
    let mut s = Session::default();
    let path_sketch = run(&mut s, "sketch.create", json!({"plane":"XY","name":"Root path"}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let wire = run(&mut s, "sketch.line3d", json!({"points":[[17,32,45],[17,32,55]]}))["curves"][0].as_str().unwrap().to_string();
    run(&mut s, "sketch.finish", json!({}));
    let owner = run(&mut s, "component.create", json!({"name":"Sweep owner","grounded":false}))["occurrence"].as_u64().unwrap();
    run(&mut s, "occurrence.move", json!({"occurrence":owner,"axis":[0,0,1],"angle":"90 deg","translate":[20,30,40]}));
    let profile = run(&mut s, "sketch.create", json!({"plane":"XY","offset":5,"name":"Profile"}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.circle.center", json!({"center":[2,3],"radius":2}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "solid.sweep", json!({"sketch":profile,"path_sketch":path_sketch,"path":[wire],"operation":"new"}));
    let check = |s: &Session, length: f64| {
        let world = s.world_state();
        let body = world.bodies.last().unwrap();
        let measure = solidcraft_kernel::measure(&body.body).unwrap();
        assert!((measure.volume - 4.0 * std::f64::consts::PI * length).abs() < 0.05, "{}", measure.volume);
        near(measure.centroid, Vec3::new(17.0, 32.0, 45.0 + length / 2.0));
    };
    check(&s, 10.0);
    let saved = s.doc.to_json();
    run(&mut s, "sketch.edit", json!({"sketch":path_sketch}));
    run(&mut s, "sketch.move3d", json!({"sketch":path_sketch,"wire":wire,"index":1,"by":[0,0,10]}));
    check(&s, 20.0);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.to_json(), saved);
    check(&s, 10.0);
    let mut reopened = Session { doc: solidcraft_doc::format::read(&saved).unwrap().into(), ..Default::default() };
    reopened.refresh();
    check(&reopened, 10.0);
}

#[test]
fn wire_frame_three_owner_sweep_profile_move_invalidates_warm_cache() {
    let mut s = Session::default();
    let profile_owner = run(&mut s, "component.create", json!({"name":"Profile owner","parent":0,"grounded":false}));
    let profile_occurrence = profile_owner["occurrence"].as_u64().unwrap();
    run(&mut s, "occurrence.move", json!({"occurrence":profile_occurrence,"translate":[4,7,5]}));
    let profile = run(&mut s, "sketch.create", json!({"plane":"XY","name":"Foreign profile"}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.circle.center", json!({"center":[0,0],"radius":2}));
    run(&mut s, "sketch.finish", json!({}));

    let path_owner = run(&mut s, "component.create", json!({"name":"Path owner","parent":0}));
    let path_occurrence = path_owner["occurrence"].as_u64().unwrap();
    run(&mut s, "occurrence.move", json!({"occurrence":path_occurrence,"translate":[-12,9,-3]}));
    let path_sketch = run(&mut s, "sketch.create", json!({"plane":"XY","name":"Foreign path"}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let wire = run(&mut s, "sketch.line3d", json!({"points":[[4,7,5],[4,7,15]]}))["curves"][0].as_str().unwrap().to_string();
    run(&mut s, "sketch.finish", json!({}));

    let consumer = run(&mut s, "component.create", json!({"name":"Sweep consumer","parent":0}));
    let consumer_occurrence = consumer["occurrence"].as_u64().unwrap();
    run(&mut s, "occurrence.move", json!({"occurrence":consumer_occurrence,"axis":[0,0,1],"angle":"90 deg","translate":[100,-30,20]}));
    run(&mut s, "solid.sweep", json!({"sketch":profile,"path_sketch":path_sketch,"path":[wire],"operation":"new"}));
    let measure = |session: &Session, x: f64| {
        let state = session.world_state();
        let body = state.bodies.last().unwrap();
        assert_eq!(body.component, consumer["component"].as_u64().unwrap());
        let result = solidcraft_kernel::measure(&body.body).unwrap();
        assert!((result.volume - 40.0 * std::f64::consts::PI).abs() < 0.05, "{}", result.volume);
        near(result.centroid, Vec3::new(x, 7.0, 10.0));
        result
    };
    measure(&s, 4.0);
    // A repeated refresh warms the exact same evaluator cache before changing only
    // the foreign profile placement, whose frame must be part of Sweep's digest.
    s.refresh();
    measure(&s, 4.0);
    let saved = s.doc.to_json();
    let fixed_path = s.doc.component_transform(path_owner["component"].as_u64().unwrap());
    let fixed_consumer = s.doc.component_transform(consumer["component"].as_u64().unwrap());
    run(&mut s, "occurrence.move", json!({"occurrence":profile_occurrence,"translate":[3,0,0]}));
    assert_eq!(s.doc.component_transform(path_owner["component"].as_u64().unwrap()), fixed_path);
    assert_eq!(s.doc.component_transform(consumer["component"].as_u64().unwrap()), fixed_consumer);
    let warm = measure(&s, 7.0);
    let mut cold = Session { doc: solidcraft_doc::format::read(&s.doc.to_json()).unwrap().into(), ..Default::default() };
    cold.refresh();
    let reopened = measure(&cold, 7.0);
    near(warm.centroid, reopened.centroid);
    assert!((warm.volume - reopened.volume).abs() < 1e-8);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.to_json(), saved);
    measure(&s, 4.0);
}

#[test]
fn arc3d_is_exact_inspectable_movable_and_reopens() {
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane":"XY"}));
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let sketch = s.active_sketch.unwrap();
    let d = 5.0 * std::f64::consts::SQRT_2;
    let r = run(&mut s, "sketch.arc3d", json!({"start":[10,0,0],"through":[d,d,0],"end":[0,10,0]}));
    let id = r["curves"][0].as_str().unwrap().to_string();
    let sk = s.doc.sketch(sketch).unwrap();
    let w = &sk.wires[sk.wire_index(&id).unwrap()];
    let arc = w.arc.expect("an exact arc is stored");
    assert!((arc.radius - 10.0).abs() < 1e-9, "radius {}", arc.radius);
    assert!(arc.center.dist(Vec3::ZERO) < 1e-9, "centre {:?}", arc.center);
    assert_eq!(w.fit.len(), 3);
    assert_eq!(w.pts.first(), w.fit.first());
    assert_eq!(w.pts.last(), w.fit.get(2));
    assert!(w.pts.iter().all(|p| (arc.center.dist(*p) - arc.radius).abs() < 1e-6));
    let _ = sk;
    let insp = run(&mut s, "sketch.inspect", json!({}));
    let entry = insp["wires"].as_array().unwrap().iter().find(|e| e["id"] == json!(id)).unwrap();
    assert!((entry["arc"]["radius"].as_f64().unwrap() - 10.0).abs() < 1e-9);
    assert!((entry["arc"]["sweep_deg"].as_f64().unwrap() - 90.0).abs() < 1e-6);
    // Pulling the through point in rebuilds the arc exactly.
    run(&mut s, "sketch.move3d", json!({"wire":id,"index":1,"to":[0,5,0]}));
    let sk = s.doc.sketch(sketch).unwrap();
    let arc = sk.wires[sk.wire_index(&id).unwrap()].arc.unwrap();
    assert!((arc.radius - 62.5_f64.sqrt()).abs() < 1e-9, "radius {}", arc.radius);
    let saved = s.doc.to_json();
    let restored = solidcraft_doc::format::read(&saved).unwrap();
    assert_eq!(restored.sketch(sketch).unwrap().wires, s.doc.sketch(sketch).unwrap().wires);
    let idx = s.doc.sketch(sketch).unwrap().wire_index(&id).unwrap();
    run(&mut s, "edit.undo", json!({}));
    assert!((s.doc.sketch(sketch).unwrap().wires[idx].arc.unwrap().radius - 10.0).abs() < 1e-9);
}

#[test]
fn arc3d_follows_the_sketch_owner_placement() {
    let (mut s, _, _, second) = second_frame();
    let sketch = s.active_sketch.unwrap();
    let d = 5.0 * std::f64::consts::SQRT_2;
    // World points of a quarter circle whose local centre is (0,0,5), radius 10.
    let id = run(&mut s, "sketch.arc3d", json!({"start":[20,40,45],"through":[20.0 - d,30.0 + d,45],"end":[10,30,45]}))["curves"][0]
        .as_str()
        .unwrap()
        .to_string();
    let sk = s.doc.sketch(sketch).unwrap();
    let arc = sk.wires[sk.wire_index(&id).unwrap()].arc.unwrap();
    near(arc.center, Vec3::new(0.0, 0.0, 5.0));
    assert!((arc.radius - 10.0).abs() < 1e-7, "radius {}", arc.radius);
    let _ = sk;
    let world = s.world_state();
    let ws = &world.sketch(sketch).unwrap().sketch;
    let arc = ws.wires[ws.wire_index(&id).unwrap()].arc.unwrap();
    near(arc.center, Vec3::new(20.0, 30.0, 45.0));
    near(ws.wires[ws.wire_index(&id).unwrap()].fit[0], Vec3::new(20.0, 40.0, 45.0));
    drop(world);
    run(&mut s, "occurrence.move", json!({"occurrence":second,"translate":[5,0,0]}));
    let world = s.world_state();
    let ws = &world.sketch(sketch).unwrap().sketch;
    let arc = ws.wires[ws.wire_index(&id).unwrap()].arc.unwrap();
    near(arc.center, Vec3::new(25.0, 30.0, 45.0));
    assert!((arc.radius - 10.0).abs() < 1e-7, "radius {}", arc.radius);
}

#[test]
fn arc3d_hostile_inputs_rollback_full_session() {
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane":"XY"}));
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let doc = s.doc.to_json();
    let history = (s.undo.len(), s.redo.len());
    for (cmd, p) in [
        ("sketch.arc3d", json!({"start":[0,0,0],"through":[1,0,0],"end":[2,0,0]})), // collinear
        ("sketch.arc3d", json!({"start":[0,0,0],"through":[0,0,0],"end":[0,10,0]})), // coincident
        ("sketch.arc3d", json!({"start":[0,0,0],"through":[f64::NAN,5,0],"end":[10,0,0]})),
        ("sketch.arc3d", json!({"start":[0,0,0],"through":[1e100,5,0],"end":[10,0,0]})),
        ("sketch.arc3d", json!({"start":[0,0,0],"through":[5,5,0]})), // missing end
        ("sketch.arc3d", json!({"start":[0,0,0],"through":["x",5,0],"end":[10,0,0]})),
        ("sketch.arc3d", json!({"through":[5,5,0],"end":[10,0,0]})), // missing start
    ] {
        assert!(s.execute(cmd, &p).is_err(), "{cmd}: {p}");
        assert_eq!(s.doc.to_json(), doc);
        assert_eq!((s.undo.len(), s.redo.len()), history);
    }
}

#[test]
fn arc3d_pipes_along_the_arc() {
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane":"XY"}));
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let sketch = s.active_sketch.unwrap();
    let d = 10.0 * std::f64::consts::SQRT_2;
    let wire = run(&mut s, "sketch.arc3d", json!({"start":[20,0,0],"through":[d,d,0],"end":[0,20,0]}))["curves"][0].as_str().unwrap().to_string();
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "solid.pipe", json!({"path_sketch":sketch,"path":[wire],"diameter":4}));
    let world = s.world_state();
    let body = world.bodies.last().unwrap();
    let measure = solidcraft_kernel::measure(&body.body).unwrap();
    let quarter = 20.0 * std::f64::consts::FRAC_PI_2;
    assert!((measure.volume - 4.0 * std::f64::consts::PI * quarter).abs() < 0.5, "{}", measure.volume);
}


#[test]
fn spatial_guide_fit_points_define_reusable_construction_plane() {
    let mut s = Session::default();
    let guide = run(&mut s, "sketch.create", json!({"plane":"XY","name":"Spatial guides"}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let wire = run(&mut s, "sketch.spline3d", json!({"points":[[0,0,0],[10,0,5],[20,0,10],[20,10,20]]}))["curves"][0]
        .as_str()
        .unwrap()
        .to_owned();
    run(&mut s, "sketch.point3d", json!({"point":[0,10,5]}));
    let point_id = s.doc.sketch(guide).unwrap().wires.last().unwrap().id.clone();
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "construct.plane.three_points", json!({
        "a":{"sketch":guide,"point":format!("{wire}.start")},
        "b":{"sketch":guide,"point":format!("{wire}.1")},
        "c":{"sketch":guide,"point":point_id},
        "name":"Guide workplane"
    }));
    let created = run(&mut s, "sketch.create", json!({"plane":"Guide workplane","name":"Guide profile"}));
    let profile = created["sketch"].as_u64().unwrap();
    let verify = |s: &Session, b: Vec3| {
        let st = s.world_state();
        let plane = st.sketch(profile).unwrap().plane;
        near(plane.origin, Vec3::ZERO);
        assert!(plane.height(b).abs() < 1e-7, "{plane:?} {b:?}");
        assert!(plane.height(Vec3::new(0.0, 10.0, 5.0)).abs() < 1e-7);
        assert!((plane.x.len() - 1.0).abs() < 1e-12);
        assert!((plane.y.len() - 1.0).abs() < 1e-12);
        assert!(plane.normal().dot(plane.x).abs() < 1e-12);
    };
    verify(&s, Vec3::new(10.0, 0.0, 5.0));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "sketch.edit", json!({"sketch":guide}));
    run(&mut s, "sketch.move3d", json!({"wire":wire,"index":1,"by":[0,2,0]}));
    verify(&s, Vec3::new(10.0, 2.0, 5.0));
    let restored = Session::new(solidcraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    verify(&restored, Vec3::new(10.0, 2.0, 5.0));
}

#[test]
fn spatial_guide_invalid_fit_point_reference_is_atomic() {
    let mut s = Session::default();
    let guide = run(&mut s, "sketch.create", json!({"plane":"XY","name":"Guide"}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let wire = run(&mut s, "sketch.line3d", json!({"points":[[0,0,1],[3,4,5]]}))["curves"][0].as_str().unwrap().to_owned();
    run(&mut s, "sketch.finish", json!({}));
    let original = s.doc.to_json();
    let history = (s.undo.len(), s.redo.len());
    for invalid in [format!("{wire}.7"), format!("{wire}.through"), wire.clone()] {
        let result = s.execute("construct.plane.three_points", &json!({
            "a":{"sketch":guide,"point":invalid},
            "b":{"point":[0,0,0]},
            "c":{"point":[0,10,0]},
            "name":"Invalid plane"
        }));
        assert!(result.is_err(), "{invalid}: {result:?}");
        assert_eq!(s.doc.to_json(), original);
        assert_eq!((s.undo.len(), s.redo.len()), history);
    }
}


#[test]
fn spatial_polyline_and_local_xyz_uses_right_handed_workplane() {
    let mut s = Session::default();
    let sk = run(&mut s, "sketch.create", json!({
        "plane":{"origin":[20,30,40],"x_dir":[0,1,0],"y_dir":[0,0,1]}, "name":"Oblique workplane"
    }))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let paths = run(&mut s, "sketch.polyline3d", json!({
        "frame":"sketch", "closed":true, "points":[[0,0,0],[10,0,0],[10,5,0]]
    }))["curves"].as_array().unwrap().clone();
    assert_eq!(paths.len(), 3);
    let sketched = s.doc.sketch(sk).unwrap();
    let edges: Vec<_> = paths.iter().map(|id| sketched.wires.iter().find(|w| w.id == id.as_str().unwrap()).unwrap()).collect();
    for i in 0..3 {
        near(*edges[i].fit.last().unwrap(), edges[(i + 1) % 3].fit[0]);
    }
    near(edges[0].fit[0], Vec3::new(20.0, 30.0, 40.0));
    near(edges[0].fit[1], Vec3::new(20.0, 40.0, 40.0));
    near(edges[1].fit[1], Vec3::new(20.0, 40.0, 45.0));
    let id = run(&mut s, "sketch.point3d", json!({"point":[2,3,4],"frame":"sketch"}))["point"].as_str().unwrap().to_owned();
    near(s.doc.sketch(sk).unwrap().wires.iter().find(|w| w.id == id).unwrap().fit[0], Vec3::new(24.0,32.0,43.0));
    run(&mut s, "sketch.move3d", json!({"wire":id,"by":[1,2,3],"frame":"sketch"}));
    near(s.doc.sketch(sk).unwrap().wires.iter().find(|w| w.id == id).unwrap().fit[0], Vec3::new(27.0,33.0,45.0));
    let state = s.doc.to_json();
    let restored = solidcraft_doc::format::read(&state).unwrap();
    assert_eq!(restored.sketch(sk).unwrap().wires, s.doc.sketch(sk).unwrap().wires);
}

#[test]
fn spatial_local_coordinates_follow_the_selected_occurrence() {
    let (mut s, _, _, _) = second_frame();
    let sketch = s.active_sketch.unwrap();
    let point = run(&mut s, "sketch.point3d", json!({"point":[2,3,5],"frame":"sketch"}))["point"].as_str().unwrap().to_owned();
    near(s.doc.sketch(sketch).unwrap().wires.iter().find(|w| w.id == point).unwrap().fit[0], Vec3::new(2.0,3.0,5.0));
    near(s.world_state().sketch(sketch).unwrap().sketch.wires.iter().find(|w| w.id == point).unwrap().fit[0], Vec3::new(17.0,32.0,45.0));
    run(&mut s, "sketch.move3d", json!({"wire":point,"by":[1,2,3],"frame":"sketch"}));
    near(s.doc.sketch(sketch).unwrap().wires.iter().find(|w| w.id == point).unwrap().fit[0], Vec3::new(3.0,5.0,8.0));
    near(s.world_state().sketch(sketch).unwrap().sketch.wires.iter().find(|w| w.id == point).unwrap().fit[0], Vec3::new(15.0,33.0,48.0));
}

#[test]
fn spatial_fit_insert_remove_preserves_curve_identity_undo_and_saved_file() {
    let mut s = Session::default();
    let sketch = run(&mut s, "sketch.create", json!({"plane":"XY","name":"Chair backrest"}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let wire = run(&mut s, "sketch.spline3d", json!({"points":[[0,0,0],[4,2,15],[8,6,28],[12,0,38]]}))["curves"][0]
        .as_str().unwrap().to_owned();
    let before = s.doc.to_json();
    let inserted = run(&mut s, "sketch.insert_fit3d", json!({"wire":wire,"index":2,"at":[6,5,21]}));
    assert_eq!(inserted["fit_points"], 5);
    let w = s.doc.sketch(sketch).unwrap().wires.iter().find(|w| w.id == wire).unwrap();
    assert_eq!(w.fit.len(), 5);
    near(w.fit[2], Vec3::new(6.0,5.0,21.0));
    near(*w.pts.first().unwrap(), Vec3::ZERO);
    near(*w.pts.last().unwrap(), Vec3::new(12.0,0.0,38.0));
    assert!(w.pts.iter().all(|p| p.is_finite()));
    let after = s.doc.to_json();
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.to_json(), before);
    run(&mut s, "edit.redo", json!({}));
    assert_eq!(s.doc.to_json(), after);
    let reopened = solidcraft_doc::format::read(&after).unwrap();
    assert_eq!(reopened.sketch(sketch).unwrap().wires, s.doc.sketch(sketch).unwrap().wires);
    let removed = run(&mut s, "sketch.remove_fit3d", json!({"wire":wire,"index":2}));
    assert_eq!(removed["fit_points"], 4);
    assert_eq!(s.doc.to_json(), before);
}

#[test]
fn spatial_closed_spline_and_bad_fit_edits_are_transactional() {
    let mut s = Session::default();
    let sk = run(&mut s, "sketch.create", json!({"plane":"XY","name":"Lamp spine"}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let id = run(&mut s, "sketch.spline3d", json!({"points":[[0,0,0],[8,0,12],[8,8,24],[0,8,12]],"closed":true}))["curves"][0]
        .as_str().unwrap().to_owned();
    let w = s.doc.sketch(sk).unwrap().wires.iter().find(|w| w.id == id).unwrap();
    assert_eq!(w.fit.len(), 5);
    near(*w.pts.first().unwrap(), *w.pts.last().unwrap());
    assert!(w.pts.iter().all(|p| p.is_finite()));
    let before = s.doc.to_json();
    let history = (s.undo.len(), s.redo.len());
    for (cmd, params) in [
        ("sketch.insert_fit3d", json!({"wire":id,"index":0,"at":[1,2,3]})),
        ("sketch.insert_fit3d", json!({"wire":id,"index":1,"at":[0,0,0]})),
        ("sketch.insert_fit3d", json!({"wire":id,"index":2,"at":[1e100,0,0]})),
        ("sketch.remove_fit3d", json!({"wire":id,"index":0})),
        ("sketch.remove_fit3d", json!({"wire":id,"index":4})),
        ("sketch.polyline3d", json!({"points":[[0,0,0],[1,0,0]],"closed":true})),
        ("sketch.line3d", json!({"points":[[0,0,0],[1,0,0]],"closed":"yes"})),
        ("sketch.point3d", json!({"point":[1,2,3],"frame":"unsupported"})),
    ] {
        assert!(s.execute(cmd, &params).is_err(), "{cmd} {params}");
        assert_eq!(s.doc.to_json(), before, "{cmd}");
        assert_eq!((s.undo.len(), s.redo.len()), history);
    }
}


#[test]
fn spatial_saved_normal_plane_and_through_point_follow_referenced_3d_point() {
    let mut s = Session::default();
    run(&mut s, "construct.plane.normal", json!({"origin":[3,4,5],"normal":[2,3,6],"name":"Tilted"}));
    let tilt = run(&mut s, "sketch.create", json!({"plane":"Tilted","name":"Lamp section"}))["sketch"].as_u64().unwrap();
    {
        let st = s.world_state();
        let plane = st.sketch(tilt).unwrap().plane;
        near(plane.origin, Vec3::new(3.0,4.0,5.0));
        near(plane.normal(), Vec3::new(2.0 / 7.0,3.0 / 7.0,6.0 / 7.0));
    }
    run(&mut s, "sketch.finish", json!({}));
    let guide = run(&mut s, "sketch.create", json!({"plane":"XY","name":"Guide"}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.options", json!({"sketch_3d":true}));
    let point = run(&mut s, "sketch.point3d", json!({"point":[10,20,30]}))["point"].as_str().unwrap().to_owned();
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "construct.plane.through_point", json!({
        "plane":"XY","point":{"sketch":guide,"point":point},"name":"Height through spatial point"
    }));
    let profile = run(&mut s, "sketch.create", json!({"plane":"Height through spatial point","name":"Dynamic section"}))["sketch"].as_u64().unwrap();
    let check = |s: &Session, z: f64| {
        let st = s.world_state();
        let pl = st.sketch(profile).unwrap().plane;
        near(pl.origin, Vec3::new(0.0,0.0,z));
        near(pl.normal(), Vec3::Z);
    };
    check(&s,30.0);
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "sketch.edit", json!({"sketch":guide}));
    run(&mut s, "sketch.move3d", json!({"wire":point,"by":[0,0,5]}));
    check(&s,35.0);
    let reopen = Session::new(solidcraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    check(&reopen,35.0);
    let before = s.doc.to_json();
    let history = (s.undo.len(), s.redo.len());
    for bad_plane in [
        json!({"origin":[0,0,0],"normal":[0,0,0],"name":"Bad normal"}),
        json!({"origin":[0,0,0],"normal":[1e100,0,0],"name":"Bad normal"}),
        json!({"origin":[0,0,0],"normal":[0,0,0.0],"name":"Bad normal"}),
    ] {
        assert!(s.execute("construct.plane.normal", &bad_plane).is_err());
        assert_eq!(s.doc.to_json(), before);
        assert_eq!((s.undo.len(), s.redo.len()), history);
    }
}
