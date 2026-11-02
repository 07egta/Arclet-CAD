//! Conservative, opt-in port type checking for graph connections.
//!
//! Existing Graph::connect and saved v1 graphs remain accepted unchanged.
//! Shape is a union of point and polyline curve, not a B-rep or NURBS surface.
//! Vector and Surface are reserved semantic types, not implemented runtime values.

use crate::{Graph, GraphError, NodeId, component};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Number,
    Boolean,
    Text,
    Vector,
    Point,
    Curve,
    Surface,
    Solid,
    Layers,
    /// A transformable point or polyline curve.
    Shape,
    /// A component or port missing from the type tables. Checks against it always pass, so an
    /// unknown port never blocks a connection.
    Unknown,
}

impl DataType {
    pub fn name(self) -> &'static str {
        match self {
            Self::Number => "Number",
            Self::Boolean => "Boolean",
            Self::Text => "Text",
            Self::Vector => "Vector (reserved)",
            Self::Point => "Point",
            Self::Curve => "Curve (polyline)",
            Self::Surface => "Surface (reserved)",
            Self::Solid => "Solid (triangle mesh)",
            Self::Layers => "Sliced Layers",
            Self::Shape => "Point or Curve",
            Self::Unknown => "Unknown",
        }
    }

    /// True only if the current node runtime implements this kind of value.
    pub fn is_supported(self) -> bool {
        !matches!(self, Self::Vector | Self::Surface | Self::Unknown)
    }

    /// Whether a wire carrying `source` may feed an input of this type.
    ///
    /// Shape feeds Shape inputs (Move, Rotate, patterns) from Point or Curve. It does NOT feed
    /// Curve inputs (Extrude): the runtime Extrude takes a polyline only (`Value::poly`), and a
    /// Shape may be a single Point, which would fail at evaluation. Keep this rule until Extrude
    /// accepts points or the type is narrowed. Unknown on either side always passes.
    pub fn accepts(self, source: Self) -> bool {
        self == source
            || self == Self::Unknown
            || source == Self::Unknown
            || matches!((self, source), (Self::Number, Self::Boolean) | (Self::Shape, Self::Point | Self::Curve))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeIssue {
    pub source: NodeId,
    pub source_output: usize,
    pub target: NodeId,
    pub target_input: usize,
    pub expected: DataType,
    pub actual: DataType,
    pub input_name: &'static str,
}

impl TypeIssue {
    pub fn message(&self) -> String {
        format!(
            "node {} input '{}' (socket {}) expects {}, but node {} output {} provides {}",
            self.target.0,
            self.input_name,
            self.target_input,
            self.expected.name(),
            self.source.0,
            self.source_output,
            self.actual.name()
        )
    }
}

/// Declared input port type. None means the port index is out of range for a known component.
/// A component missing from the tables gives `Some(DataType::Unknown)`.
pub fn input_type(name: &str, index: usize) -> Option<DataType> {
    let ty = match name {
        "Number" | "Series" | "Add" | "Subtract" | "Multiply" | "Divide" | "Point" | "Circle" | "Rectangle" => {
            match component(name)?.inputs.get(index) {
                Some(_) => DataType::Number,
                None => return None,
            }
        }
        "Move" | "Rotate" | "Linear Pattern" | "Circular Pattern" => match index {
            0 => DataType::Shape,
            i if i < component(name)?.inputs.len() => DataType::Number,
            _ => return None,
        },
        "Extrude" => match index {
            0 => DataType::Curve,
            1 => DataType::Number,
            _ => return None,
        },
        "Merge Solids" => match index {
            0 => DataType::Solid,
            _ => return None,
        },
        "Slice" => match index {
            0 => DataType::Solid,
            1 => DataType::Number,
            _ => return None,
        },
        "G-code" => match index {
            0 => DataType::Layers,
            i if i < component(name)?.inputs.len() => DataType::Number,
            _ => return None,
        },
        _ => DataType::Unknown,
    };
    Some(ty)
}

/// Declared output port type, conservatively using Shape for polymorphic nodes.
/// None means the port index is out of range for a known component. A component missing from
/// the tables gives `Some(DataType::Unknown)`.
pub fn output_type(name: &str, index: usize) -> Option<DataType> {
    let Some(known) = component(name) else { return Some(DataType::Unknown) };
    if index >= known.outputs.len() {
        return None;
    }
    Some(match name {
        "Number" | "Series" | "Add" | "Subtract" | "Multiply" | "Divide" => DataType::Number,
        "Point" => DataType::Point,
        "Circle" | "Rectangle" => DataType::Curve,
        "Move" | "Rotate" | "Linear Pattern" | "Circular Pattern" => DataType::Shape,
        "Extrude" | "Merge Solids" => DataType::Solid,
        "Slice" => DataType::Layers,
        "G-code" => DataType::Text,
        _ => DataType::Unknown,
    })
}

impl Graph {
    /// Report all incompatible wires without changing legacy evaluation behavior.
    pub fn type_issues(&self) -> Result<Vec<TypeIssue>, GraphError> {
        self.order()?;
        let mut issues = Vec::new();
        for (target, node) in &self.nodes {
            for (index, socket) in node.inputs.iter().enumerate() {
                let Some(wire) = socket.wire else { continue };
                let source_node = self.nodes.get(&wire.from).ok_or(GraphError::NoNode)?;
                let expected = input_type(node.component, index).ok_or(GraphError::NoSocket)?;
                let actual = output_type(source_node.component, wire.output).ok_or(GraphError::NoSocket)?;
                if !expected.accepts(actual) {
                    let input_name = component(node.component).and_then(|c| c.inputs.get(index)).map(|input| input.name).unwrap_or("input");
                    issues.push(TypeIssue {
                        source: wire.from,
                        source_output: wire.output,
                        target: *target,
                        target_input: index,
                        expected,
                        actual,
                        input_name,
                    });
                }
            }
        }
        Ok(issues)
    }

    /// Validate types and cycles before connecting. A rejected edit is atomic.
    pub fn connect_typed_checked(&mut self, from: NodeId, output: usize, to: NodeId, input: usize) -> Result<(), GraphError> {
        let source = self.nodes.get(&from).ok_or(GraphError::NoNode)?;
        let target = self.nodes.get(&to).ok_or(GraphError::NoNode)?;
        let actual = output_type(source.component, output).ok_or(GraphError::NoSocket)?;
        let expected = input_type(target.component, input).ok_or(GraphError::NoSocket)?;
        if !expected.accepts(actual) {
            let input_name = component(target.component).and_then(|c| c.inputs.get(input)).map(|i| i.name).unwrap_or("input");
            let issue = TypeIssue { source: from, source_output: output, target: to, target_input: input, expected, actual, input_name };
            return Err(GraphError::Invalid(issue.message()));
        }
        self.connect_checked(from, output, to, input)
    }
}
