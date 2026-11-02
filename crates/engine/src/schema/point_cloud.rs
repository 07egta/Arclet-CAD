//! Input/output schemas for bounded, read-only point-cloud reconstruction diagnostics.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::types::{IdOrName, present};

/// Analyze a cloud with a bounded sample. All lengths are in millimetres.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct AnalyzeParams {
    /// Source point cloud name or id.
    pub cloud: IdOrName,
    /// plane, circle, sphere, cylinder, nearest, outliers, or align.
    pub mode: String,
    /// Bounded sample count (1..100000, default 20000).
    #[serde(default, deserialize_with = "present")]
    pub max_points: Option<u32>,
    /// Absolute residual tolerance in millimetres, default 0.1.
    #[serde(default, deserialize_with = "present")]
    pub tolerance_mm: Option<f64>,
    /// Query world point for nearest.
    #[serde(default, deserialize_with = "present")]
    pub point: Option<[f64; 3]>,
    /// Fixed target point cloud for align; the source is moved toward the target.
    #[serde(default, deserialize_with = "present")]
    pub target: Option<IdOrName>,
}

/// Analysis metadata and shape-dependent numeric result.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalyzeResult {
    pub cloud: u64,
    pub mode: String,
    pub sampled: usize,
    pub source_points: usize,
    pub stride: usize,
    /// Shape-dependent estimate. This does not modify CAD history.
    pub result: Value,
    /// Parameter-check issues in warn mode.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::AnalyzeParams;
    use crate::schema::input_schema_for;
    use crate::schema::tests::assert_keys;

    #[test]
    fn analysis_schema_keys_match_handler() {
        assert_keys(input_schema_for::<AnalyzeParams>(), &["cloud", "max_points", "mode", "point", "target", "tolerance_mm"]);
    }
}
