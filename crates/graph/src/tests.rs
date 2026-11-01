use super::*;

fn text_of(ev: &Evaluation, id: NodeId) -> String {
    match ev.outputs.get(&(id, 0)).and_then(|t| t.branches.values().next()).and_then(|l| l.first()) {
        Some(Value::Text(t)) => t.clone(),
        other => panic!("expected text, got {other:?}"),
    }
}

fn numbers(ev: &Evaluation, id: NodeId) -> Vec<f64> {
    ev.outputs.get(&(id, 0)).map(|t| t.branches.values().flatten().filter_map(|v| v.number().ok()).collect()).unwrap_or_default()
}

#[test]
fn every_component_has_matching_defaults_and_unique_name() {
    let mut names = std::collections::BTreeSet::new();
    for c in COMPONENTS {
        assert_eq!((c.defaults)().len(), c.inputs.len(), "{}", c.name);
        assert!(names.insert(c.name), "duplicate {}", c.name);
    }
}

#[test]
fn malformed_public_identifiers_cannot_overwrite_nodes_or_create_unloadable_saves() {
    let mut graph = Graph::new();
    let first = graph.add_node("Number", (0.0, 0.0)).unwrap();
    let node = graph.nodes[&first].clone();
    graph.nodes.insert(NodeId(1), node.clone());
    let before = graph.clone();
    assert!(graph.add_node("Circle", (1.0, 1.0)).is_err());
    assert_eq!(graph, before, "a counter collision must not replace user content");
    assert!(graph.to_json().is_err());
    graph.nodes.remove(&NodeId(1));
    graph.nodes.insert(NodeId(u32::MAX), node);
    assert!(graph.to_json().is_err(), "saving must reject IDs that cannot be loaded again");
}

#[test]
fn numbers_flow_along_wires() {
    let mut g = Graph::new();
    let a = g.add_node("Number", (0.0, 0.0)).unwrap();
    let add = g.add_node("Add", (1.0, 0.0)).unwrap();
    g.set_constant(a, 0, Value::Number(4.0)).unwrap();
    g.connect(a, 0, add, 0).unwrap();
    g.set_constant(add, 1, Value::Number(6.0)).unwrap();
    let ev = g.evaluate().unwrap();
    assert_eq!(numbers(&ev, add), vec![10.0]);
}

#[test]
fn lists_match_item_by_item_and_the_short_one_repeats() {
    let mut g = Graph::new();
    let s = g.add_node("Series", (0.0, 0.0)).unwrap();
    g.set_constant(s, 2, Value::Number(3.0)).unwrap(); // 0, 1, 2
    let add = g.add_node("Add", (1.0, 0.0)).unwrap();
    g.connect(s, 0, add, 0).unwrap();
    g.set_constant(add, 1, Value::Number(10.0)).unwrap();
    assert_eq!(numbers(&g.evaluate().unwrap(), add), vec![10.0, 11.0, 12.0]);
}

#[test]
fn graft_and_flatten_change_the_tree() {
    let t = Tree { branches: BTreeMap::from([(vec![0], vec![Value::Number(1.0), Value::Number(2.0)]), (vec![1], vec![Value::Number(3.0)])]) };
    assert_eq!(t.flatten().branches.len(), 1);
    assert_eq!(t.flatten().item_count(), 3);
    assert_eq!(t.graft().branches.len(), 3);
}

#[test]
fn a_loop_of_wires_is_an_error_not_a_hang() {
    let mut g = Graph::new();
    let a = g.add_node("Add", (0.0, 0.0)).unwrap();
    let b = g.add_node("Add", (1.0, 0.0)).unwrap();
    g.connect(a, 0, b, 0).unwrap();
    g.connect(b, 0, a, 0).unwrap();
    assert_eq!(g.evaluate(), Err(GraphError::Cycle));
}

#[test]
fn a_failing_node_marks_itself_and_what_follows_but_not_the_rest() {
    let mut g = Graph::new();
    let d = g.add_node("Divide", (0.0, 0.0)).unwrap();
    g.set_constant(d, 1, Value::Number(0.0)).unwrap();
    let after = g.add_node("Add", (1.0, 0.0)).unwrap();
    g.connect(d, 0, after, 0).unwrap();
    let fine = g.add_node("Number", (0.0, 1.0)).unwrap();
    let ev = g.evaluate().unwrap();
    assert!(ev.errors.contains_key(&d));
    assert!(ev.errors.contains_key(&after));
    assert!(!ev.errors.contains_key(&fine));
}

#[test]
fn hostile_numbers_are_errors() {
    let mut g = Graph::new();
    let s = g.add_node("Series", (0.0, 0.0)).unwrap();
    g.set_constant(s, 2, Value::Number(1e12)).unwrap();
    let c = g.add_node("Circle", (0.0, 0.0)).unwrap();
    g.set_constant(c, 1, Value::Number(f64::NAN)).unwrap();
    let ev = g.evaluate().unwrap();
    assert!(ev.errors.contains_key(&s));
    assert!(ev.errors.contains_key(&c));
}

#[test]
fn bad_wires_are_refused() {
    let mut g = Graph::new();
    let a = g.add_node("Number", (0.0, 0.0)).unwrap();
    assert_eq!(g.connect(a, 5, a, 0), Err(GraphError::NoSocket));
    assert_eq!(g.connect(NodeId(99), 0, a, 0), Err(GraphError::NoNode));
    assert!(matches!(g.add_node("Nope", (0.0, 0.0)), Err(GraphError::UnknownComponent(_))));
}

#[test]
fn removing_a_node_cuts_its_wires() {
    let mut g = Graph::new();
    let a = g.add_node("Number", (0.0, 0.0)).unwrap();
    let b = g.add_node("Add", (1.0, 0.0)).unwrap();
    g.connect(a, 0, b, 0).unwrap();
    g.remove_node(a);
    assert!(g.nodes.get(&b).is_some_and(|n| n.inputs.first().is_some_and(|s| s.wire.is_none())));
    assert!(g.evaluate().unwrap().errors.is_empty());
}

/// circle -> linear pattern of 3 -> extrude -> merge -> slice -> G-code
#[test]
fn a_pattern_of_cylinders_becomes_g_code() {
    let mut g = Graph::new();
    let circle = g.add_node("Circle", (0.0, 0.0)).unwrap();
    g.set_constant(circle, 0, Value::Number(5.0)).unwrap();
    let pat = g.add_node("Linear Pattern", (1.0, 0.0)).unwrap();
    g.connect(circle, 0, pat, 0).unwrap();
    g.set_constant(pat, 1, Value::Number(3.0)).unwrap();
    g.set_constant(pat, 2, Value::Number(20.0)).unwrap();
    let ext = g.add_node("Extrude", (2.0, 0.0)).unwrap();
    g.connect(pat, 0, ext, 0).unwrap();
    g.set_constant(ext, 1, Value::Number(4.0)).unwrap();
    let merge = g.add_node("Merge Solids", (3.0, 0.0)).unwrap();
    g.connect(ext, 0, merge, 0).unwrap();
    let sl = g.add_node("Slice", (4.0, 0.0)).unwrap();
    g.connect(merge, 0, sl, 0).unwrap();
    let gc = g.add_node("G-code", (5.0, 0.0)).unwrap();
    g.connect(sl, 0, gc, 0).unwrap();

    let ev = g.evaluate().unwrap();
    assert!(ev.errors.is_empty(), "{:?}", ev.errors);
    let layers = match ev.outputs.get(&(sl, 0)).and_then(|t| t.branches.values().next()).and_then(|l| l.first()) {
        Some(Value::Layers(l)) => l.clone(),
        other => panic!("expected layers, got {other:?}"),
    };
    assert_eq!(layers.layers.len(), 20);
    assert!(layers.layers.iter().all(|l| l.loops.len() == 3));
    let text = text_of(&ev, gc);
    assert_eq!(text.matches(";LAYER:").count(), 20);
    assert!(text.contains("G1 X"));
}

#[test]
fn a_circular_pattern_makes_that_many_shapes() {
    let mut g = Graph::new();
    let r = g.add_node("Rectangle", (0.0, 0.0)).unwrap();
    let mv = g.add_node("Move", (1.0, 0.0)).unwrap();
    g.connect(r, 0, mv, 0).unwrap();
    g.set_constant(mv, 1, Value::Number(30.0)).unwrap();
    let pat = g.add_node("Circular Pattern", (2.0, 0.0)).unwrap();
    g.connect(mv, 0, pat, 0).unwrap();
    g.set_constant(pat, 1, Value::Number(5.0)).unwrap();
    let ev = g.evaluate().unwrap();
    assert!(ev.errors.is_empty());
    assert_eq!(ev.outputs.get(&(pat, 0)).map(Tree::item_count), Some(5));
}

#[test]
fn a_graph_can_be_stopped_part_way() {
    let mut g = Graph::new();
    g.add_node("Number", (0.0, 0.0)).unwrap();
    assert_eq!(g.evaluate_until(&|| true), Err(GraphError::Cancelled));
    assert!(g.evaluate_until(&|| false).is_ok());
}

#[test]
fn graph_json_round_trips_values_wires_modifiers_and_positions() {
    let mut graph = Graph::new();
    let series = graph.add_node("Series", (120.0, 40.0)).unwrap();
    let add = graph.add_node("Add", (420.0, 80.0)).unwrap();
    graph.set_constant(series, 2, Value::Number(5.0)).unwrap();
    graph.set_constant(add, 1, Value::Number(3.5)).unwrap();
    graph.connect_checked(series, 0, add, 0).unwrap();
    graph.set_modifier(add, 0, Modifier::Graft).unwrap();
    let saved = graph.to_json().unwrap();
    let loaded = Graph::from_json(&saved).unwrap();
    assert_eq!(loaded, graph);
    assert_eq!(loaded.evaluate().unwrap(), graph.evaluate().unwrap());
    assert_eq!(loaded.to_json().unwrap(), saved);
}

#[test]
fn graph_documents_reject_unknown_versions_duplicate_ids_and_missing_wires() {
    let mut graph = Graph::new();
    let id = graph.add_node("Number", (0.0, 0.0)).unwrap();
    let saved = graph.to_json().unwrap();
    let original: serde_json::Value = serde_json::from_str(&saved).unwrap();
    let mut changed = original.clone();
    changed["version"] = 99.into();
    assert!(Graph::from_json(&changed.to_string()).is_err());
    changed = original.clone();
    let duplicate = changed["nodes"][0].clone();
    changed["nodes"].as_array_mut().unwrap().push(duplicate);
    assert!(Graph::from_json(&changed.to_string()).is_err());
    changed = original.clone();
    changed["nodes"][0]["inputs"][0]["wire"] = serde_json::json!({"from":55,"output":0});
    assert_eq!(Graph::from_json(&changed.to_string()), Err(GraphError::NoNode));
    changed["nodes"][0]["inputs"][0]["wire"] = serde_json::json!({"from":id.0,"output":0});
    assert_eq!(Graph::from_json(&changed.to_string()), Err(GraphError::Cycle));
    changed = original;
    changed["nodes"][0]["id"] = u32::MAX.into();
    assert!(Graph::from_json(&changed.to_string()).is_err());
}

#[test]
fn hostile_document_values_and_positions_are_refused() {
    let mut graph = Graph::new();
    assert!(graph.add_node("Number", (f32::NAN, 0.0)).is_err());
    assert!(graph.add_node("Number", (-500.0, -500.0)).is_err());
    let id = graph.add_node("Number", (0.0, 0.0)).unwrap();
    assert!(graph.move_node(id, (f32::INFINITY, 0.0)).is_err());
    graph.set_constant(id, 0, Value::Number(f64::NAN)).unwrap();
    assert!(graph.to_json().is_err());
    let oversized = " ".repeat(MAX_DOCUMENT_BYTES + 1);
    assert!(Graph::from_json(&oversized).is_err());
    graph.set_constant(id, 0, Value::Text("x".repeat(MAX_TEXT_BYTES + 1))).unwrap();
    assert!(graph.to_json().is_err());
}

#[test]
fn save_refuses_aggregate_constants_that_cannot_be_loaded() {
    let mut graph = Graph::new();
    for _ in 0..7 {
        let id = graph.add_node("Number", (0.0, 0.0)).unwrap();
        graph.set_constant(id, 0, Value::Text("x".repeat(MAX_TEXT_BYTES))).unwrap();
    }
    assert!(graph.to_json().is_err());
}

#[test]
fn graph_round_trip_preserves_id_sequence_after_deleting_the_last_node() {
    let mut graph = Graph::new();
    graph.add_node("Number", (0.0, 0.0)).unwrap();
    let deleted = graph.add_node("Number", (0.0, 0.0)).unwrap();
    graph.remove_node(deleted);
    let loaded = Graph::from_json(&graph.to_json().unwrap()).unwrap();
    assert_eq!(loaded, graph);
}

#[test]
fn rejected_cycle_connection_preserves_the_old_wire() {
    let mut graph = Graph::new();
    let number = graph.add_node("Number", (0.0, 0.0)).unwrap();
    let a = graph.add_node("Add", (1.0, 0.0)).unwrap();
    let b = graph.add_node("Add", (2.0, 0.0)).unwrap();
    graph.connect_checked(number, 0, a, 0).unwrap();
    graph.connect_checked(a, 0, b, 0).unwrap();
    let before = graph.clone();
    assert_eq!(graph.connect_checked(b, 0, a, 0), Err(GraphError::Cycle));
    assert_eq!(graph, before);
    assert!(graph.evaluate().unwrap().errors.is_empty());
}

#[test]
fn list_evaluation_observes_cancellation_inside_one_node() {
    let mut graph = Graph::new();
    let series = graph.add_node("Series", (0.0, 0.0)).unwrap();
    let add = graph.add_node("Add", (1.0, 0.0)).unwrap();
    graph.set_constant(series, 2, Value::Number(100.0)).unwrap();
    graph.connect_checked(series, 0, add, 0).unwrap();
    let calls = std::cell::Cell::new(0usize);
    let result = graph.evaluate_until(&|| {
        calls.set(calls.get() + 1);
        calls.get() >= 20
    });
    assert_eq!(result, Err(GraphError::Cancelled));
    assert!(calls.get() < 30);
}

#[test]
fn browser_preview_yields_and_refuses_print_work_without_blocking_other_nodes() {
    let mut graph = Graph::new();
    let number = graph.add_node("Number", (0.0, 0.0)).unwrap();
    let slice = graph.add_node("Slice", (1.0, 0.0)).unwrap();
    let mut job = PreviewEvaluation::new(graph).unwrap();
    assert_eq!(job.progress(), (0, 2));
    assert!(!job.step(&|| false).unwrap());
    assert_eq!(job.progress(), (1, 2));
    assert!(job.step(&|| false).unwrap());
    let result = job.finish().unwrap();
    assert_eq!(numbers(&result, number), vec![1.0]);
    assert!(result.errors.get(&slice).is_some_and(|error| error.contains("desktop")));
}

#[test]
fn browser_preview_rejects_large_expansion_before_allocating_it() {
    let mut graph = Graph::new();
    let circle = graph.add_node("Circle", (0.0, 0.0)).unwrap();
    let pattern = graph.add_node("Linear Pattern", (1.0, 0.0)).unwrap();
    graph.set_constant(circle, 1, Value::Number(512.0)).unwrap();
    graph.set_constant(pattern, 1, Value::Number(256.0)).unwrap();
    graph.connect_checked(circle, 0, pattern, 0).unwrap();
    let mut job = PreviewEvaluation::new(graph).unwrap();
    while !job.step(&|| false).unwrap() {}
    let result = job.finish().unwrap();
    assert!(result.errors.get(&pattern).is_some_and(|error| error.contains("budget")));
    assert!(!result.outputs.contains_key(&(pattern, 0)));
}

#[test]
fn malformed_public_graph_cannot_silently_fall_back_to_a_constant() {
    let mut graph = Graph::new();
    let node = graph.add_node("Add", (0.0, 0.0)).unwrap();
    graph.nodes.get_mut(&node).unwrap().inputs[0].wire = Some(Wire { from: NodeId(900), output: 0 });
    assert_eq!(graph.evaluate(), Err(GraphError::NoNode));
}

#[test]
fn evaluation_budget_is_shared_by_independent_nodes() {
    let mut graph = Graph::new();
    let circle = graph.add_node("Circle", (0.0, 0.0)).unwrap();
    graph.set_constant(circle, 1, Value::Number(8.0)).unwrap();
    let first = graph.add_node("Linear Pattern", (1.0, 0.0)).unwrap();
    let second = graph.add_node("Linear Pattern", (2.0, 0.0)).unwrap();
    for id in [first, second] {
        graph.connect_checked(circle, 0, id, 0).unwrap();
        graph.set_constant(id, 1, Value::Number(4.0)).unwrap();
    }
    // Each node fits 50 units in isolation; retaining 8 + 32 leaves only 10 for the next.
    let result = graph.evaluate_with_budget(&|| false, 50).unwrap();
    assert!(result.outputs.contains_key(&(first, 0)));
    assert!(!result.outputs.contains_key(&(second, 0)));
    assert!(result.errors.get(&second).is_some_and(|error| error.contains("budget")));
}

#[test]
fn extrusion_overflow_is_a_node_error_instead_of_a_non_finite_mesh() {
    let mut graph = Graph::new();
    let id = graph.add_node("Extrude", (0.0, 0.0)).unwrap();
    graph
        .set_constant(id, 0, Value::Poly(Poly { pts: vec![Vec3::new(1e308, 0.0, 0.0), Vec3::new(1e308, 1.0, 0.0), Vec3::ZERO], closed: true }))
        .unwrap();
    graph.set_constant(id, 1, Value::Number(1.0)).unwrap();
    let result = graph.evaluate().unwrap();
    assert!(result.errors.get(&id).is_some_and(|error| error.contains("finite")));
    assert!(!result.outputs.contains_key(&(id, 0)));
}
