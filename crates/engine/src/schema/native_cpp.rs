//! Contracts and result types for the native C++ bridge commands in `cmd/native_cpp.rs`
//! (`sketch.offset_native`, `sketch.boolean_native`, `mesh.boolean`). Desktop only, behind the
//! engine's `clipper2` and `manifold` features, which are off by default.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `sketch.offset_native`: offsets one closed polygon ring.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[serde(deny_unknown_fields)]
pub struct OffsetNativeParams {
    /// The closed ring as [x, y] points in millimetres (at least 3; the closing edge is implied).
    pub ring: Vec<[f64; 2]>,
    /// Offset distance in millimetres: positive grows the ring, negative shrinks it.
    pub distance: f64,
}

/// The boolean operation between two polygons.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolygonOp {
    Union,
    Intersection,
    Difference,
    Xor,
}

/// `sketch.boolean_native`: a boolean between two closed polygon rings.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[serde(deny_unknown_fields)]
pub struct BooleanNativeParams {
    /// First ring, [x, y] points in millimetres.
    pub subject: Vec<[f64; 2]>,
    /// Second ring, [x, y] points in millimetres.
    pub clip: Vec<[f64; 2]>,
    pub op: PolygonOp,
}

/// What the polygon commands return. The document is not changed.
#[derive(Debug, Serialize, JsonSchema)]
pub struct PolygonReply {
    /// The backend that did the work (`clipper2`).
    pub backend: String,
    /// The resulting rings, [x, y] in millimetres. They carry no parent/hole hierarchy.
    pub rings: Vec<Vec<[f64; 2]>>,
    pub ring_count: u64,
}

/// The boolean operation between two meshes.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MeshOp {
    Union,
    Difference,
    Intersection,
}

/// `mesh.boolean`: a boolean between the display meshes of two bodies. The result is a triangle
/// mesh, not NURBS or B-rep, and no body is changed.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[serde(deny_unknown_fields)]
pub struct MeshBooleanParams {
    /// Name of the first body.
    pub a: String,
    /// Name of the second body (subtracted from `a` for `difference`).
    pub b: String,
    pub op: MeshOp,
    /// Tessellation chord tolerance in millimetres (default: 1/1000 of each body's size, at least 0.001).
    pub tolerance_mm: Option<f64>,
    /// Return the vertices and triangles (default true); false returns only the counts.
    pub include_mesh: Option<bool>,
}

/// What `mesh.boolean` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct MeshBooleanReply {
    /// The backend that did the work (`manifold-cpp`).
    pub backend: String,
    /// Always `triangle_mesh`: the result is not NURBS or B-rep and replaces no body.
    pub kind: String,
    pub vertex_count: u64,
    pub triangle_count: u64,
    /// Vertices [x, y, z] in millimetres, in world placement (when `include_mesh` is not false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertices: Option<Vec<[f64; 3]>>,
    /// Triangles as vertex index triples (when `include_mesh` is not false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triangles: Option<Vec<[u32; 3]>>,
}
