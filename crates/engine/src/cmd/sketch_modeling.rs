//! Modeling commands built on the existing sketch editor and 3D wire representation.
//! Join preserves the exact component curves (G0, not a Rhino PolyCurve); the two
//! parametric generators are sampled fit curves, not analytic NURBS helices.
use std::collections::BTreeSet;
use std::f64::consts::TAU;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use solidcraft_geom::{Vec2, Vec3};
use solidcraft_sketch::{ConstraintKind, CurveKind};

use super::sketch::{add_c, edit};
use super::{CommandSpec, in_sketch};
use crate::params::bad;
use crate::schema::sketch_modeling::{HelixParams, JoinParams, JoinResult, PathResult, PolylineParams, PolylineResult, SpiralParams};
use crate::schema::{TypedContract, output_schema_for};
use crate::{Result, Session};

const POLYLINE_CONTRACT: TypedContract<PolylineParams> = TypedContract::new();
const JOIN_CONTRACT: TypedContract<JoinParams> = TypedContract::new();
const HELIX_CONTRACT: TypedContract<HelixParams> = TypedContract::new();
const SPIRAL_CONTRACT: TypedContract<SpiralParams> = TypedContract::new();

fn in_sketch_3d(s: &Session) -> std::result::Result<(), String> {
    in_sketch(s)?;
    let id = s.active_sketch.ok_or_else(|| "no active sketch".to_string())?;
    let sketch = s.doc.sketch(id).map_err(|_| "active sketch was not found".to_string())?;
    if sketch.view.three_d { Ok(()) } else { Err("turn on 3D Sketch (sketch.options sketch_3d: true) to create a helix".into()) }
}

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("sketch.polyline", "Polyline", crate::checked!("sketch.polyline", polyline))
        .at("SKETCH", "CREATE")
        .icon("line")
        .enabled(in_sketch)
        .contract(&POLYLINE_CONTRACT)
        .output(output_schema_for::<PolylineResult>)
        .params("points: [[x,y]...] (2..1000); closed?, construction?, infer?, connect?: bool; sketch?: id|name. Creates connected exact line segments in one undo step (uses sketch.line). Returns curves, constraints, sketch."),
    CommandSpec::new("sketch.join", "Join Curves", crate::checked!("sketch.join", join))
        .at("SKETCH", "MODIFY")
        .icon("chain")
        .enabled(in_sketch)
        .contract(&JOIN_CONTRACT)
        .output(output_schema_for::<JoinResult>)
        .params("curves: [2..64 existing open curve ids], tolerance?: 0..0.01 mm (default 0.000001), sketch?: id|name. Orders one non-branching chain, constrains touching endpoints coincident in one undo step and preserves exact source curves. G0 only: returns curves in chain order, reversed, closed, constraints, sketch; does not create a single PolyCurve."),
    CommandSpec::new("sketch.helix", "Helix", crate::checked!("sketch.helix", helix))
        .at("SKETCH", "CREATE")
        .icon("spline")
        .enabled(in_sketch_3d)
        .contract(&HELIX_CONTRACT)
        .output(output_schema_for::<PathResult>)
        .params("center, axis: world [x,y,z], radius > 0, turns > 0, height != 0; phase?: degrees (default 0), samples_per_turn?: 4..256 (default 48), sketch?: id|name. Requires sketch_3d:true. Returns one sampled 3D fit wire usable as a sweep/pipe path, its id, sample count and approximation status."),
    CommandSpec::new("sketch.spiral", "Spiral", crate::checked!("sketch.spiral", spiral))
        .at("SKETCH", "CREATE")
        .icon("spline")
        .enabled(in_sketch)
        .contract(&SPIRAL_CONTRACT)
        .output(output_schema_for::<PathResult>)
        .params("center: sketch [x,y], start_radius, end_radius: mm (nonnegative and different), turns > 0; phase?: degrees (default 0), samples_per_turn?: 4..256 (default 48), sketch?: id|name. Creates one sampled planar fit spline; returns curve, samples, approximation, sketch."),
];

fn parse<P: DeserializeOwned>(cmd: &str, p: &Value) -> Result<P> {
    serde_json::from_value(p.clone()).map_err(|e| bad(cmd, e.to_string()))
}

fn finite(cmd: &str, name: &str, value: f64) -> Result<f64> {
    if value.is_finite() && value.abs() < 1e9 {
        Ok(value)
    } else {
        Err(bad(cmd, format!("'{}' must be a finite number smaller than 1e9 in magnitude", name)))
    }
}

fn samples(cmd: &str, turns: f64, per_turn: usize) -> Result<usize> {
    if !turns.is_finite() || !(0.0 < turns && turns <= 100.0) {
        return Err(bad(cmd, "'turns' must be finite, greater than zero and at most 100"));
    }
    if !(4..=256).contains(&per_turn) {
        return Err(bad(cmd, "'samples_per_turn' must be between 4 and 256"));
    }
    let count = (turns * per_turn as f64).ceil() as usize;
    if count < 1 || count + 1 > solidcraft_sketch::MAX_FIT_POINTS {
        return Err(bad(
            cmd,
            format!("too many fit points: at most {} including endpoints; reduce turns or samples_per_turn", solidcraft_sketch::MAX_FIT_POINTS),
        ));
    }
    Ok(count)
}

/// Keep the already-implemented sketch.line as the only source of line geometry.
fn polyline(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.polyline";
    let args: PolylineParams = parse(cmd, p)?;
    if !(2..=1000).contains(&args.points.len()) {
        return Err(bad(cmd, "polyline needs 2 to 1000 points"));
    }
    if args.closed && args.points.len() < 3 {
        return Err(bad(cmd, "a closed polyline needs at least 3 vertices"));
    }
    let points: Vec<Vec2> = args.points.iter().map(|v| v.0).collect();
    for pair in points.windows(2) {
        if pair[0].dist(pair[1]) <= 1e-9 {
            return Err(bad(cmd, "consecutive points cannot coincide"));
        }
    }
    if args.closed && points[0].dist(*points.last().ok_or_else(|| bad(cmd, "missing last point"))?) <= 1e-9 {
        return Err(bad(cmd, "do not repeat the first point in a closed polyline"));
    }
    let line = super::find_command("sketch.line").ok_or_else(|| bad(cmd, "sketch.line is not available"))?;
    line.run_nested(s, p)
}

fn root(parent: &mut [usize], n: usize) -> usize {
    let mut i = n;
    while parent[i] != i {
        i = parent[i];
    }
    let r = i;
    let mut i = n;
    while parent[i] != i {
        let next = parent[i];
        parent[i] = r;
        i = next;
    }
    r
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let a = root(parent, a);
    let b = root(parent, b);
    if a != b {
        parent[b] = a;
    }
}

/// Join is a topological/constraint operation: no sampling and no change of curve type.
/// Branches and disconnected selections are errors, never partially joined results.
fn join(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.join";
    let args: JoinParams = parse(cmd, p)?;
    let tol = finite(cmd, "tolerance", args.tolerance)?;
    if !(0.0..=0.01).contains(&tol) {
        return Err(bad(cmd, "tolerance must be between 0 and 0.01 mm"));
    }
    if !(2..=64).contains(&args.curves.len()) {
        return Err(bad(cmd, "join requires 2 to 64 curves"));
    }
    if args.curves.iter().collect::<BTreeSet<_>>().len() != args.curves.len() {
        return Err(bad(cmd, "duplicate curve ids"));
    }
    let ((ordered, backwards, closed, constraints), info) = edit(s, p, cmd, true, |sk, _| {
        let mut ids = Vec::with_capacity(args.curves.len());
        let mut ends = Vec::with_capacity(args.curves.len());
        for name in &args.curves {
            let ci = sk.curve_index(name).ok_or_else(|| bad(cmd, format!("unknown curve '{name}'")))?;
            let curve = &sk.curves[ci];
            if curve.construction || curve.centerline || curve.infinite || curve.link.is_some() {
                return Err(bad(cmd, format!("curve '{name}' is not an editable profile curve")));
            }
            let (a, b) = curve.kind.ends().ok_or_else(|| bad(cmd, format!("curve '{name}' is closed or has no two ends")))?;
            let pa = sk.point(a).ok_or_else(|| bad(cmd, "missing curve start"))?;
            let pb = sk.point(b).ok_or_else(|| bad(cmd, "missing curve end"))?;
            if pa.dist(pb) <= 1e-9 {
                return Err(bad(cmd, format!("curve '{name}' has coincident endpoints")));
            }
            ids.push(ci);
            ends.push(((a, pa), (b, pb)));
        }
        let n = ends.len();
        let mut parent: Vec<usize> = (0..2 * n).collect();
        for i in 0..2 * n {
            for j in (i + 1)..2 * n {
                if i / 2 != j / 2 {
                    let pi = if i % 2 == 0 { ends[i / 2].0.1 } else { ends[i / 2].1.1 };
                    let pj = if j % 2 == 0 { ends[j / 2].0.1 } else { ends[j / 2].1.1 };
                    if pi.dist(pj) <= tol {
                        union(&mut parent, i, j);
                    }
                }
            }
        }
        let nodes: Vec<usize> = (0..2 * n).map(|i| root(&mut parent, i)).collect();
        let mut degree = vec![0_usize; 2 * n];
        for &node in &nodes {
            degree[node] += 1;
            if degree[node] > 2 {
                return Err(bad(cmd, "branch junction: more than two selected ends coincide"));
            }
        }
        let open: Vec<usize> = (0..2 * n).filter(|&i| degree[nodes[i]] == 1).map(|i| nodes[i]).collect();
        if !(open.is_empty() || open.len() == 2) {
            return Err(bad(cmd, "curves do not form one open chain or one closed loop"));
        }
        let closed = open.is_empty();
        let start = open.first().copied().unwrap_or(nodes[0]);
        let mut at = start;
        let mut used = vec![false; n];
        let mut indices = Vec::with_capacity(n);
        let mut backwards = Vec::with_capacity(n);
        let mut joins: Vec<(usize, usize)> = Vec::new();
        let mut prev_end = None;
        let mut first_start = None;
        for step in 0..n {
            let options: Vec<usize> = (0..n).filter(|&i| !used[i] && (nodes[2 * i] == at || nodes[2 * i + 1] == at)).collect();
            if options.is_empty() || (options.len() > 1 && step > 0) {
                return Err(bad(cmd, "selection is disconnected or branches at a junction"));
            }
            let i = options[0];
            let forward = nodes[2 * i] == at;
            let begin = if forward { ends[i].0.0 } else { ends[i].1.0 };
            let end = if forward { ends[i].1.0 } else { ends[i].0.0 };
            if let Some(prev) = prev_end {
                joins.push((prev, begin));
            } else {
                first_start = Some(begin);
            }
            prev_end = Some(end);
            indices.push(ids[i]);
            backwards.push(!forward);
            used[i] = true;
            at = if forward { nodes[2 * i + 1] } else { nodes[2 * i] };
        }
        if closed {
            if at != start {
                return Err(bad(cmd, "the selected loop is not closed"));
            }
            if let (Some(a), Some(b)) = (prev_end, first_start) {
                joins.push((a, b));
            }
        } else if open.get(1).copied() != Some(at) {
            return Err(bad(cmd, "the selected chain has disconnected ends"));
        }
        let mut coincident: Vec<usize> = (0..sk.points.len()).collect();
        for c in &sk.constraints {
            if let ConstraintKind::Coincident { p, q } = c.kind {
                if p < coincident.len() && q < coincident.len() {
                    union(&mut coincident, p, q);
                }
            }
        }
        let mut constraints = Vec::new();
        for (a, b) in joins {
            if root(&mut coincident, a) != root(&mut coincident, b) {
                constraints.push(add_c(sk, ConstraintKind::Coincident { p: a, q: b })?);
                union(&mut coincident, a, b);
            }
        }
        let ordered: Vec<String> = indices.iter().map(|&ci| sk.curves[ci].id.clone()).collect();
        Ok((ordered, backwards, closed, constraints))
    })?;
    Ok(json!({"curves": ordered, "reversed": backwards, "closed": closed, "constraints": constraints, "sketch": info}))
}

/// Radius is constant while z changes linearly along a unit axis. Kept as a 3D
/// fit wire (same representation as sketch.spline3d); not an analytic helix.
fn helix(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.helix";
    let args: HelixParams = parse(cmd, p)?;
    let radius = finite(cmd, "radius", args.radius)?;
    let height = finite(cmd, "height", args.height)?;
    let phase = finite(cmd, "phase", args.phase)?;
    if radius <= 0.0 || height.abs() <= 1e-9 {
        return Err(bad(cmd, "radius must be positive and height nonzero"));
    }
    let count = samples(cmd, args.turns, args.samples_per_turn)?;
    let axis = args.axis.0.normalized().ok_or_else(|| bad(cmd, "axis must be a nonzero vector"))?;
    let guide = if axis.dot(Vec3::X).abs() > 0.9 { Vec3::Y } else { Vec3::X };
    let u = (guide - axis * axis.dot(guide)).normalized().ok_or_else(|| bad(cmd, "invalid helix axis"))?;
    let v = axis.cross(u);
    let origin = args.center.0;
    // Delegate both 3D-sketch validation and world-to-component conversion to the
    // existing spline3d command. It already owns 3D wire persistence and undo.
    let points: Vec<Value> = (0..=count)
        .map(|i| {
            let t = i as f64 / count as f64;
            let angle = TAU * args.turns * t + phase.to_radians();
            let world = origin + u * (radius * angle.cos()) + v * (radius * angle.sin()) + axis * (height * t);
            json!([world.x, world.y, world.z])
        })
        .collect();
    let mut params = json!({"points": points});
    if let Some(sketch) = p.get("sketch") {
        params["sketch"] = sketch.clone();
    }
    let existing = super::find_command("sketch.spline3d").ok_or_else(|| bad(cmd, "sketch.spline3d is not available"))?;
    let reply = existing.run_nested(s, &params)?;
    let curve = reply["curves"].get(0).and_then(Value::as_str).ok_or_else(|| bad(cmd, "sketch.spline3d returned no curve"))?;
    Ok(json!({"curve": curve, "samples": count + 1, "approximation": "sampled_3d_fit_wire", "sketch": reply["sketch"]}))
}

/// Archimedean spiral between two radii, interpolated as an existing cubic fit spline.
fn spiral(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.spiral";
    let args: SpiralParams = parse(cmd, p)?;
    let r0 = finite(cmd, "start_radius", args.start_radius)?;
    let r1 = finite(cmd, "end_radius", args.end_radius)?;
    let phase = finite(cmd, "phase", args.phase)?;
    if r0 < 0.0 || r1 < 0.0 || (r1 - r0).abs() <= 1e-9 {
        return Err(bad(cmd, "radii must be nonnegative and differ by more than 1e-9 mm"));
    }
    let count = samples(cmd, args.turns, args.samples_per_turn)?;
    let center = args.center.0;
    let points: Vec<Vec2> = (0..=count)
        .map(|i| {
            let t = i as f64 / count as f64;
            let r = r0 + (r1 - r0) * t;
            let angle = TAU * args.turns * t + phase.to_radians();
            Vec2::new(center.x + r * angle.cos(), center.y + r * angle.sin())
        })
        .collect();
    let (curve, info) = edit(s, p, cmd, false, |sk, _| {
        let ids = points.iter().map(|&p| sk.add_point(p, None)).collect::<std::result::Result<Vec<_>, _>>()?;
        let ci = sk.add_curve(CurveKind::Spline { pts: ids, control: false, degree: 3, periodic: false }, None)?;
        Ok(sk.curves[ci].id.clone())
    })?;
    Ok(json!({"curve": curve, "samples": points.len(), "approximation": "sampled_planar_fit_spline", "sketch": info}))
}
