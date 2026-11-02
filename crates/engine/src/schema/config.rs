//! Input contracts and output shapes for configuration sweep and bulk validation.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::present;

/// Validate all configuration rows on independent scratch models.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ValidateParams {
    /// Include evaluated parameters, rule traces, and appearance status for successful rows.
    #[serde(default, deserialize_with = "present")]
    pub details: Option<bool>,
}

/// A row that was checked without changing the source design.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidatedRow {
    pub name: String,
    pub valid: bool,
    /// The failed formula, unit conversion, rule, sketch or geometry check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Evaluated parameters, only for successful rows with details requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
    /// Ordered configuration rules, only for successful rows with details requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<Value>,
    /// Resolved material and appearance status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<Value>,
}

/// Read-only response to `config.validate`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidateResult {
    pub valid: bool,
    pub checked: usize,
    pub failed: usize,
    pub rows: Vec<ValidatedRow>,
    /// Parameter-check issues in warn mode.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

/// Cartesian parameter sweep, from a named or currently active configuration.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SweepParams {
    /// Name prefix for generated rows (default `Variant`).
    #[serde(default, deserialize_with = "present")]
    pub prefix: Option<String>,
    /// Optional existing row to inherit values from (default: active row).
    #[serde(default, deserialize_with = "present")]
    pub from: Option<String>,
    /// Column keys or parameter names, with candidate expression / material / suppression values.
    /// Each axis is checked against its column type by the command handler.
    pub axes: BTreeMap<String, Vec<Value>>,
    /// Validate and report combinations without saving the generated rows.
    #[serde(default, deserialize_with = "present")]
    pub preview: Option<bool>,
}

/// A row produced by the sweep, including its exact saved cell text.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SweptRow {
    pub name: String,
    pub values: BTreeMap<String, String>,
}

/// Response to `config.sweep`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SweepResult {
    pub preview: bool,
    pub count: usize,
    pub rows: Vec<SweptRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{SweepParams, ValidateParams};
    use crate::schema::input_schema_for;
    use crate::schema::tests::assert_keys;

    #[test]
    fn configuration_schema_keys_match_the_commands() {
        assert_keys(input_schema_for::<ValidateParams>(), &["details"]);
        assert_keys(input_schema_for::<SweepParams>(), &["axes", "from", "prefix", "preview"]);
    }
}
