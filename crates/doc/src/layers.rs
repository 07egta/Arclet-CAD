//! Layers: named groups of bodies with an eye (visible) and a lock, and one current layer.
//!
//! Layer 0 (`DEFAULT`) is implicit: it always exists, holds every body no layer claims, and
//! cannot be hidden or locked. The other layers are listed in `items`; `of` maps a body name to
//! its layer. The layer's eye and lock are applied to its bodies by the layer commands, through
//! the session's `visibility.hidden_bodies` and `visibility.locked_bodies` (not stored here), so
//! hiding and locking reuse what the browser already enforces. Saved with the design and undone
//! with it, as the design's other items are.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The implicit layer that holds every body no other layer claims. It is always visible and
/// unlocked (its eye and lock are not offered).
pub const DEFAULT: u64 = 0;

/// One layer. `colour` is `#rrggbb`; `visible` is its eye and `locked` its lock.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub colour: String,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
}

fn yes() -> bool {
    true
}

/// The design's layers: the listed ones, the current one (`DEFAULT` when none), and the layer
/// each body is on (bodies not listed are on `DEFAULT`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layers {
    pub items: Vec<Layer>,
    pub current: u64,
    pub of: BTreeMap<String, u64>,
}

impl Layers {
    /// No layer is listed, nothing is assigned and the current layer is the default.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.current == DEFAULT && self.of.is_empty()
    }

    /// A listed layer by id (`None` for the default layer, which is not listed).
    pub fn get(&self, id: u64) -> Option<&Layer> {
        self.items.iter().find(|l| l.id == id)
    }

    /// The layer a body is on.
    pub fn layer_of(&self, body: &str) -> u64 {
        self.of.get(body).copied().filter(|id| self.get(*id).is_some()).unwrap_or(DEFAULT)
    }

    /// The bodies on a layer, by name.
    pub fn members(&self, id: u64) -> Vec<String> {
        self.of.iter().filter(|(_, layer)| **layer == id).map(|(body, _)| body.clone()).collect()
    }

    /// The eye and lock a body follows: its layer's, or the default layer's (shown, unlocked).
    pub fn state_of(&self, body: &str) -> (bool, bool) {
        self.get(self.layer_of(body)).map_or((true, false), |l| (l.visible, l.locked))
    }

    /// Follow a body rename.
    pub fn rename_body(&mut self, old: &str, new: &str) {
        if let Some(layer) = self.of.remove(old) {
            self.of.insert(new.to_string(), layer);
        }
    }
}

/// Whether a colour is `#rrggbb`.
pub fn is_hex_colour(text: &str) -> bool {
    text.len() == 7 && text.starts_with('#') && text[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_layers_fall_back_to_the_default_layer() {
        let mut layers = Layers::default();
        layers.of.insert("Plate".into(), 7);
        assert_eq!(layers.layer_of("Plate"), DEFAULT, "a body on a layer that is not listed is on the default layer");
        layers.items.push(Layer { id: 7, name: "Parts".into(), colour: "#ff0000".into(), visible: true, locked: false });
        assert_eq!(layers.layer_of("Plate"), 7);
        assert_eq!(layers.members(7), vec!["Plate".to_string()]);
        layers.rename_body("Plate", "Lid");
        assert_eq!(layers.layer_of("Lid"), 7);
        assert_eq!(layers.layer_of("Plate"), DEFAULT);
    }

    #[test]
    fn colours_are_hex_triplets() {
        assert!(is_hex_colour("#1a2B3c"));
        assert!(!is_hex_colour("#fff"));
        assert!(!is_hex_colour("1a2b3c"));
        assert!(!is_hex_colour("#zzzzzz"));
    }
}
