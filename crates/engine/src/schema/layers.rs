//! Contracts and result types for the layer commands in `cmd/layers.rs`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `layer.new`: a new layer, listed after the others.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct NewParams {
    /// 1 to 128 characters; default "Layer N".
    pub name: Option<String>,
    /// `#rrggbb`; default: the next colour of the palette.
    pub colour: Option<String>,
}

/// `layer.rename`.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct RenameParams {
    /// The layer's id (not the default layer, 0).
    pub layer: u64,
    /// 1 to 128 characters.
    pub name: String,
}

/// `layer.set_visible`: the layer's eye. Its bodies follow it.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct VisibleParams {
    /// The layer's id (the default layer has no eye yet).
    pub layer: u64,
    pub visible: bool,
}

/// `layer.set_locked`: the layer's lock. Its bodies follow it.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct LockedParams {
    /// The layer's id (the default layer has no lock yet).
    pub layer: u64,
    pub locked: bool,
}

/// `layer.set_current`: the layer new work is put on (0 = the default layer).
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct CurrentParams {
    pub layer: u64,
}

/// `layer.assign`: moves bodies to a layer (0 = the default layer), which sets their eye and lock.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct AssignParams {
    /// Body names.
    pub bodies: Vec<String>,
    pub layer: u64,
}

/// What the layer commands return; each sets the fields it changed.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LayerReply {
    pub layer: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bodies: Option<u64>,
}
