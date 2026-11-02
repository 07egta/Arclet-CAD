//! Incremental evaluation of editable node graphs.
//!
//! This is an opt-in evaluator. The existing Graph::evaluate and desktop mesh
//! insertion APIs keep their behavior. A failed or cancelled update cannot
//! replace the last complete evaluation.

use std::collections::{BTreeSet, HashMap};

use crate::{Evaluation, Graph, GraphError, MAX_RETAINED_WEIGHT, NodeId, Tree, component, output_weight};

/// Diagnostic counts for a completed incremental update.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IncrementalStats {
    /// Nodes whose component was executed again, including nodes that errored.
    pub recomputed: Vec<NodeId>,
    /// Nodes whose earlier output or error was retained.
    pub reused: Vec<NodeId>,
}

/// A bounded, transactional cache owned by one editable graph workspace.
///
/// Inputs and dependencies, not visual canvas positions, determine whether a
/// component needs to run again. Errors are cached as well as successful outputs.
/// Do not share one cache between unrelated concurrent graph documents.
#[derive(Clone, Debug, Default)]
pub struct IncrementalEvaluator {
    previous: Option<Graph>,
    result: Evaluation,
    /// Retained resource weight immediately before each node in the last run.
    retained_before: HashMap<NodeId, usize>,
}

impl IncrementalEvaluator {
    pub fn new() -> Self {
        Self::default()
    }

    /// The last *completed* evaluation, if any.
    pub fn current(&self) -> Option<&Evaluation> {
        self.previous.as_ref().map(|_| &self.result)
    }

    /// Only return results produced for this exact graph snapshot.
    /// Useful for hosts attaching geometry to editable CAD documents.
    pub fn current_for(&self, graph: &Graph) -> Option<&Evaluation> {
        if self.previous.as_ref() == Some(graph) {
            Some(&self.result)
        } else {
            None
        }
    }

    /// Release all cached values, including potentially large geometry.
    pub fn clear(&mut self) {
        self.previous = None;
        self.result = Evaluation::default();
        self.retained_before.clear();
    }

    /// Incrementally evaluate a graph. Cancellation preserves the old result.
    pub fn update(&mut self, graph: &Graph, stop: &dyn Fn() -> bool) -> Result<IncrementalStats, GraphError> {
        self.update_revision(graph, 0, &|| 0, stop)
    }

    /// Reject an obsolete result if the editor has changed to a later revision.
    ///
    /// Callers provide the revision captured with this immutable graph snapshot
    /// and a callback reading the latest editor revision. The callback is checked
    /// before and after expensive work, including immediately before publishing.
    /// The UI must also compare revisions when it receives a worker result.
    pub fn update_revision(
        &mut self,
        graph: &Graph,
        revision: u64,
        latest_revision: &dyn Fn() -> u64,
        stop: &dyn Fn() -> bool,
    ) -> Result<IncrementalStats, GraphError> {
        let obsolete = || stop() || latest_revision() != revision;
        if obsolete() {
            return Err(GraphError::Cancelled);
        }

        // Validate every endpoint, socket and cycle before touching the cache.
        let order = graph.order()?;
        let mut next = Evaluation::default();
        let mut changed = BTreeSet::new();
        let mut stats = IncrementalStats::default();
        let mut retained = 0usize;
        let mut retained_before = HashMap::with_capacity(graph.nodes.len());

        for id in order {
            if obsolete() {
                return Err(GraphError::Cancelled);
            }
            retained_before.insert(id, retained);
            let Some(node) = graph.nodes.get(&id) else {
                return Err(GraphError::NoNode);
            };
            let Some(c) = component(node.component) else {
                return Err(GraphError::UnknownComponent(node.component.into()));
            };
            let previous_node = self.previous.as_ref().and_then(|old| old.nodes.get(&id));
            let input_changed = previous_node.is_none_or(|old| old.component != node.component || old.inputs != node.inputs);
            let parent_changed = node.inputs.iter().filter_map(|socket| socket.wire).any(|wire| changed.contains(&wire.from));
            // The per-node work budget is the *remaining* global budget.
            // A preceding independent edit may change that budget even if
            // this node and all its own ancestors were untouched.
            let budget_changed = self.retained_before.get(&id) != Some(&retained);
            let mut needs_run = input_changed || parent_changed || budget_changed;

            // Reuse a node only when *all* output sockets are present, or its
            // earlier execution failed. Never reuse partial/evicted output.
            let previous_error = self.result.errors.get(&id);
            let cached: Option<Vec<Tree>> = if previous_error.is_some() {
                None
            } else {
                (0..c.outputs.len()).map(|output| self.result.outputs.get(&(id, output)).cloned()).collect()
            };
            if previous_error.is_none() && cached.is_none() {
                needs_run = true;
            }
            // Even a valid cached output must fit after any newly inserted
            // earlier nodes have consumed their portion of the global budget.
            if !needs_run && let Some(ref trees) = cached {
                match output_weight(trees) {
                    Ok(weight) if weight <= MAX_RETAINED_WEIGHT.saturating_sub(retained) => {}
                    _ => needs_run = true,
                }
            }

            if needs_run {
                changed.insert(id);
                stats.recomputed.push(id);
                match graph.run_node(c, node, &next, &obsolete, false, MAX_RETAINED_WEIGHT.saturating_sub(retained)) {
                    Ok(trees) => {
                        if obsolete() {
                            return Err(GraphError::Cancelled);
                        }
                        retained = retained.saturating_add(output_weight(&trees).map_err(GraphError::Invalid)?);
                        for (output, tree) in trees.into_iter().enumerate() {
                            next.outputs.insert((id, output), tree);
                        }
                    }
                    Err(error) => {
                        if obsolete() {
                            return Err(GraphError::Cancelled);
                        }
                        next.errors.insert(id, error);
                    }
                }
            } else {
                stats.reused.push(id);
                if let Some(error) = previous_error {
                    next.errors.insert(id, error.clone());
                } else if let Some(trees) = cached {
                    retained = retained.saturating_add(output_weight(&trees).map_err(GraphError::Invalid)?);
                    for (output, tree) in trees.into_iter().enumerate() {
                        next.outputs.insert((id, output), tree);
                    }
                }
            }
        }

        // Snapshot cloning may be nontrivial on large graphs. Do it before
        // the final revision check so an edit during the clone is discarded.
        let snapshot = graph.clone();
        if obsolete() {
            return Err(GraphError::Cancelled);
        }
        // Publish together: failed work has not mutated the previous cache.
        self.previous = Some(snapshot);
        self.result = next;
        self.retained_before = retained_before;
        Ok(stats)
    }
}
