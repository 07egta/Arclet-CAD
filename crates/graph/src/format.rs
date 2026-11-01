//! Versioned graph documents. Saved graphs contain inputs and layout, never evaluation caches.
//! Loading is transactional: callers receive a fully checked graph or keep the old document.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use solidcraft_geom::Vec3;
use solidcraft_slice::Tri;

use crate::{Graph, GraphError, MAX_ITEMS, MAX_NODES, Modifier, Node, NodeId, Poly, Socket, Value, Wire, component, valid_position, value_weight};

pub const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 256 * 1024;
const DOCUMENT_FORMAT: &str = "melt3d.node-graph";
const DOCUMENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: String,
    version: u32,
    #[serde(default)]
    next_id: u32,
    nodes: Vec<SavedNode>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedNode {
    id: u32,
    component: String,
    at: (f32, f32),
    inputs: Vec<SavedSocket>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSocket {
    constant: Constant,
    wire: Option<SavedWire>,
    modifier: SavedModifier,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedWire {
    from: u32,
    output: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedModifier {
    None,
    Flatten,
    Graft,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case", deny_unknown_fields)]
enum Constant {
    Number(f64),
    Bool(bool),
    Text(String),
    Point(Vec3),
    Curve { points: Vec<Vec3>, closed: bool },
    Mesh(Vec<Tri>),
    List(Vec<Constant>),
}

fn invalid(message: impl Into<String>) -> GraphError {
    GraphError::Invalid(message.into())
}

impl Graph {
    /// Serialize editable inputs and node positions as a stable versioned JSON document.
    pub fn to_json(&self) -> Result<String, GraphError> {
        self.order()?;
        let mut nodes = Vec::with_capacity(self.nodes.len());
        let mut total_weight = 0usize;
        for (id, node) in &self.nodes {
            for socket in &node.inputs {
                total_weight = total_weight.saturating_add(value_weight(&socket.constant, 0).map_err(invalid)?);
                if total_weight > MAX_ITEMS {
                    return Err(invalid("graph constants exceed the document budget"));
                }
            }
            let inputs = node.inputs.iter().map(SavedSocket::from_socket).collect::<Result<Vec<_>, _>>()?;
            nodes.push(SavedNode { id: id.0, component: node.component.into(), at: node.at, inputs });
        }
        let text = serde_json::to_string_pretty(&Document { format: DOCUMENT_FORMAT.into(), version: DOCUMENT_VERSION, next_id: self.next, nodes })
            .map_err(|error| invalid(error.to_string()))?;
        if text.len() > MAX_DOCUMENT_BYTES {
            return Err(invalid("graph document exceeds 4 MiB"));
        }
        Ok(text)
    }

    /// Reject incompatible versions, duplicate IDs, invalid wires, cycles and unbounded data.
    pub fn from_json(text: &str) -> Result<Self, GraphError> {
        if text.len() > MAX_DOCUMENT_BYTES {
            return Err(invalid("graph document exceeds 4 MiB"));
        }
        let document: Document = serde_json::from_str(text).map_err(|error| invalid(format!("JSON: {error}")))?;
        if document.format != DOCUMENT_FORMAT || document.version != DOCUMENT_VERSION {
            return Err(invalid("unsupported graph document format or version"));
        }
        if document.nodes.len() > MAX_NODES {
            return Err(GraphError::TooManyNodes);
        }
        let mut graph = Graph { nodes: BTreeMap::new(), next: document.next_id };
        let mut total_weight = 0usize;
        for saved in document.nodes {
            let component = component(&saved.component).ok_or_else(|| GraphError::UnknownComponent(saved.component.clone()))?;
            if saved.inputs.len() != component.inputs.len() || !valid_position(saved.at) {
                return Err(invalid("invalid node position or input count"));
            }
            let id = NodeId(saved.id);
            if graph.nodes.contains_key(&id) {
                return Err(invalid("duplicate node identifier"));
            }
            let next = saved.id.checked_add(1).ok_or_else(|| invalid("node identifiers exhausted"))?;
            graph.next = graph.next.max(next);
            let mut inputs = Vec::with_capacity(saved.inputs.len());
            for socket in saved.inputs {
                let value = socket.constant.into_value(0)?;
                total_weight = total_weight.saturating_add(value_weight(&value, 0).map_err(invalid)?);
                if total_weight > MAX_ITEMS {
                    return Err(invalid("graph constants exceed the document budget"));
                }
                inputs.push(Socket {
                    constant: value,
                    wire: socket.wire.map(|wire| Wire { from: NodeId(wire.from), output: wire.output }),
                    modifier: match socket.modifier {
                        SavedModifier::None => Modifier::None,
                        SavedModifier::Flatten => Modifier::Flatten,
                        SavedModifier::Graft => Modifier::Graft,
                    },
                });
            }
            graph.nodes.insert(id, Node { component: component.name, inputs, at: saved.at });
        }
        graph.order()?;
        Ok(graph)
    }
}

impl SavedSocket {
    fn from_socket(socket: &Socket) -> Result<Self, GraphError> {
        Ok(Self {
            constant: Constant::from_value(&socket.constant, 0)?,
            wire: socket.wire.map(|wire| SavedWire { from: wire.from.0, output: wire.output }),
            modifier: match socket.modifier {
                Modifier::None => SavedModifier::None,
                Modifier::Flatten => SavedModifier::Flatten,
                Modifier::Graft => SavedModifier::Graft,
            },
        })
    }
}

fn finite_point(point: Vec3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

impl Constant {
    fn from_value(value: &Value, depth: usize) -> Result<Self, GraphError> {
        if depth > 16 || value_weight(value, depth).map_err(invalid)? > MAX_ITEMS {
            return Err(invalid("constant exceeds the document budget"));
        }
        match value {
            Value::Number(number) if number.is_finite() => Ok(Self::Number(*number)),
            Value::Bool(value) => Ok(Self::Bool(*value)),
            Value::Text(text) if text.len() <= MAX_TEXT_BYTES => Ok(Self::Text(text.clone())),
            Value::Point(point) if finite_point(*point) => Ok(Self::Point(*point)),
            Value::Poly(poly) if poly.pts.len() <= MAX_ITEMS && poly.pts.iter().all(|point| finite_point(*point)) => {
                Ok(Self::Curve { points: poly.pts.clone(), closed: poly.closed })
            }
            Value::Solid(triangles) if triangles.len() <= MAX_ITEMS / 3 && triangles.iter().flatten().all(|point| finite_point(*point)) => {
                Ok(Self::Mesh(triangles.as_ref().clone()))
            }
            Value::List(values) if values.len() <= MAX_ITEMS => {
                Ok(Self::List(values.iter().map(|value| Self::from_value(value, depth + 1)).collect::<Result<_, _>>()?))
            }
            Value::Layers(_) => Err(invalid("saved constants cannot contain sliced output; connect a Slice node")),
            _ => Err(invalid("constant is not finite or exceeds its size limit")),
        }
    }

    fn into_value(self, depth: usize) -> Result<Value, GraphError> {
        if depth > 16 {
            return Err(invalid("constant lists are nested too deeply"));
        }
        let value = match self {
            Self::Number(number) if number.is_finite() => Value::Number(number),
            Self::Bool(value) => Value::Bool(value),
            Self::Text(text) if text.len() <= MAX_TEXT_BYTES => Value::Text(text),
            Self::Point(point) if finite_point(point) => Value::Point(point),
            Self::Curve { points, closed } if points.len() <= MAX_ITEMS && points.iter().all(|point| finite_point(*point)) => {
                Value::Poly(Poly { pts: points, closed })
            }
            Self::Mesh(triangles) if triangles.len() <= MAX_ITEMS / 3 && triangles.iter().flatten().all(|point| finite_point(*point)) => {
                Value::Solid(Arc::new(triangles))
            }
            Self::List(values) if values.len() <= MAX_ITEMS => {
                Value::List(values.into_iter().map(|value| value.into_value(depth + 1)).collect::<Result<_, _>>()?)
            }
            _ => return Err(invalid("constant is not finite or exceeds its size limit")),
        };
        Ok(value)
    }
}
