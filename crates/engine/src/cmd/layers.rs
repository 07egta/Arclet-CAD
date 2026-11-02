//! Layer commands: named sets of bodies with an eye and a lock, and the current layer. The
//! layers themselves are kept in the design (`doc.layers`); an eye or a lock is applied to the
//! bodies on the layer through the browser's own hidden and locked lists, so the rest of the app
//! already respects it. Every command is an undo step.

use serde_json::{Value, json};
use solidcraft_doc::layers::{DEFAULT, Layer, is_hex_colour};

use super::CommandSpec;
use crate::params::bad;
use crate::schema::layers::{AssignParams, CurrentParams, DeleteParams, LayerReply, LockedParams, NewParams, RenameParams, VisibleParams};
use crate::{Result, Session};

const PALETTE: [&str; 6] = ["#4a90d9", "#e8833a", "#5cb85c", "#d9534f", "#9b59b6", "#f1c40f"];

const NEW_CONTRACT: crate::schema::TypedContract<NewParams> = crate::schema::TypedContract::new();
const DELETE_CONTRACT: crate::schema::TypedContract<DeleteParams> = crate::schema::TypedContract::new();
const RENAME_CONTRACT: crate::schema::TypedContract<RenameParams> = crate::schema::TypedContract::new();
const VISIBLE_CONTRACT: crate::schema::TypedContract<VisibleParams> = crate::schema::TypedContract::new();
const LOCKED_CONTRACT: crate::schema::TypedContract<LockedParams> = crate::schema::TypedContract::new();
const CURRENT_CONTRACT: crate::schema::TypedContract<CurrentParams> = crate::schema::TypedContract::new();
const ASSIGN_CONTRACT: crate::schema::TypedContract<AssignParams> = crate::schema::TypedContract::new();

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("layer.new", "New Layer", crate::checked!("layer.new", new_layer))
        .icon("folder")
        .contract(&NEW_CONTRACT)
        .output(crate::schema::output_schema_for::<LayerReply>)
        .params("name?: text (default Layer N); colour?: #rrggbb (default: the next palette colour) → layer: id, name, colour"),
    CommandSpec::new("layer.delete", "Delete Layer", crate::checked!("layer.delete", delete_layer))
        .icon("delete")
        .contract(&DELETE_CONTRACT)
        .output(crate::schema::output_schema_for::<LayerReply>)
        .params("layer: id (not 0); objects are returned to layer 0 → layer, bodies"),
    CommandSpec::new("layer.rename", "Rename Layer", crate::checked!("layer.rename", rename_layer))
        .icon("rename")
        .contract(&RENAME_CONTRACT)
        .output(crate::schema::output_schema_for::<LayerReply>)
        .params("layer: id (not 0), name: text → layer, name"),
    CommandSpec::new("layer.set_visible", "Show/Hide Layer", crate::checked!("layer.set_visible", set_visible))
        .icon("eye")
        .contract(&VISIBLE_CONTRACT)
        .output(crate::schema::output_schema_for::<LayerReply>)
        .params("layer: id (not 0), visible: bool; its bodies are hidden or shown with it → layer, visible"),
    CommandSpec::new("layer.set_locked", "Lock/Unlock Layer", crate::checked!("layer.set_locked", set_locked))
        .icon("lock")
        .contract(&LOCKED_CONTRACT)
        .output(crate::schema::output_schema_for::<LayerReply>)
        .params("layer: id (not 0), locked: bool; its bodies are locked with it → layer, locked"),
    CommandSpec::new("layer.set_current", "Set Current Layer", crate::checked!("layer.set_current", set_current))
        .icon("folder")
        .contract(&CURRENT_CONTRACT)
        .output(crate::schema::output_schema_for::<LayerReply>)
        .params("layer: id (0 = the default layer) → layer"),
    CommandSpec::new("layer.assign", "Move to Layer", crate::checked!("layer.assign", assign))
        .icon("reorder")
        .contract(&ASSIGN_CONTRACT)
        .output(crate::schema::output_schema_for::<LayerReply>)
        .params("bodies: [body names], layer: id (0 = the default layer); the bodies take that layer's eye and lock → layer, bodies (count)"),
];

/// The layer a command names: a listed layer, or the default layer (0).
fn layer_id(s: &Session, p: &Value, cmd: &str) -> Result<u64> {
    let id = p.get("layer").and_then(Value::as_u64).ok_or_else(|| bad(cmd, "`layer` must be a layer id"))?;
    if id == DEFAULT || s.doc.layers.get(id).is_some() { Ok(id) } else { Err(bad(cmd, format!("no layer {id}"))) }
}

fn name_of(p: &Value, cmd: &str) -> Result<String> {
    match p.get("name").and_then(Value::as_str).map(str::trim) {
        Some(n) if !n.is_empty() && n.len() <= 128 => Ok(n.to_string()),
        _ => Err(bad(cmd, "`name` must be 1…128 characters")),
    }
}

fn flag(p: &Value, key: &str, cmd: &str) -> Result<bool> {
    p.get(key).and_then(Value::as_bool).ok_or_else(|| bad(cmd, format!("`{key}` must be true or false")))
}

fn new_layer(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "layer.new";
    let count = s.doc.layers.items.len();
    let name = match p.get("name") {
        None | Some(Value::Null) => format!("Layer {}", count + 1),
        Some(_) => name_of(p, cmd)?,
    };
    let colour = match p.get("colour").and_then(Value::as_str) {
        Some(c) if is_hex_colour(c) => c.to_string(),
        Some(_) => return Err(bad(cmd, "`colour` must be #rrggbb")),
        None => PALETTE[count % PALETTE.len()].to_string(),
    };
    let d = s.doc_mut();
    let id = d.allocate_id()?;
    d.layers.items.push(Layer { id, name: name.clone(), colour: colour.clone(), visible: true, locked: false });
    Ok(json!({ "layer": id, "name": name, "colour": colour }))
}

fn delete_layer(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "layer.delete";
    let id = layer_id(s, p, cmd)?;
    if id == DEFAULT {
        return Err(bad(cmd, "the default layer cannot be deleted"));
    }
    let members = s.doc.layers.members(id);
    let doc = s.doc_mut();
    doc.layers.items.retain(|layer| layer.id != id);
    doc.layers.of.retain(|_, assigned| *assigned != id);
    if doc.layers.current == id {
        doc.layers.current = DEFAULT;
    }
    for body in &members {
        sync_body(s, body);
    }
    Ok(json!({ "layer": id, "bodies": members.len() }))
}

fn rename_layer(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "layer.rename";
    let id = layer_id(s, p, cmd)?;
    if id == DEFAULT {
        return Err(bad(cmd, "the default layer keeps its name"));
    }
    let name = name_of(p, cmd)?;
    if let Some(layer) = s.doc_mut().layers.items.iter_mut().find(|l| l.id == id) {
        layer.name = name.clone();
    }
    Ok(json!({ "layer": id, "name": name }))
}

/// Apply a layer's eye and lock to every body on it, through the browser's lists.
fn sync_members(s: &mut Session, id: u64) {
    for body in s.doc.layers.members(id) {
        sync_body(s, &body);
    }
}

fn sync_body(s: &mut Session, body: &str) {
    let (visible, locked) = s.doc.layers.state_of(body);
    let hidden = &mut s.visibility.hidden_bodies;
    hidden.retain(|b| b != body);
    if !visible {
        hidden.push(body.to_string());
    }
    let lock = &mut s.visibility.locked_bodies;
    lock.retain(|b| b != body);
    if locked {
        lock.push(body.to_string());
    }
}

fn set_visible(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "layer.set_visible";
    let id = layer_id(s, p, cmd)?;
    if id == DEFAULT {
        return Err(bad(cmd, "the default layer has no eye yet"));
    }
    let visible = flag(p, "visible", cmd)?;
    if let Some(layer) = s.doc_mut().layers.items.iter_mut().find(|l| l.id == id) {
        layer.visible = visible;
    }
    sync_members(s, id);
    Ok(json!({ "layer": id, "visible": visible }))
}

fn set_locked(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "layer.set_locked";
    let id = layer_id(s, p, cmd)?;
    if id == DEFAULT {
        return Err(bad(cmd, "the default layer has no lock yet"));
    }
    let locked = flag(p, "locked", cmd)?;
    if let Some(layer) = s.doc_mut().layers.items.iter_mut().find(|l| l.id == id) {
        layer.locked = locked;
    }
    sync_members(s, id);
    Ok(json!({ "layer": id, "locked": locked }))
}

fn set_current(s: &mut Session, p: &Value) -> Result<Value> {
    let id = layer_id(s, p, "layer.set_current")?;
    s.doc_mut().layers.current = id;
    Ok(json!({ "layer": id }))
}

fn assign(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "layer.assign";
    let id = layer_id(s, p, cmd)?;
    let bodies: Vec<String> = match p.get("bodies") {
        Some(Value::Array(a)) => a
            .iter()
            .map(|b| b.as_str().filter(|n| !n.is_empty() && n.len() <= 256).map(str::to_string))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| bad(cmd, "`bodies` must be a list of body names"))?,
        _ => return Err(bad(cmd, "`bodies` must be a list of body names")),
    };
    if bodies.len() > 10_000 {
        return Err(bad(cmd, "`bodies` is too long"));
    }
    let present: std::collections::BTreeSet<String> =
        s.world_state().bodies.iter().map(|body| body.name.clone()).collect();
    if let Some(missing) = bodies.iter().find(|body| !present.contains(*body)) {
        return Err(bad(cmd, format!("no body `{missing}`")));
    }
    let layers = &mut s.doc_mut().layers;
    for body in &bodies {
        if id == DEFAULT {
            layers.of.remove(body);
        } else {
            layers.of.insert(body.clone(), id);
        }
    }
    for body in &bodies {
        sync_body(s, body);
    }
    Ok(json!({ "layer": id, "bodies": bodies.len() }))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::Session;

    fn run(s: &mut Session, id: &str, p: Value) -> Value {
        match s.execute(id, &p) {
            Ok(v) => v,
            Err(e) => panic!("{id} {p}: {e}"),
        }
    }

    fn two_bodies() -> Session {
        let mut s = Session::default();
        run(&mut s, "solid.box", json!({"length": 10, "width": 10, "height": 10, "body_name": "Plate"}));
        run(&mut s, "solid.box", json!({"length": 5, "width": 5, "height": 5, "corner": [20, 0, 0], "body_name": "Pin"}));
        s
    }

    #[test]
    fn a_layer_hides_and_locks_its_bodies_and_undo_restores_them() {
        let mut s = two_bodies();
        let layer = run(&mut s, "layer.new", json!({"name": "Parts"}))["layer"].as_u64().unwrap();
        run(&mut s, "layer.assign", json!({"bodies": ["Plate"], "layer": layer}));
        run(&mut s, "layer.set_visible", json!({"layer": layer, "visible": false}));
        run(&mut s, "layer.set_locked", json!({"layer": layer, "locked": true}));
        assert_eq!(s.visibility.hidden_bodies, vec!["Plate".to_string()]);
        assert_eq!(s.visibility.locked_bodies, vec!["Plate".to_string()]);
        assert!(s.visibility.hidden_bodies.iter().all(|b| b != "Pin"), "a body on the default layer is untouched");
        run(&mut s, "layer.set_visible", json!({"layer": layer, "visible": true}));
        assert!(s.visibility.hidden_bodies.is_empty());
        // Moving the body back to the default layer releases the lock it took from the layer.
        run(&mut s, "layer.assign", json!({"bodies": ["Plate"], "layer": 0}));
        assert!(s.visibility.locked_bodies.is_empty());
        assert_eq!(s.doc.layers.layer_of("Plate"), 0);
        run(&mut s, "edit.undo", json!({}));
        assert_eq!(s.doc.layers.layer_of("Plate"), layer, "the assignment is one undo step");
    }

    #[test]
    fn layer_names_colours_and_the_current_layer_are_checked() {
        let mut s = two_bodies();
        let made = run(&mut s, "layer.new", json!({}));
        assert_eq!(made["name"], "Layer 1");
        assert_eq!(made["colour"], "#4a90d9");
        let id = made["layer"].as_u64().unwrap();
        run(&mut s, "layer.rename", json!({"layer": id, "name": "Glass"}));
        run(&mut s, "layer.set_current", json!({"layer": id}));
        assert_eq!(s.doc.layers.current, id);
        assert_eq!(s.doc.layers.items[0].name, "Glass");
        let bad = [
            ("layer.new", json!({"colour": "red"})),
            ("layer.new", json!({"name": ""})),
            ("layer.rename", json!({"layer": 0, "name": "Default"})),
            ("layer.set_visible", json!({"layer": 0, "visible": false})),
            ("layer.set_visible", json!({"layer": 999, "visible": false})),
            ("layer.assign", json!({"bodies": "Plate", "layer": id})),
            ("layer.set_locked", json!({"layer": id, "locked": "yes"})),
        ];
        for (cmd, p) in bad {
            assert!(s.execute(cmd, &p).is_err(), "{cmd} {p} must be refused");
        }
    }

    #[test]
    fn deletion_rehomes_members_and_is_undoable() {
        let mut s = two_bodies();
        let id = run(&mut s, "layer.new", json!({"name": "Discardable"}))["layer"].as_u64().unwrap();
        run(&mut s, "layer.assign", json!({"layer": id, "bodies": ["Plate", "Pin"]}));
        run(&mut s, "layer.set_current", json!({"layer": id}));
        assert!(s.execute("layer.delete", &json!({"layer": 0})).is_err());
        assert_eq!(run(&mut s, "layer.delete", json!({"layer": id}))["bodies"], 2);
        assert_eq!(s.doc.layers.current, 0);
        assert_eq!(s.doc.layers.layer_of("Plate"), 0);
        assert!(s.world_state().body("Plate").is_some());
        run(&mut s, "edit.undo", json!({}));
        assert_eq!(s.doc.layers.layer_of("Plate"), id);
        run(&mut s, "edit.redo", json!({}));
        assert_eq!(s.doc.layers.layer_of("Plate"), 0);
    }

    #[test]
    fn unknown_assignment_is_atomic() {
        let mut s = two_bodies();
        let id = run(&mut s, "layer.new", json!({}))["layer"].as_u64().unwrap();
        let before = s.doc.layers.clone();
        assert!(s.execute("layer.assign", &json!({"layer": id, "bodies": ["Plate", "Missing"]})).is_err());
        assert_eq!(s.doc.layers, before);
    }

    #[test]
    fn renaming_a_body_keeps_its_layer() {
        let mut s = two_bodies();
        let layer = run(&mut s, "layer.new", json!({}))["layer"].as_u64().unwrap();
        run(&mut s, "layer.assign", json!({"bodies": ["Plate"], "layer": layer}));
        run(&mut s, "body.rename", json!({"body": "Plate", "name": "Base"}));
        assert_eq!(s.doc.layers.layer_of("Base"), layer);
    }
}
