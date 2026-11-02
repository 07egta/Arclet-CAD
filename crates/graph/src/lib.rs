//! A node graph in the style of a visual algorithm editor: components joined by wires, data held
//! in trees, lists matched item by item. The components make curves, patterns and solids, slice
//! them and write G-code.
//!
//! Everything is plain data, so it is tested without a window. A component that fails marks only
//! its own node (and the nodes after it); the rest of the graph still runs. All counts are capped.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::Arc;

use solidcraft_geom::Vec3;
use solidcraft_slice::{PrintConfig, Sliced, Tri};

mod components;
mod format;
mod incremental;
mod typing;
mod workspace;

pub use components::{COMPONENTS, Component, Input, component};
pub use format::{MAX_DOCUMENT_BYTES, MAX_TEXT_BYTES};
pub use incremental::{IncrementalEvaluator, IncrementalStats};
pub use typing::{DataType, TypeIssue, input_type, output_type};
pub use workspace::{GraphWorkspace, MAX_WORKSPACE_BYTES, ModelOutputLink, NodeGroup, NodeViewState};

/// Most nodes in one graph.
pub const MAX_NODES: usize = 2_000;
/// Most items one component run may make, and most items one wire may carry.
pub const MAX_ITEMS: usize = 100_000;
/// Total retained scalar/geometry units across every node in one desktop evaluation.
/// This prevents many individually valid patterns from retaining gigabytes of duplicate meshes.
pub const MAX_RETAINED_WEIGHT: usize = 2_000_000;

/// A curve: a chain of points, closed or open.
#[derive(Clone, Debug, PartialEq)]
pub struct Poly {
    pub pts: Vec<Vec3>,
    pub closed: bool,
}

/// One piece of data on a wire.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Text(String),
    Point(Vec3),
    Poly(Poly),
    Solid(Arc<Vec<Tri>>),
    Layers(Arc<Sliced>),
    /// A whole branch handed to a component that asked for it (see [`Input::gather`]).
    List(Vec<Value>),
}

impl Value {
    pub fn number(&self) -> Result<f64, String> {
        match self {
            Value::Number(n) if n.is_finite() => Ok(*n),
            Value::Number(_) => Err("a number is not finite".into()),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err("expected a number".into()),
        }
    }
    pub fn poly(&self) -> Result<&Poly, String> {
        match self {
            Value::Poly(p) => Ok(p),
            _ => Err("expected a curve".into()),
        }
    }
    /// A short name for the kind of data, for the UI.
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Bool(_) => "yes/no",
            Value::Text(_) => "text",
            Value::Point(_) => "point",
            Value::Poly(_) => "curve",
            Value::Solid(_) => "solid",
            Value::Layers(_) => "layers",
            Value::List(_) => "list",
        }
    }
}

/// Data on a wire: lists of values filed under branch paths.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tree {
    pub branches: BTreeMap<Vec<u32>, Vec<Value>>,
}

impl Tree {
    pub fn single(v: Value) -> Tree {
        Tree { branches: BTreeMap::from([(Vec::new(), vec![v])]) }
    }
    pub fn item_count(&self) -> usize {
        self.branches.values().map(Vec::len).sum()
    }
    /// Every item in one branch.
    pub fn flatten(&self) -> Tree {
        let all: Vec<Value> = self.branches.values().flatten().cloned().collect();
        Tree { branches: BTreeMap::from([(Vec::new(), all)]) }
    }
    /// Every item in a branch of its own.
    pub fn graft(&self) -> Tree {
        let mut out = BTreeMap::new();
        for (path, items) in &self.branches {
            for (i, v) in items.iter().enumerate() {
                let mut p = path.clone();
                p.push(i as u32);
                out.insert(p, vec![v.clone()]);
            }
        }
        Tree { branches: out }
    }
}

/// How an input reshapes the data before the component sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Modifier {
    #[default]
    None,
    Flatten,
    Graft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

/// A wire's start: output number `output` of node `from`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wire {
    pub from: NodeId,
    pub output: usize,
}

/// One input of a node: a typed-in value, or a wire that replaces it.
#[derive(Clone, Debug, PartialEq)]
pub struct Socket {
    pub constant: Value,
    pub wire: Option<Wire>,
    pub modifier: Modifier,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub component: &'static str,
    pub inputs: Vec<Socket>,
    /// Where the node sits on the canvas.
    pub at: (f32, f32),
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum GraphError {
    #[error("unknown component: {0}")]
    UnknownComponent(String),
    #[error("too many nodes (up to {MAX_NODES})")]
    TooManyNodes,
    #[error("no such node")]
    NoNode,
    #[error("no such input or output")]
    NoSocket,
    #[error("the wires form a loop")]
    Cycle,
    #[error("stopped")]
    Cancelled,
    #[error("invalid graph: {0}")]
    Invalid(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    pub nodes: BTreeMap<NodeId, Node>,
    next: u32,
}

impl Graph {
    pub fn new() -> Graph {
        Graph::default()
    }

    pub fn add_node(&mut self, component_name: &str, at: (f32, f32)) -> Result<NodeId, GraphError> {
        if !valid_position(at) {
            return Err(GraphError::Invalid("node position must be finite and within the canvas".into()));
        }
        if self.nodes.len() >= MAX_NODES {
            return Err(GraphError::TooManyNodes);
        }
        let c = component(component_name).ok_or_else(|| GraphError::UnknownComponent(component_name.to_string()))?;
        let inputs = (c.defaults)().into_iter().map(|constant| Socket { constant, wire: None, modifier: Modifier::None }).collect();
        let id = NodeId(self.next);
        if self.nodes.contains_key(&id) {
            return Err(GraphError::Invalid("node identifier counter conflicts with an existing node".into()));
        }
        self.next = self.next.checked_add(1).ok_or_else(|| GraphError::Invalid("node identifiers exhausted".into()))?;
        self.nodes.insert(id, Node { component: c.name, inputs, at });
        Ok(id)
    }

    /// Joins output `output` of `from` to input `input` of `to`, replacing any wire already there.
    pub fn connect(&mut self, from: NodeId, output: usize, to: NodeId, input: usize) -> Result<(), GraphError> {
        let outputs = self.nodes.get(&from).and_then(|n| component(n.component)).ok_or(GraphError::NoNode)?.outputs.len();
        if output >= outputs {
            return Err(GraphError::NoSocket);
        }
        let node = self.nodes.get_mut(&to).ok_or(GraphError::NoNode)?;
        let socket = node.inputs.get_mut(input).ok_or(GraphError::NoSocket)?;
        socket.wire = Some(Wire { from, output });
        Ok(())
    }

    pub fn disconnect(&mut self, to: NodeId, input: usize) -> Result<(), GraphError> {
        let node = self.nodes.get_mut(&to).ok_or(GraphError::NoNode)?;
        node.inputs.get_mut(input).ok_or(GraphError::NoSocket)?.wire = None;
        Ok(())
    }

    /// Connect a wire only if it leaves the graph acyclic. A rejected edit preserves the old wire.
    pub fn connect_checked(&mut self, from: NodeId, output: usize, to: NodeId, input: usize) -> Result<(), GraphError> {
        let old = self.nodes.get(&to).and_then(|n| n.inputs.get(input)).ok_or(GraphError::NoSocket)?.wire;
        self.connect(from, output, to, input)?;
        if let Err(error) = self.order() {
            if let Some(socket) = self.nodes.get_mut(&to).and_then(|n| n.inputs.get_mut(input)) {
                socket.wire = old;
            }
            return Err(error);
        }
        Ok(())
    }

    /// Move a card without accepting non-finite coordinates from a saved document or script.
    pub fn move_node(&mut self, id: NodeId, at: (f32, f32)) -> Result<(), GraphError> {
        if !valid_position(at) {
            return Err(GraphError::Invalid("node position must be finite and within the canvas".into()));
        }
        self.nodes.get_mut(&id).ok_or(GraphError::NoNode)?.at = at;
        Ok(())
    }

    pub fn set_constant(&mut self, node: NodeId, input: usize, value: Value) -> Result<(), GraphError> {
        let node = self.nodes.get_mut(&node).ok_or(GraphError::NoNode)?;
        node.inputs.get_mut(input).ok_or(GraphError::NoSocket)?.constant = value;
        Ok(())
    }

    pub fn set_modifier(&mut self, node: NodeId, input: usize, modifier: Modifier) -> Result<(), GraphError> {
        let node = self.nodes.get_mut(&node).ok_or(GraphError::NoNode)?;
        node.inputs.get_mut(input).ok_or(GraphError::NoSocket)?.modifier = modifier;
        Ok(())
    }

    /// Removes a node and every wire that came from it.
    pub fn remove_node(&mut self, id: NodeId) {
        self.nodes.remove(&id);
        for n in self.nodes.values_mut() {
            for s in &mut n.inputs {
                if s.wire.is_some_and(|w| w.from == id) {
                    s.wire = None;
                }
            }
        }
    }

    /// Runs the graph. A node that fails is listed in `errors` and so are the nodes after it.
    pub fn evaluate(&self) -> Result<Evaluation, GraphError> {
        self.evaluate_until(&|| false)
    }

    /// Like [`Graph::evaluate`], but checks `stop` between nodes and matched items and within
    /// the slicer's geometry/path batches. Returns [`GraphError::Cancelled`] when requested.
    pub fn evaluate_until(&self, stop: &dyn Fn() -> bool) -> Result<Evaluation, GraphError> {
        self.evaluate_with_budget(stop, MAX_RETAINED_WEIGHT)
    }

    fn evaluate_with_budget(&self, stop: &dyn Fn() -> bool, budget: usize) -> Result<Evaluation, GraphError> {
        if stop() {
            return Err(GraphError::Cancelled);
        }
        let order = self.order()?;
        let mut ev = Evaluation::default();
        let mut retained = 0usize;
        for id in order {
            if stop() {
                return Err(GraphError::Cancelled);
            }
            let Some(node) = self.nodes.get(&id) else { continue };
            let Some(c) = component(node.component) else { continue };
            match self.run_node(c, node, &ev, stop, false, budget.saturating_sub(retained)) {
                Ok(trees) => {
                    retained = retained.saturating_add(output_weight(&trees).map_err(GraphError::Invalid)?);
                    for (k, t) in trees.into_iter().enumerate() {
                        ev.outputs.insert((id, k), t);
                    }
                }
                Err(e) => {
                    if stop() {
                        return Err(GraphError::Cancelled);
                    }
                    ev.errors.insert(id, e);
                }
            }
        }
        Ok(ev)
    }

    fn order(&self) -> Result<Vec<NodeId>, GraphError> {
        if self.nodes.len() > MAX_NODES {
            return Err(GraphError::TooManyNodes);
        }
        let mut indegree: HashMap<NodeId, usize> = HashMap::new();
        let mut after: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for (id, node) in &self.nodes {
            if id.0 >= self.next {
                return Err(GraphError::Invalid("node identifier exceeds the document counter".into()));
            }
            let c = component(node.component).ok_or_else(|| GraphError::UnknownComponent(node.component.into()))?;
            if node.inputs.len() != c.inputs.len() || !valid_position(node.at) {
                return Err(GraphError::Invalid("invalid node inputs or position".into()));
            }
            let mut n = 0;
            for w in node.inputs.iter().filter_map(|s| s.wire) {
                let source = self.nodes.get(&w.from).ok_or(GraphError::NoNode)?;
                if component(source.component).is_none_or(|c| w.output >= c.outputs.len()) {
                    return Err(GraphError::NoSocket);
                }
                n += 1;
                after.entry(w.from).or_default().push(*id);
            }
            indegree.insert(*id, n);
        }
        // Stable order gives repeatable previews and tests across machines.
        let mut ready: Vec<NodeId> = indegree.iter().filter(|(_, n)| **n == 0).map(|(id, _)| *id).collect();
        ready.sort();
        let mut queue: VecDeque<NodeId> = ready.into();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(id) = queue.pop_front() {
            order.push(id);
            for next in after.get(&id).map(Vec::as_slice).unwrap_or(&[]) {
                if let Some(n) = indegree.get_mut(next) {
                    *n = n.saturating_sub(1);
                    if *n == 0 {
                        queue.push_back(*next);
                    }
                }
            }
        }
        if order.len() != self.nodes.len() {
            return Err(GraphError::Cycle);
        }
        Ok(order)
    }

    fn run_node(
        &self,
        c: &Component,
        node: &Node,
        ev: &Evaluation,
        stop: &dyn Fn() -> bool,
        lightweight: bool,
        budget: usize,
    ) -> Result<Vec<Tree>, String> {
        if lightweight && matches!(c.name, "Slice" | "G-code") {
            return Err("Print nodes require the desktop app while browser background printing is being prepared".into());
        }
        let mut trees: Vec<Tree> = Vec::with_capacity(node.inputs.len());
        for (i, s) in node.inputs.iter().enumerate() {
            let name = c.inputs.get(i).map(|x| x.name).unwrap_or("input");
            let t = match s.wire {
                Some(w) if self.nodes.contains_key(&w.from) => match ev.outputs.get(&(w.from, w.output)) {
                    Some(t) => t.clone(),
                    None => return Err(format!("'{name}' is fed by a node that has an error")),
                },
                _ => Tree::single(s.constant.clone()),
            };
            trees.push(match s.modifier {
                Modifier::None => t,
                Modifier::Flatten => t.flatten(),
                Modifier::Graft => t.graft(),
            });
        }
        match_and_run(c, &trees, stop, lightweight, budget)
    }
}

fn valid_position(at: (f32, f32)) -> bool {
    at.0.is_finite() && at.1.is_finite() && (0.0..=1_000_000.0).contains(&at.0) && (0.0..=1_000_000.0).contains(&at.1)
}

/// Cooperative browser preview. Each step evaluates at most one bounded, non-print node.
/// Large graphs and printing remain available in the desktop worker, where cancellation can
/// be observed inside slicing and G-code generation.
pub struct PreviewEvaluation {
    graph: Graph,
    order: VecDeque<NodeId>,
    total: usize,
    result: Evaluation,
    retained: usize,
}

impl PreviewEvaluation {
    pub fn new(graph: Graph) -> Result<Self, GraphError> {
        if graph.nodes.len() > 256 {
            return Err(GraphError::Invalid("browser preview supports up to 256 nodes; use the desktop app for larger graphs".into()));
        }
        let order = graph.order()?;
        let total = order.len();
        Ok(Self { graph, order: order.into(), total, result: Evaluation::default(), retained: 0 })
    }

    /// Returns true when all nodes have been processed. A failed node is isolated normally.
    pub fn step(&mut self, stop: &dyn Fn() -> bool) -> Result<bool, GraphError> {
        if stop() {
            return Err(GraphError::Cancelled);
        }
        let Some(id) = self.order.pop_front() else { return Ok(true) };
        if let Some(node) = self.graph.nodes.get(&id)
            && let Some(c) = component(node.component)
        {
            match self.graph.run_node(c, node, &self.result, stop, true, 131_072usize.saturating_sub(self.retained)) {
                Ok(trees) => {
                    self.retained = self.retained.saturating_add(output_weight(&trees).map_err(GraphError::Invalid)?);
                    for (output, tree) in trees.into_iter().enumerate() {
                        self.result.outputs.insert((id, output), tree);
                    }
                }
                Err(error) => {
                    self.result.errors.insert(id, error);
                }
            }
        }
        Ok(self.order.is_empty())
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.total.saturating_sub(self.order.len()), self.total)
    }

    pub fn finish(self) -> Result<Evaluation, GraphError> {
        if self.order.is_empty() { Ok(self.result) } else { Err(GraphError::Invalid("evaluation has not finished".into())) }
    }
}

/// What a run produced.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Evaluation {
    pub outputs: HashMap<(NodeId, usize), Tree>,
    pub errors: HashMap<NodeId, String>,
}

/// Runs a component over its inputs. The branches of all inputs are matched by path (an input
/// with a single branch serves every path), and inside a branch the lists are matched item by
/// item, the shorter ones repeating their last item.
fn match_and_run(c: &Component, trees: &[Tree], stop: &dyn Fn() -> bool, lightweight: bool, budget: usize) -> Result<Vec<Tree>, String> {
    let limit = if lightweight { 16_384 } else { MAX_RETAINED_WEIGHT }.min(budget);
    let mut input_weight = 0usize;
    for value in trees.iter().flat_map(|tree| tree.branches.values()).flatten() {
        input_weight = input_weight.saturating_add(value_weight(value, 0)?);
        if input_weight > limit {
            return Err("node inputs exceed the evaluation budget; reduce the pattern size".into());
        }
    }
    let mut paths: BTreeSet<&Vec<u32>> = BTreeSet::new();
    for t in trees {
        paths.extend(t.branches.keys());
    }
    let mut outs: Vec<Tree> = vec![Tree::default(); c.outputs.len()];
    let mut made = 0usize;
    let mut weight = 0usize;
    for path in paths {
        if stop() {
            return Err("stopped".into());
        }
        let mut lists: Vec<Vec<Value>> = Vec::with_capacity(trees.len());
        for (i, t) in trees.iter().enumerate() {
            let branch = match t.branches.get(path) {
                Some(b) => Some(b),
                None if t.branches.len() == 1 => t.branches.values().next(),
                None => None,
            };
            let list = match branch {
                Some(b) if c.inputs.get(i).is_some_and(|x| x.gather) => vec![Value::List(b.clone())],
                Some(b) => b.clone(),
                None => Vec::new(),
            };
            lists.push(list);
        }
        if lists.iter().any(Vec::is_empty) {
            continue;
        }
        let n = lists.iter().map(Vec::len).max().unwrap_or(0);
        if n > MAX_ITEMS {
            return Err(format!("too many items (up to {MAX_ITEMS})"));
        }
        for j in 0..n {
            if stop() {
                return Err("stopped".into());
            }
            let args: Vec<Value> = lists.iter().filter_map(|l| l.get(j.min(l.len().saturating_sub(1))).cloned()).collect();
            if args.len() != lists.len() {
                return Err("input lists could not be matched".into());
            }
            if estimated_work(c.name, &args)? > limit.saturating_sub(weight) {
                return Err("node output would exceed the evaluation budget; reduce the pattern size".into());
            }
            let result = components::run_until(c, &args, stop)?;
            if result.len() != outs.len() {
                return Err("the component made the wrong number of outputs".into());
            }
            for (out, items) in outs.iter_mut().zip(result) {
                for value in &items {
                    weight = weight.saturating_add(value_weight(value, 0)?);
                }
                if weight > limit {
                    return Err("node output exceeds the evaluation budget; reduce the pattern size".into());
                }
                made += items.len();
                if made > MAX_ITEMS {
                    return Err(format!("too many items (up to {MAX_ITEMS})"));
                }
                out.branches.entry(path.clone()).or_default().extend(items);
            }
        }
    }
    Ok(outs)
}

fn value_weight(value: &Value, depth: usize) -> Result<usize, String> {
    if depth > 16 {
        return Err("input lists are nested too deeply".into());
    }
    match value {
        Value::Number(n) if !n.is_finite() => Err("a number is not finite".into()),
        Value::Point(p) if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() => Err("a point is not finite".into()),
        Value::Text(text) => Ok(text.len().saturating_div(16).saturating_add(1)),
        Value::Poly(poly) => {
            if poly.pts.iter().any(|p| !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite()) {
                return Err("a curve contains a non-finite point".into());
            }
            Ok(poly.pts.len().max(1))
        }
        Value::Solid(triangles) => {
            if triangles.iter().flatten().any(|point| !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite()) {
                return Err("a mesh contains a non-finite point".into());
            }
            Ok(triangles.len().saturating_mul(3))
        }
        Value::Layers(layers) => {
            if !layers.layer_height.is_finite()
                || layers
                    .layers
                    .iter()
                    .any(|layer| !layer.top.is_finite() || layer.loops.iter().flatten().any(|point| !point.x.is_finite() || !point.y.is_finite()))
            {
                return Err("sliced layers contain a non-finite value".into());
            }
            Ok(layers.layers.iter().flat_map(|layer| &layer.loops).map(Vec::len).fold(0usize, usize::saturating_add))
        }
        Value::List(values) => values.iter().try_fold(1usize, |sum, value| Ok(sum.saturating_add(value_weight(value, depth + 1)?))),
        _ => Ok(1),
    }
}

fn output_weight(trees: &[Tree]) -> Result<usize, String> {
    trees.iter().flat_map(|tree| tree.branches.values()).flatten().try_fold(0usize, |sum, value| Ok(sum.saturating_add(value_weight(value, 0)?)))
}

fn estimated_work(name: &str, args: &[Value]) -> Result<usize, String> {
    let first = args.first().map(|v| value_weight(v, 0)).transpose()?.unwrap_or(1);
    let multiplier = match name {
        "Linear Pattern" | "Circular Pattern" => args.get(1).map(Value::number).transpose()?.unwrap_or(1.0).max(0.0) as usize,
        "Extrude" => 12,
        _ => 1,
    };
    let count = if name == "Series" { args.get(2).map(Value::number).transpose()?.unwrap_or(0.0).max(0.0) as usize } else { 0 };
    Ok(first.saturating_mul(multiplier).max(count))
}

/// A generic Marlin-style printer set up from the numbers a node gives.
pub(crate) fn print_config(line_width: f64, nozzle: f64, bed: f64, walls: f64, spacing: f64) -> PrintConfig {
    PrintConfig {
        line_width,
        nozzle_temp: nozzle,
        bed_temp: bed,
        walls: walls.clamp(0.0, 8.0) as u32,
        infill_spacing: spacing.max(0.0),
        ..PrintConfig::default()
    }
}

#[cfg(test)]
mod tests;
