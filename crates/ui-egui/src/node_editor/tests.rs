use super::*;

fn frame(editor: &mut NodeEditor, ctx: &egui::Context, events: Vec<egui::Event>) {
    crate::scenario::clear_handles();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))),
            focused: true,
            events,
            ..Default::default()
        },
        |ui| {
            editor.show(ui);
        },
    );
    output.textures_delta.clear();
}

fn pointer(at: egui::Pos2, button: egui::PointerButton, pressed: bool) -> Vec<egui::Event> {
    vec![egui::Event::PointerMoved(at), egui::Event::PointerButton { pos: at, button, pressed, modifiers: egui::Modifiers::NONE }]
}

fn click(editor: &mut NodeEditor, ctx: &egui::Context, at: egui::Pos2) {
    frame(editor, ctx, pointer(at, egui::PointerButton::Primary, true));
    frame(editor, ctx, pointer(at, egui::PointerButton::Primary, false));
}

fn idle_editor() -> NodeEditor {
    let mut editor = NodeEditor::default();
    editor.auto_run = false;
    editor.pending_run = false;
    editor
}

#[test]
fn graph_file_failure_preserves_the_open_document_and_history() {
    let mut editor = idle_editor();
    let before = editor.graph.clone();
    assert!(editor.load_json("{bad json}").is_err());
    assert_eq!(editor.graph, before);
    assert!(editor.undo.is_empty());
    let saved = editor.to_json().unwrap();
    editor.load_json(&saved).unwrap();
    assert!(!editor.is_dirty());
}

#[test]
fn hostile_file_errors_keep_a_bounded_utf8_banner_and_preserve_the_graph() {
    let mut editor = idle_editor();
    let before = editor.graph.clone();
    let mut document: serde_json::Value = serde_json::from_str(&editor.to_json().unwrap()).unwrap();
    document["nodes"][0]["component"] = serde_json::Value::String("🧊".repeat(20_000));
    let error = editor.load_json(&document.to_string()).unwrap_err();
    assert!(error.len() > 50_000, "the parser can echo an untrusted component name");
    editor.report_error(error);
    let banner = editor.error.as_ref().unwrap();
    assert!(banner.chars().count() <= MAX_ERROR_CHARS);
    assert!(banner.ends_with('…'));
    assert!(banner.contains('🧊'), "the visible prefix must keep valid Unicode characters");
    assert_eq!(editor.graph, before);
    editor.report_error("A short error");
    assert_eq!(editor.error.as_deref(), Some("A short error"));
}

#[test]
fn stale_worker_results_are_not_shown_for_edited_values() {
    let mut editor = idle_editor();
    let old = editor.graph.evaluate().unwrap();
    let revision = editor.revision;
    editor.changed(0.0);
    editor.finish(Ok(old), revision);
    assert!(editor.evaluation().is_none());
}

#[test]
fn async_save_marks_its_snapshot_and_keeps_new_edits_dirty() {
    let mut editor = idle_editor();
    let saved = editor.to_json().unwrap();
    editor.mark_saved();
    assert!(!editor.is_dirty());
    editor.graph.add_node("Number", (0.0, 0.0)).unwrap();
    editor.mark_saved_json(&saved).unwrap();
    assert!(editor.is_dirty());
    editor.load_json(&saved).unwrap();
    assert!(!editor.is_dirty());
}

#[test]
fn clicking_visible_ports_connects_nodes_and_right_click_finishes_the_gesture() {
    let ctx = crate::fonts::test_context();
    let mut editor = idle_editor();
    let mut graph = Graph::new();
    let source = graph.add_node("Number", (64.0, 64.0)).unwrap();
    let target = graph.add_node("Add", (400.0, 64.0)).unwrap();
    editor.replace_graph(graph).unwrap();
    frame(&mut editor, &ctx, vec![]);
    frame(&mut editor, &ctx, vec![]);
    let from = crate::scenario::handle_at(&format!("nodes:output:{}:0", source.0)).unwrap();
    click(&mut editor, &ctx, from);
    assert_eq!(editor.wire_start, Some((source, 0)));
    let to = crate::scenario::handle_at(&format!("nodes:input:{}:0", target.0)).unwrap();
    click(&mut editor, &ctx, to);
    assert_eq!(editor.graph.nodes[&target].inputs[0].wire, Some(solidcraft_graph::Wire { from: source, output: 0 }));
    assert!(editor.wire_start.is_none());
    click(&mut editor, &ctx, from);
    let at = egui::pos2(700.0, 500.0);
    frame(&mut editor, &ctx, pointer(at, egui::PointerButton::Secondary, true));
    frame(&mut editor, &ctx, pointer(at, egui::PointerButton::Secondary, false));
    assert!(editor.wire_start.is_none());
}

#[test]
fn dragging_a_card_moves_it_and_is_one_undo_step() {
    let ctx = crate::fonts::test_context();
    let mut editor = idle_editor();
    let id = *editor.graph.nodes.keys().next().unwrap();
    let before = editor.graph.clone();
    frame(&mut editor, &ctx, vec![]);
    frame(&mut editor, &ctx, vec![]);
    let start = crate::scenario::handle_at(&format!("nodes:card:{}", id.0)).unwrap();
    frame(&mut editor, &ctx, pointer(start, egui::PointerButton::Primary, true));
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(start + egui::vec2(30.0, 20.0))]);
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(start + egui::vec2(60.0, 40.0))]);
    frame(&mut editor, &ctx, pointer(start + egui::vec2(60.0, 40.0), egui::PointerButton::Primary, false));
    assert_ne!(editor.graph.nodes[&id].at, before.nodes[&id].at);
    assert_eq!(editor.undo.len(), 1);
    editor.undo(0.0);
    assert_eq!(editor.graph, before);
}

#[test]
fn a_visible_slider_changes_its_value_and_undo_restores_it() {
    let ctx = crate::fonts::test_context();
    let mut editor = idle_editor();
    let circle = *editor.graph.nodes.iter().find(|(_, node)| node.component == "Circle").unwrap().0;
    editor.selected = Some(circle);
    let before = editor.graph.clone();
    frame(&mut editor, &ctx, vec![]);
    frame(&mut editor, &ctx, vec![]);
    let field = crate::scenario::rect_of(&format!("nodes:value:{}:0", circle.0)).unwrap();
    // The left part is the slider track; the exact numeric field is on its right.
    let at = egui::pos2(field.left() + field.width() * 0.4, field.center().y);
    click(&mut editor, &ctx, at);
    assert_ne!(editor.graph.nodes[&circle].inputs[0].constant, before.nodes[&circle].inputs[0].constant);
    assert_eq!(editor.undo.len(), 1);
    editor.undo(0.0);
    assert_eq!(editor.graph, before);
}

#[test]
fn a_failed_node_draws_an_error_while_an_independent_output_remains_available() {
    let ctx = crate::fonts::test_context();
    let mut editor = idle_editor();
    let mut graph = Graph::new();
    let bad = graph.add_node("Circle", (64.0, 64.0)).unwrap();
    let good = graph.add_node("Number", (400.0, 64.0)).unwrap();
    graph.set_constant(bad, 0, Value::Number(-1.0)).unwrap();
    editor.replace_graph(graph).unwrap();
    editor.selected = Some(bad);
    let result = editor.graph.evaluate().unwrap();
    let message = result.errors[&bad].clone();
    assert!(result.outputs.contains_key(&(good, 0)));
    editor.finish(Ok(result), editor.revision);
    let mut shapes = Vec::new();
    for _ in 0..2 {
        let mut output = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))), ..Default::default() },
            |ui| {
                editor.show(ui);
            },
        );
        output.textures_delta.clear();
        shapes = std::mem::take(&mut output.shapes);
    }
    fn text(shape: &egui::Shape, output: &mut String) {
        match shape {
            egui::Shape::Text(value) => output.push_str(value.galley.text()),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    text(shape, output);
                }
            }
            _ => {}
        }
    }
    let mut drawn = String::new();
    for shape in &shapes {
        text(&shape.shape, &mut drawn);
    }
    assert!(drawn.contains(tr("ui.nodes.node_error")));
    assert!(drawn.contains(&message));
}

#[test]
fn default_example_makes_a_mesh_without_node_errors() {
    let graph = sample_graph().unwrap();
    let result = graph.evaluate().unwrap();
    assert!(result.errors.is_empty());
    assert!(
        result
            .outputs
            .values()
            .flat_map(|tree| tree.branches.values())
            .flatten()
            .any(|value| matches!(value, Value::Solid(triangles) if !triangles.is_empty()))
    );
}

#[test]
fn resizing_fills_the_workspace_and_keeps_room_for_the_inspector() {
    let ctx = crate::fonts::test_context();
    crate::theme::apply(&ctx);
    let mut editor = idle_editor();
    let tokens = Tokens::get();
    for size in [egui::vec2(1440.0, 900.0), egui::vec2(1920.0, 1080.0), egui::vec2(960.0, 720.0)] {
        let mut bounds = egui::Rect::NOTHING;
        for _ in 0..3 {
            let mut output =
                ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)), ..Default::default() }, |ui| {
                    bounds = ui.available_rect_before_wrap();
                    editor.show(ui);
                });
            output.textures_delta.clear();
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|shape| { matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == tokens.panel && rect.rect.contains_rect(bounds)) }),
                "the toolbar and inspector share the theme background at {size:?}"
            );
            let canvas = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect) if rect.fill == tokens.viewport_bottom => Some(rect.rect.intersect(shape.clip_rect)),
                    _ => None,
                })
                .expect("the visible node canvas is painted");
            assert!((canvas.left() - bounds.left()).abs() <= 1.0, "{canvas:?} in {bounds:?}");
            assert!((canvas.bottom() - bounds.bottom()).abs() <= 2.0, "the canvas fills the available height at {size:?}: {canvas:?}");
            let inspector_space = bounds.right() - canvas.right();
            assert!((240.0..=300.0).contains(&inspector_space), "only the inspector is reserved at {size:?}: {inspector_space}");
        }
    }
}

#[test]
fn editor_renders_real_controls_and_original_icons_in_both_themes() {
    let previous = crate::theme::is_dark();
    let directory = std::env::var_os("NODE_EDITOR_SCREENSHOTS");
    for dark in [false, true] {
        crate::theme::set_dark(dark);
        let ctx = crate::fonts::test_context();
        crate::theme::apply(&ctx);
        let mut editor = idle_editor();
        let evaluation = editor.graph.evaluate().unwrap();
        editor.finish(Ok(evaluation), editor.revision);
        let mut textures = std::collections::HashMap::new();
        let mut last = None;
        crate::icon_svg::audit::start();
        for frame in 0..3 {
            crate::scenario::clear_handles();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))),
                    time: Some(frame as f64 / 10.0),
                    focused: true,
                    ..Default::default()
                },
                |ui| {
                    editor.show(ui);
                },
            );
            crate::ui_kit::update_test_textures(&mut textures, &output.textures_delta);
            output.textures_delta.clear();
            last = Some(output);
        }
        let icons = crate::icon_svg::audit::take();
        crate::icon_svg::audit::stop();
        assert!(!icons.is_empty());
        assert!(icons.iter().all(|icon| !icon.placeholder), "{icons:?}");
        for id in editor.graph.nodes.keys() {
            let card = crate::scenario::rect_of(&format!("nodes:card:{}", id.0)).unwrap();
            assert!(ctx.content_rect().contains_rect(card));
        }
        if let Some(directory) = &directory {
            std::fs::create_dir_all(directory).unwrap();
            let output = last.unwrap();
            let image = crate::ui_kit::raster_ui(&ctx, output.shapes, output.pixels_per_point, ctx.content_rect(), &textures);
            image.save(std::path::Path::new(directory).join(if dark { "node-editor-dark.png" } else { "node-editor-light.png" })).unwrap();
        }
    }
    crate::theme::set_dark(previous);
}

#[test]
fn explicit_unsaved_state_survives_content_equality_and_can_be_saved_again() {
    let mut editor = idle_editor();
    let document = editor.to_json().unwrap();
    editor.mark_saved();
    assert!(!editor.is_dirty());
    editor.mark_unsaved();
    assert!(editor.is_dirty());
    assert_eq!(editor.to_json().unwrap(), document);
    editor.mark_saved_json(&document).unwrap();
    assert!(!editor.is_dirty());
}
