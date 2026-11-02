//! 3D sketch drawing: with the sketch's 3D Sketch option on (Sketch Palette), lines, arcs,
//! splines and points may leave the sketch plane. They are kept as drawn 3D curves (wires with
//! fit points, exact arcs keeping their analytic form): drawn and snapped to, movable point by
//! point, and usable as sweep and pipe paths.

use serde_json::{Value, json};
use solidcraft_geom::{Plane, Vec3};
use solidcraft_sketch::WireSpace;

use super::sketch::edit;
use super::sketch_store;
use super::{CommandSpec, in_sketch};
use crate::params::{bad, req_vec3, vec3};
use crate::{Result, Session};

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("sketch.line3d", "3D Line", line3d).icon("line").enabled(in_sketch).params(concat!(
        "points: [[x,y,z]…] (2 or more); closed?: bool (3 or more points, joins last to first); frame?: world (default) | sketch (local xyz in the stored sketch plane, with z along its normal): one straight 3D wire per segment, joined end to end (needs 3D Sketch on)",
        created_doc!()
    )),
    CommandSpec::new("sketch.polyline3d", "3D Polyline", polyline3d).icon("line").enabled(in_sketch).params(concat!(
        "points: [[x,y,z]…] (2 or more); closed?: bool; frame?: world (default) | sketch: a chain of straight 3D segments with stable independent curve ids; see sketch.line3d (needs 3D Sketch on)",
        created_doc!()
    )),
    CommandSpec::new("sketch.spline3d", "3D Spline", spline3d)
        .icon("spline")
        .enabled(in_sketch)
        .params(concat!("points: [[x,y,z]…] fit points (2 or more); closed?: bool (3 or more distinct points, smooth periodic curve); frame?: world (default) | sketch: a smooth 3D curve through them (needs 3D Sketch on)", created_doc!())),
    CommandSpec::new("sketch.arc3d", "3D Arc", arc3d)
        .icon("arc3")
        .enabled(in_sketch_3d)
        .params(concat!(
            "start, through, end: [x,y,z] points; frame?: world (default) | sketch (local XYZ). Exact circular arc through the points (needs 3D Sketch on), stored analytically for path consumers",
            created_doc!()
        )),
    CommandSpec::new("sketch.point3d", "3D Point", point3d)
        .icon("point")
        .enabled(in_sketch)
        .params(concat!("point: [x,y,z]; frame?: world (default) | sketch (local XYZ); needs 3D Sketch on", created_doc!())),
    CommandSpec::new("sketch.move3d", "Move 3D Point", move3d)
        .icon("sketch_move")
        .enabled(in_sketch)
        .params("wire: drawn 3D curve or point id, index?: fit point (default 0), to: [x,y,z] | by: [dx,dy,dz]; frame?: world (default) | sketch (local XYZ); points joined to it move too"),
    CommandSpec::new("sketch.insert_fit3d", "Insert 3D Spline Fit Point", insert_fit3d)
        .icon("spline").enabled(in_sketch)
        .params("wire: drawn 3D spline id (3 or more fit points, not an arc or link); index: interior insertion index (before this point); at: [x,y,z]; frame?: world (default) | sketch (local XYZ); preserves wire id for downstream features"),
    CommandSpec::new("sketch.remove_fit3d", "Remove 3D Spline Fit Point", remove_fit3d)
        .icon("spline").enabled(in_sketch)
        .params("wire: drawn 3D spline id (3 or more fit points, not an arc or link); index: interior fit-point index to remove; preserves wire id and curve endpoints"),
];

fn points(cmd: &str, p: &Value, min: usize) -> Result<Vec<Vec3>> {
    let a = p.get("points").and_then(Value::as_array).ok_or_else(|| bad(cmd, "`points` must list [x,y,z] points"))?;
    if a.len() < min || a.len() > solidcraft_sketch::MAX_FIT_POINTS {
        return Err(bad(cmd, format!("`points` needs {min} to {} points", solidcraft_sketch::MAX_FIT_POINTS)));
    }
    a.iter().map(|v| vec3(v).ok_or_else(|| bad(cmd, "a point must be [x, y, z]"))).collect()
}

/// User-supplied XYZ is normally world-space. With frame:"sketch", XYZ is measured
/// along the selected sketch plane's local orthonormal x/y/normal basis. The evaluated plane
/// already includes the active occurrence's placement; the same conversion works for a sketch
/// owned by a rotated component and for a saved construction plane.
fn input_frame(s: &Session, p: &Value, cmd: &str) -> Result<Option<Plane>> {
    match p.get("frame") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(v)) if v == "world" => Ok(None),
        Some(Value::String(v)) if v == "sketch" => {
            let id = sketch_store::target(s, p, cmd)?;
            s.world_state().sketch(id).map(|sk| sk.plane).ok_or_else(|| bad(cmd, "the sketch plane could not be evaluated"))
        }
        _ => Err(bad(cmd, "`frame` must be \"world\" or \"sketch\"")),
    }
}

fn framed_vector(frame: Option<Plane>, delta: Vec3) -> Vec3 {
    frame.map_or(delta, |pl| pl.x * delta.x + pl.y * delta.y + pl.normal() * delta.z)
}

fn framed_point(frame: Option<Plane>, point: Vec3) -> Vec3 {
    frame.map_or(point, |pl| pl.origin + framed_vector(Some(pl), point))
}

fn closed_flag(p: &Value, cmd: &str) -> Result<bool> {
    match p.get("closed") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        _ => Err(bad(cmd, "`closed` must be true or false")),
    }
}

fn fit_index(p: &Value, cmd: &str) -> Result<usize> {
    p.get("index")
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .filter(|i| *i < solidcraft_sketch::MAX_FIT_POINTS)
        .ok_or_else(|| bad(cmd, "`index` must be a fit point index below 500"))
}

fn needs_3d(sk: &solidcraft_sketch::Sketch, cmd: &str) -> Result<()> {
    if sk.view.three_d { Ok(()) } else { Err(bad(cmd, "turn on 3D Sketch (sketch.options sketch_3d: true) to draw off the sketch plane")) }
}

/// The 3D Arc button starts an interactive tool, so it is only offered while a 3D sketch is
/// being edited; otherwise the tool could only fail on its third click.
fn in_sketch_3d(s: &Session) -> std::result::Result<(), String> {
    let id = sketch_store::active(s).ok_or("no sketch is being edited (Create Sketch or Edit Sketch first)")?;
    match sketch_store::read(&s.doc, id) {
        Ok(sk) if sk.view.three_d => Ok(()),
        Ok(_) => Err("turn on 3D Sketch (sketch.options sketch_3d: true) to draw 3D arcs".into()),
        Err(_) => Err("no sketch is being edited (Create Sketch or Edit Sketch first)".into()),
    }
}

/// World command inputs belong to the target sketch's owner, which may differ from the
/// active component when a script supplies an explicit sketch.
fn inverse_owner_frame(s: &Session, p: &Value, cmd: &str) -> Result<solidcraft_doc::Mat> {
    let id = sketch_store::target(s, p, cmd)?;
    let owner = s.doc.feature(id).map(|f| f.component).ok_or_else(|| bad(cmd, "target sketch is missing"))?;
    if owner != 0 && owner != s.active_component {
        if s.doc.occurrences.len() > 100_000 {
            return Err(bad(cmd, "sketch owner placement expansion exceeds supported bounds"));
        }
        let mut children: std::collections::BTreeMap<u64, Vec<&solidcraft_doc::Occurrence>> = std::collections::BTreeMap::new();
        for occurrence in &s.doc.occurrences {
            children.entry(occurrence.parent).or_default().push(occurrence);
        }
        let mut stack = vec![(0, 0usize)];
        let mut count = 0;
        let mut visited = 0;
        while let Some((component, depth)) = stack.pop() {
            for o in children.get(&component).into_iter().flatten() {
                visited += 1;
                if depth >= 64 || visited > 100_000 {
                    return Err(bad(cmd, "sketch owner placement expansion exceeds supported bounds"));
                }
                if o.component == owner {
                    count += 1;
                    if count > 1 {
                        return Err(bad(cmd, "activate the desired sketch owner occurrence before editing world 3D points"));
                    }
                }
                stack.push((o.component, depth + 1));
            }
        }
    }
    let frame = if owner == s.active_component { s.active_component_frame() } else { s.placed_document().component_transform(owner) };
    solidcraft_doc::mat_inverse(&frame).ok_or_else(|| bad(cmd, "the sketch owner placement is not invertible"))
}

fn line3d(s: &mut Session, p: &Value) -> Result<Value> {
    draw_segments3d(s, p, "sketch.line3d")
}

fn polyline3d(s: &mut Session, p: &Value) -> Result<Value> {
    draw_segments3d(s, p, "sketch.polyline3d")
}

fn draw_segments3d(s: &mut Session, p: &Value, cmd: &str) -> Result<Value> {
    let inverse = inverse_owner_frame(s, p, cmd)?;
    let frame = input_frame(s, p, cmd)?;
    let closed = closed_flag(p, cmd)?;
    let world = points(cmd, p, if closed { 3 } else { 2 })?;
    if closed && world.first().zip(world.last()).is_some_and(|(a, b)| a.dist(*b) < 1e-9) {
        return Err(bad(cmd, "closed polylines must not repeat their first point; use closed: true"));
    }
    let pts: Vec<Vec3> = world.into_iter().map(|at| solidcraft_doc::apply_point(&inverse, framed_point(frame, at))).collect();
    let (ids, info) = edit(s, p, cmd, false, |sk, _| {
        needs_3d(sk, cmd)?;
        let mut ids = pts.windows(2).map(|w| sk.add_drawn_wire_in(w, WireSpace::Component).map_err(Into::into)).collect::<Result<Vec<String>>>()?;
        if closed {
            let first = pts.first().copied().ok_or_else(|| bad(cmd, "missing first point"))?;
            let last = pts.last().copied().ok_or_else(|| bad(cmd, "missing last point"))?;
            ids.push(sk.add_drawn_wire_in(&[last, first], WireSpace::Component)?);
        }
        Ok(ids)
    })?;
    Ok(json!({"curves": ids, "sketch": info}))
}

fn spline3d(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.spline3d";
    let inverse = inverse_owner_frame(s, p, cmd)?;
    let frame = input_frame(s, p, cmd)?;
    let closed = closed_flag(p, cmd)?;
    let mut pts = points(cmd, p, if closed { 3 } else { 2 })?;
    if closed {
        if pts.first().zip(pts.last()).is_some_and(|(a, b)| a.dist(*b) < 1e-9) {
            return Err(bad(cmd, "closed splines must not repeat their first fit point; use closed: true"));
        }
        let start = pts.first().copied().ok_or_else(|| bad(cmd, "missing first fit point"))?;
        pts.push(start);
    }
    let pts: Vec<Vec3> = pts.into_iter().map(|at| solidcraft_doc::apply_point(&inverse, framed_point(frame, at))).collect();
    let (id, info) = edit(s, p, cmd, false, |sk, _| {
        needs_3d(sk, cmd)?;
        Ok(sk.add_drawn_wire_in(&pts, WireSpace::Component)?)
    })?;
    Ok(json!({"curves": [id], "sketch": info}))
}

fn arc3d(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.arc3d";
    let inverse = inverse_owner_frame(s, p, cmd)?;
    let frame = input_frame(s, p, cmd)?;
    let pts: Vec<Vec3> = ["start", "through", "end"]
        .iter()
        .map(|k| req_vec3(cmd, p, k))
        .collect::<Result<Vec<Vec3>>>()?
        .into_iter()
        .map(|at| solidcraft_doc::apply_point(&inverse, framed_point(frame, at)))
        .collect();
    let (id, info) = edit(s, p, cmd, false, |sk, _| {
        needs_3d(sk, cmd)?;
        Ok(sk.add_drawn_arc_in(&pts, WireSpace::Component)?)
    })?;
    Ok(json!({"curves": [id], "sketch": info}))
}

fn point3d(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.point3d";
    let inverse = inverse_owner_frame(s, p, cmd)?;
    let frame = input_frame(s, p, cmd)?;
    let at = solidcraft_doc::apply_point(&inverse, framed_point(frame, req_vec3(cmd, p, "point")?));
    let (id, info) = edit(s, p, cmd, false, |sk, _| {
        needs_3d(sk, cmd)?;
        Ok(sk.add_drawn_wire_in(&[at], WireSpace::Component)?)
    })?;
    Ok(json!({"point": id, "sketch": info}))
}

fn move3d(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.move3d";
    let wire = p.get("wire").and_then(Value::as_str).ok_or_else(|| bad(cmd, "`wire` must be a 3D curve or point id"))?.to_string();
    let index = match p.get("index") {
        None => 0,
        Some(v) => v
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .filter(|v| *v < solidcraft_sketch::MAX_FIT_POINTS)
            .ok_or_else(|| bad(cmd, "`index` must be a fit point index below 500"))?,
    };
    let parse = |name| p.get(name).map(|v| vec3(v).ok_or_else(|| bad(cmd, format!("`{name}` must be [x,y,z]")))).transpose();
    let to = parse("to")?;
    let by = parse("by")?;
    let inverse = inverse_owner_frame(s, p, cmd)?;
    let frame = input_frame(s, p, cmd)?;
    let (n, info) = edit(s, p, cmd, false, |sk, _| {
        let drawn = sk
            .wire_index(&wire)
            .and_then(|w| sk.wires.get(w))
            .filter(|w| !w.fit.is_empty() && w.link.is_none())
            .ok_or_else(|| bad(cmd, format!("`{wire}` is not an editable drawn 3D curve")))?;
        let from = drawn.fit.get(index).copied().ok_or_else(|| bad(cmd, format!("`{wire}` has no drawn point {index}")))?;
        let to = match (to, by) {
            (Some(t), _) => {
                if drawn.space == WireSpace::Component {
                    solidcraft_doc::apply_point(&inverse, framed_point(frame, t))
                } else {
                    framed_point(frame, t)
                }
            }
            (None, Some(d)) => from + if drawn.space == WireSpace::Component { solidcraft_doc::apply_vector(&inverse, framed_vector(frame, d)) } else { framed_vector(frame, d) },
            _ => return Err(bad(cmd, "give `to` or `by`")),
        };
        Ok(sk.move_drawn_point(&wire, index, to)?)
    })?;
    Ok(json!({"moved": n, "sketch": info}))
}

fn insert_fit3d(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.insert_fit3d";
    let wire = p.get("wire").and_then(Value::as_str).ok_or_else(|| bad(cmd, "`wire` must be a 3D spline id"))?.to_owned();
    let index = fit_index(p, cmd)?;
    let frame = input_frame(s, p, cmd)?;
    let world = framed_point(frame, req_vec3(cmd, p, "at")?);
    let inverse = inverse_owner_frame(s, p, cmd)?;
    let (count, info) = edit(s, p, cmd, false, |sk, _| {
        let space = sk.wire_index(&wire).and_then(|i| sk.wires.get(i)).map(|w| w.space).ok_or_else(|| bad(cmd, "unknown wire"))?;
        let at = if space == WireSpace::Component { solidcraft_doc::apply_point(&inverse, world) } else { world };
        Ok(sk.insert_drawn_fit_point(&wire, index, at)?)
    })?;
    Ok(json!({"wire":wire,"fit_points":count,"sketch":info}))
}

fn remove_fit3d(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.remove_fit3d";
    let wire = p.get("wire").and_then(Value::as_str).ok_or_else(|| bad(cmd, "`wire` must be a 3D spline id"))?.to_owned();
    let index = fit_index(p, cmd)?;
    let (count, info) = edit(s, p, cmd, false, |sk, _| Ok(sk.remove_drawn_fit_point(&wire, index)?))?;
    Ok(json!({"wire":wire,"fit_points":count,"sketch":info}))
}

#[cfg(test)]
#[path = "sketch_3d_frame_tests.rs"]
mod frame_tests;
