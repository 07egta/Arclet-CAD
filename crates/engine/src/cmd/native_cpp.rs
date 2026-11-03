//! Optional native C++ bridge commands (desktop only; off by default, see
//! docs/native-cpp-bridge.md). They read their inputs and change nothing in the design.
//!
//! - `sketch.offset_native` and `sketch.boolean_native` run Clipper2 on 2D rings (feature `clipper2`).
//! - `mesh.boolean` runs Manifold on the display meshes of two bodies (feature `manifold`). The
//!   result is a TRIANGLE MESH, not NURBS or B-rep; it never replaces the exact body.
//!
//! Without the feature (and always in wasm) each command returns an error saying the backend is
//! not built in. None of these is verified by a local build; CI decides.

use serde_json::Value;

use super::CommandSpec;
use crate::params::bad;
use crate::schema::native_cpp::{BooleanNativeParams, MeshBooleanParams, MeshBooleanReply, OffsetNativeParams, PolygonReply};
use crate::{EngineError, Result, Session};

const OFFSET_CONTRACT: crate::schema::TypedContract<OffsetNativeParams> = crate::schema::TypedContract::new();
const BOOLEAN_CONTRACT: crate::schema::TypedContract<BooleanNativeParams> = crate::schema::TypedContract::new();
const MESH_CONTRACT: crate::schema::TypedContract<MeshBooleanParams> = crate::schema::TypedContract::new();

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("mesh.boolean", "Mesh Boolean (native)", crate::checked!("mesh.boolean", mesh_boolean))
        .icon("inspect")
        .noundo()
        .contract(&MESH_CONTRACT)
        .output(crate::schema::output_schema_for::<MeshBooleanReply>)
        .params("a: body name, b: body name, op: union|difference|intersection, tolerance_mm?: tessellation tolerance, include_mesh?: bool (default true). Native Manifold boolean of the two bodies' display meshes. The result is a TRIANGLE MESH (vertices, triangles, counts), NOT NURBS or B-rep; no body is changed or replaced. Needs a build with feature `manifold`, otherwise an error → backend, kind, vertex_count, triangle_count, vertices?, triangles?"),
    CommandSpec::new("sketch.boolean_native", "Polygon Boolean (native)", crate::checked!("sketch.boolean_native", boolean_native))
        .icon("inspect")
        .noundo()
        .doc_only()
        .contract(&BOOLEAN_CONTRACT)
        .output(crate::schema::output_schema_for::<PolygonReply>)
        .params("subject: [[x,y]…] mm, clip: [[x,y]…] mm, op: union|intersection|difference|xor. Native Clipper2 boolean of two closed rings; returns rings without hole hierarchy and changes nothing. Needs a build with feature `clipper2`, otherwise an error → backend, rings, ring_count"),
    CommandSpec::new("sketch.offset_native", "Polygon Offset (native)", crate::checked!("sketch.offset_native", offset_native))
        .icon("inspect")
        .noundo()
        .doc_only()
        .contract(&OFFSET_CONTRACT)
        .output(crate::schema::output_schema_for::<PolygonReply>)
        .params("ring: [[x,y]…] mm (closed, at least 3 points), distance: mm (positive grows, negative shrinks). Native Clipper2 offset; returns rings and changes nothing. Needs a build with feature `clipper2`, otherwise an error → backend, rings, ring_count"),
];

fn parse<T: serde::de::DeserializeOwned>(p: &Value, cmd: &str) -> Result<T> {
    serde_json::from_value(p.clone()).map_err(|e| bad(cmd, e.to_string()))
}

fn check_ring(ring: &[[f64; 2]], what: &str, cmd: &str) -> Result<()> {
    if ring.len() < 3 {
        return Err(bad(cmd, format!("`{what}` needs at least 3 points")));
    }
    if ring.iter().any(|p| !p[0].is_finite() || !p[1].is_finite()) {
        return Err(bad(cmd, format!("`{what}` has a non-finite coordinate")));
    }
    Ok(())
}

#[cfg_attr(all(feature = "clipper2", feature = "manifold", not(target_arch = "wasm32")), allow(dead_code))]
fn unavailable(cmd: &str, feature: &str) -> EngineError {
    EngineError::Other(format!("`{cmd}` needs the native {feature} backend, which is not built in (desktop builds with feature `{feature}` and its source path only)"))
}

fn offset_native(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.offset_native";
    let req: OffsetNativeParams = parse(p, cmd)?;
    check_ring(&req.ring, "ring", cmd)?;
    if !req.distance.is_finite() {
        return Err(bad(cmd, "`distance` must be a finite number"));
    }
    let rings = native_offset(&req.ring, req.distance, cmd)?;
    polygon_reply(rings)
}

fn boolean_native(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.boolean_native";
    let req: BooleanNativeParams = parse(p, cmd)?;
    check_ring(&req.subject, "subject", cmd)?;
    check_ring(&req.clip, "clip", cmd)?;
    let rings = native_boolean(&req.subject, &req.clip, req.op, cmd)?;
    polygon_reply(rings)
}

fn polygon_reply(rings: Vec<Vec<[f64; 2]>>) -> Result<Value> {
    let reply = PolygonReply { backend: backend_polygon().to_string(), ring_count: rings.len() as u64, rings };
    serde_json::to_value(reply).map_err(|e| EngineError::Other(e.to_string()))
}

#[cfg(all(feature = "clipper2", not(target_arch = "wasm32")))]
fn backend_polygon() -> &'static str {
    solidcraft_polyclip_cpp::backend()
}
#[cfg(not(all(feature = "clipper2", not(target_arch = "wasm32"))))]
fn backend_polygon() -> &'static str {
    "unavailable"
}

#[cfg(all(feature = "clipper2", not(target_arch = "wasm32")))]
fn native_offset(ring: &[[f64; 2]], distance: f64, cmd: &str) -> Result<Vec<Vec<[f64; 2]>>> {
    let _ = cmd;
    solidcraft_polyclip_cpp::offset_polygon(ring, distance).map_err(|e| EngineError::Other(format!("sketch.offset_native: {e}")))
}
#[cfg(not(all(feature = "clipper2", not(target_arch = "wasm32"))))]
fn native_offset(ring: &[[f64; 2]], distance: f64, cmd: &str) -> Result<Vec<Vec<[f64; 2]>>> {
    let _ = (ring, distance);
    Err(unavailable(cmd, "clipper2"))
}

#[cfg(all(feature = "clipper2", not(target_arch = "wasm32")))]
fn native_boolean(a: &[[f64; 2]], b: &[[f64; 2]], op: crate::schema::native_cpp::PolygonOp, cmd: &str) -> Result<Vec<Vec<[f64; 2]>>> {
    use crate::schema::native_cpp::PolygonOp;
    use solidcraft_polyclip_cpp::BooleanOp;
    let _ = cmd;
    let op = match op {
        PolygonOp::Union => BooleanOp::Union,
        PolygonOp::Intersection => BooleanOp::Intersection,
        PolygonOp::Difference => BooleanOp::Difference,
        PolygonOp::Xor => BooleanOp::Xor,
    };
    solidcraft_polyclip_cpp::boolean(a, b, op).map_err(|e| EngineError::Other(format!("sketch.boolean_native: {e}")))
}
#[cfg(not(all(feature = "clipper2", not(target_arch = "wasm32"))))]
fn native_boolean(a: &[[f64; 2]], b: &[[f64; 2]], op: crate::schema::native_cpp::PolygonOp, cmd: &str) -> Result<Vec<Vec<[f64; 2]>>> {
    let _ = (a, b, op);
    Err(unavailable(cmd, "clipper2"))
}

fn mesh_boolean(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "mesh.boolean";
    let req: MeshBooleanParams = parse(p, cmd)?;
    if req.a == req.b {
        return Err(bad(cmd, "`a` and `b` must be two different bodies"));
    }
    if let Some(t) = req.tolerance_mm
        && !(t.is_finite() && t > 0.0 && t <= 1e6)
    {
        return Err(bad(cmd, "`tolerance_mm` must be a positive finite number"));
    }
    {
        let state = s.model.state();
        for name in [&req.a, &req.b] {
            if state.body(name).is_none() {
                return Err(bad(cmd, format!("no body `{name}`")));
            }
        }
    }
    let reply = native_mesh_boolean(s, &req, cmd)?;
    serde_json::to_value(reply).map_err(|e| EngineError::Other(e.to_string()))
}

#[cfg(not(all(feature = "manifold", not(target_arch = "wasm32"))))]
fn native_mesh_boolean(s: &Session, req: &MeshBooleanParams, cmd: &str) -> Result<MeshBooleanReply> {
    let _ = (s, req);
    Err(unavailable(cmd, "manifold"))
}

#[cfg(all(feature = "manifold", not(target_arch = "wasm32")))]
fn native_mesh_boolean(s: &Session, req: &MeshBooleanParams, cmd: &str) -> Result<MeshBooleanReply> {
    use crate::schema::native_cpp::MeshOp;
    use solidcraft_manifold_cpp::{BooleanOp, Mesh};
    let a = welded_mesh(s, &req.a, req.tolerance_mm, cmd)?;
    let b = welded_mesh(s, &req.b, req.tolerance_mm, cmd)?;
    let op = match req.op {
        MeshOp::Union => BooleanOp::Union,
        MeshOp::Difference => BooleanOp::Difference,
        MeshOp::Intersection => BooleanOp::Intersection,
    };
    let out: Mesh = solidcraft_manifold_cpp::boolean(&a, &b, op).map_err(|e| EngineError::Other(format!("{cmd}: {e}")))?;
    let (vertex_count, triangle_count) = (out.vertices.len() as u64, out.triangles.len() as u64);
    let include = req.include_mesh.unwrap_or(true);
    Ok(MeshBooleanReply {
        backend: solidcraft_manifold_cpp::backend().to_string(),
        kind: "triangle_mesh".to_string(),
        vertex_count,
        triangle_count,
        vertices: include.then_some(out.vertices),
        triangles: include.then_some(out.triangles),
    })
}

/// The body's display mesh in world placement, with vertices that coincide (the mesher repeats
/// them along face seams) merged, as Manifold needs a closed, indexed mesh.
#[cfg(all(feature = "manifold", not(target_arch = "wasm32")))]
fn welded_mesh(s: &Session, name: &str, tolerance: Option<f64>, cmd: &str) -> Result<solidcraft_manifold_cpp::Mesh> {
    use std::collections::HashMap;
    let state = s.model.state();
    let body = state.body(name).ok_or_else(|| bad(cmd, format!("no body `{name}`")))?;
    let offset = *s.doc.model_body_offset(body).unwrap_or(&solidcraft_doc::IDENTITY);
    let placed = solidcraft_kernel::transform_matrix(&body.body, offset)?;
    let size = placed.size();
    let tol = tolerance.unwrap_or_else(|| (size * 1e-3).max(1e-3));
    let mesh = placed.display_mesh(tol)?;
    let quantum = (size * 1e-7).max(1e-9);
    let mut index: HashMap<[i64; 3], u32> = HashMap::new();
    let mut vertices: Vec<[f64; 3]> = Vec::new();
    let mut remap: Vec<u32> = Vec::with_capacity(mesh.positions.len());
    for v in &mesh.positions {
        let key = [(v.x / quantum).round() as i64, (v.y / quantum).round() as i64, (v.z / quantum).round() as i64];
        let id = match index.get(&key) {
            Some(id) => *id,
            None => {
                let id = u32::try_from(vertices.len()).map_err(|_| EngineError::Other(format!("{cmd}: mesh of `{name}` is too large")))?;
                vertices.push([v.x, v.y, v.z]);
                index.insert(key, id);
                id
            }
        };
        remap.push(id);
    }
    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(mesh.triangles.len());
    for t in &mesh.triangles {
        let mut w = [0u32; 3];
        for (slot, i) in w.iter_mut().zip(t.iter()) {
            *slot = *remap.get(*i as usize).ok_or_else(|| EngineError::Other(format!("{cmd}: mesh of `{name}` has an invalid index")))?;
        }
        if w[0] != w[1] && w[1] != w[2] && w[0] != w[2] {
            triangles.push(w);
        }
    }
    Ok(solidcraft_manifold_cpp::Mesh { vertices, triangles })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bad_params_are_rejected_before_any_backend_is_asked() {
        let mut s = Session::default();
        let square = json!([[0, 0], [10, 0], [10, 10], [0, 10]]);
        for (cmd, params) in [
            ("sketch.offset_native", json!({"ring": [[0, 0], [1, 1]], "distance": 1})),
            ("sketch.offset_native", json!({"ring": square, "distance": "x"})),
            ("sketch.offset_native", json!({"ring": square, "distance": 1, "extra": 1})),
            ("sketch.boolean_native", json!({"subject": square, "clip": [[0, 0]], "op": "union"})),
            ("sketch.boolean_native", json!({"subject": square, "clip": square, "op": "nope"})),
            ("mesh.boolean", json!({"a": "X", "b": "X", "op": "union"})),
            ("mesh.boolean", json!({"a": "X", "b": "Y", "op": "union", "tolerance_mm": -1})),
            ("mesh.boolean", json!({"a": "X", "b": "Y", "op": "union"})),
        ] {
            assert!(s.execute(cmd, &params).is_err(), "{cmd} {params}");
        }
    }

    #[cfg(not(feature = "clipper2"))]
    #[test]
    fn polygon_commands_report_an_unavailable_backend_as_an_error() {
        let mut s = Session::default();
        let square = json!([[0, 0], [10, 0], [10, 10], [0, 10]]);
        let e = s.execute("sketch.offset_native", &json!({"ring": square, "distance": 1})).unwrap_err();
        assert!(e.to_string().contains("not built in"), "{e}");
        let e = s.execute("sketch.boolean_native", &json!({"subject": square, "clip": square, "op": "union"})).unwrap_err();
        assert!(e.to_string().contains("not built in"), "{e}");
    }

    #[cfg(not(feature = "manifold"))]
    #[test]
    fn mesh_boolean_reports_an_unavailable_backend_as_an_error() {
        let mut s = Session::default();
        s.execute("solid.box", &json!({"length": 10, "width": 10, "height": 10})).unwrap();
        s.execute("solid.box", &json!({"length": 10, "width": 10, "height": 10})).unwrap();
        let names: Vec<String> = s.model.state().bodies.iter().map(|b| b.name.clone()).collect();
        if let [a, b, ..] = names.as_slice() {
            let e = s.execute("mesh.boolean", &json!({"a": a, "b": b, "op": "union"})).unwrap_err();
            assert!(e.to_string().contains("not built in"), "{e}");
        }
    }
}
