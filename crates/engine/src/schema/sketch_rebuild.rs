//! Typed input/output contracts for curve rebuilding and geometric end continuity.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{SketchRef, present};

/// Rebuild one non-periodic sketch spline as a new editable control-point spline. The source
/// and its constraints are retained; construction geometry is the default to avoid making a
/// second, overlapping profile boundary.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SplineRebuildParams {
    #[serde(deserialize_with = "curve_id")]
    #[schemars(length(min = 1, max = 256))]
    pub curve: String,
    /// Sketch id or name; omitted means the active sketch.
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "SketchRef")]
    pub sketch: Option<SketchRef>,
    #[serde(default = "default_degree")]
    #[schemars(range(min = 1, max = 7))]
    pub degree: usize,
    /// Must exceed degree; the aggregate work budget may impose a smaller useful combination.
    #[serde(default = "default_controls")]
    #[schemars(range(min = 2, max = 64))]
    pub control_points: usize,
    #[serde(default = "default_fit_samples")]
    #[schemars(range(min = 2, max = 2049))]
    pub fit_samples: usize,
    #[serde(default = "default_check_samples")]
    #[schemars(range(min = 2, max = 1025))]
    pub check_samples: usize,
    #[serde(default = "yes")]
    pub construction: bool,
}

fn curve_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let value = String::deserialize(d)?;
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(serde::de::Error::custom("a curve id must contain 1..256 bytes without control characters"));
    }
    Ok(value)
}

fn default_degree() -> usize {
    3
}
fn default_controls() -> usize {
    8
}
fn default_fit_samples() -> usize {
    129
}
fn default_check_samples() -> usize {
    257
}
fn yes() -> bool {
    true
}
fn default_gap() -> f64 {
    1e-6
}
fn default_angle() -> f64 {
    0.01
}
fn default_curvature() -> f64 {
    1e-6
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SampledDeviationResult {
    pub source_to_rebuilt_mm: f64,
    pub rebuilt_to_source_mm: f64,
    pub samples_per_curve: usize,
    /// Maximum of the two directed sampled estimates; not a certified Hausdorff bound.
    pub max_mm: f64,
    /// Mean of all sampled point-to-polyline distances, both directions.
    pub mean_mm: f64,
    /// Root-mean-square of all sampled point-to-polyline distances, both directions.
    pub rms_mm: f64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SplineRebuildResult {
    pub source: String,
    pub curve: String,
    pub degree: usize,
    pub control_points: usize,
    pub construction: bool,
    /// Difference at corresponding normalized parameters, not a closest-point distance.
    pub parameter_rms_mm: f64,
    pub deviation: SampledDeviationResult,
    /// Control points on the source curve before the rebuild.
    pub source_control_points: usize,
    /// Control points on the rebuilt curve.
    pub rebuilt_control_points: usize,
    /// Max |curvature(rebuilt) - curvature(source)| in 1/mm at matching samples; null when no sample had a defined curvature.
    pub max_curvature_change_per_mm: Option<f64>,
    /// Number of matching sample pairs where both curvatures were defined.
    pub curvature_samples: usize,
    pub estimate_note: String,
    /// The ordinary sketch solve report.
    pub sketch: Value,
    /// Added by the shared command path, with the new points/curve and no new constraints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

/// Compare the authored end of `before` with the authored start of `after` in one sketch.
/// Reversal is explicit and affects measurement only; neither curve is edited or welded.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct CurveContinuityParams {
    #[serde(deserialize_with = "curve_id")]
    #[schemars(length(min = 1, max = 256))]
    pub before: String,
    #[serde(deserialize_with = "curve_id")]
    #[schemars(length(min = 1, max = 256))]
    pub after: String,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "SketchRef")]
    pub sketch: Option<SketchRef>,
    #[serde(default)]
    pub reverse_before: bool,
    #[serde(default)]
    pub reverse_after: bool,
    #[serde(default = "default_gap")]
    #[schemars(range(min = 0))]
    pub gap_tolerance_mm: f64,
    #[serde(default = "default_angle")]
    #[schemars(range(min = 0, max = 180))]
    pub angle_tolerance_deg: f64,
    #[serde(default = "default_curvature")]
    #[schemars(range(min = 0))]
    pub curvature_tolerance_per_mm: f64,
    #[serde(default = "default_curvature")]
    #[schemars(range(min = 0))]
    pub curvature_change_tolerance_per_mm2: f64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CurveContinuityTolerances {
    pub gap_mm: f64,
    pub tangent_angle_deg: f64,
    pub curvature_per_mm: f64,
    pub curvature_change_per_mm2: f64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub enum CurveContinuityOrder {
    G0,
    G1,
    G2,
    G3,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CurveContinuityResult {
    pub before: String,
    pub after: String,
    pub reverse_before: bool,
    pub reverse_after: bool,
    pub gap_mm: f64,
    pub tangent_angle_deg: f64,
    pub curvature_jump_per_mm: f64,
    pub curvature_change_jump_per_mm2: f64,
    pub measured_order: Option<CurveContinuityOrder>,
    pub tolerances: CurveContinuityTolerances,
    pub scope_note: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{input_schema_for, tests::assert_keys};

    #[test]
    fn schema_keys_match_both_command_contracts() {
        assert_keys(
            input_schema_for::<SplineRebuildParams>(),
            &["check_samples", "construction", "control_points", "curve", "degree", "fit_samples", "sketch"],
        );
        assert_keys(
            input_schema_for::<CurveContinuityParams>(),
            &[
                "after",
                "angle_tolerance_deg",
                "before",
                "curvature_change_tolerance_per_mm2",
                "curvature_tolerance_per_mm",
                "gap_tolerance_mm",
                "reverse_after",
                "reverse_before",
                "sketch",
            ],
        );
    }

    #[test]
    fn command_contracts_describe_success_and_strict_unknown_keys_are_atomic() {
        use crate::schema::tests::conforms;
        use crate::schema::{ParamCheck, output_schema_for};
        use serde_json::json;

        let mut s = crate::Session::default();
        s.execute("sketch.create", &json!({"plane":"XY", "name":"Curves"})).unwrap();
        let source =
            s.execute("sketch.spline.control_point", &json!({"points":[[0, 0], [10, 0]]})).unwrap()["curves"][0].as_str().unwrap().to_string();
        let after = s.execute("sketch.spline.fit_point", &json!({"points":[[10, 0], [20, 0]]})).unwrap()["curves"][0].as_str().unwrap().to_string();
        s.param_check = ParamCheck::Strict;
        let saved = s.doc.to_json();
        assert!(s.execute("sketch.spline.rebuild", &json!({"curve":source,"unknown":true})).is_err());
        assert_eq!(s.doc.to_json(), saved);
        let report =
            s.execute("sketch.spline.rebuild", &json!({"curve":source,"degree":1,"control_points":2,"fit_samples":17,"check_samples":17})).unwrap();
        let schema = output_schema_for::<SplineRebuildResult>();
        assert!(conforms(schema.as_value(), schema.as_value(), &report), "{report}");
        let report = s.execute("inspect.curve_continuity", &json!({"before":source,"after":after})).unwrap();
        let schema = output_schema_for::<CurveContinuityResult>();
        assert!(conforms(schema.as_value(), schema.as_value(), &report), "{report}");
        assert_eq!(report["measured_order"], "G3");
    }
}
