//! Strict parameter and result schemas for the new sketch modeling commands.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{Point2, Point3, SketchRef, present};

fn default_samples() -> usize { 48 }
fn default_tolerance() -> f64 { 1e-6 }

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct PolylineParams {
    /// Sketch-space 2D points, joined by exact segments.
    pub points: Vec<Point2>,
    #[serde(default)]
    pub closed: bool,
    #[serde(default)]
    pub construction: bool,
    #[serde(default)]
    pub infer: bool,
    #[serde(default)]
    pub connect: bool,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "SketchRef")]
    pub sketch: Option<SketchRef>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PolylineResult {
    pub curves: Vec<String>,
    pub constraints: Vec<String>,
    pub sketch: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct JoinParams {
    /// Curve ids, in any order.
    pub curves: Vec<String>,
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "SketchRef")]
    pub sketch: Option<SketchRef>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct JoinResult {
    pub curves: Vec<String>,
    pub reversed: Vec<bool>,
    pub closed: bool,
    pub constraints: Vec<String>,
    pub sketch: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct HelixParams {
    /// Axis origin in world coordinates.
    pub center: Point3,
    /// Axis vector; its length is ignored.
    pub axis: Point3,
    pub radius: f64,
    pub turns: f64,
    pub height: f64,
    #[serde(default)]
    pub phase: f64,
    #[serde(default = "default_samples")]
    pub samples_per_turn: usize,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "SketchRef")]
    pub sketch: Option<SketchRef>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct SpiralParams {
    /// Center in the target sketch's 2D coordinates.
    pub center: Point2,
    pub start_radius: f64,
    pub end_radius: f64,
    pub turns: f64,
    #[serde(default)]
    pub phase: f64,
    #[serde(default = "default_samples")]
    pub samples_per_turn: usize,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "SketchRef")]
    pub sketch: Option<SketchRef>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PathResult {
    pub curve: String,
    pub samples: usize,
    pub approximation: String,
    pub sketch: Value,
}
