//! Optional graph workspace metadata. The legacy graph JSON v1 is unchanged.
//!
//! Model links in this document record *intent*, not an automatic document
//! update. A CAD host must explicitly resolve object IDs, run evaluation,
//! validate geometry and commit changes transactionally.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use solidcraft_slice::Tri;

use crate::{DataType, Graph, GraphError, IncrementalEvaluator, MAX_DOCUMENT_BYTES, MAX_NODES, NodeId, Value, output_type};

const FORMAT: &str = "arclet.graph-workspace";
const VERSION: u32 = 1;
/// Includes at most 64 KiB of grouping/view/link metadata beyond the graph.
pub const MAX_WORKSPACE_BYTES: usize = MAX_DOCUMENT_BYTES + 64 * 1024;
const MAX_GROUPS: usize = 512;
const MAX_LABEL_BYTES: usize = 128;

/// UI-only grouping: nested groups are not represented by v1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeGroup {
    pub id: u32,
    pub name: String,
    pub members: Vec<NodeId>,
    pub collapsed: bool,
}

/// Saved canvas UI state; this flag does not disable graph computation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeViewState {
    pub node: NodeId,
    pub collapsed: bool,
    pub selected: bool,
}

/// Persistent reference to an external CAD document object.
///
/// The current graph layer does not perform live updates or document commits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelOutputLink {
    pub node: NodeId,
    pub output: usize,
    pub model_object_id: String,
}

/// Output geometry prepared for a CAD host to validate and commit.
/// This never mutates a CAD document, and is not persisted as a mesh cache.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelMeshSnapshot {
    pub model_object_id: String,
    /// Multiple tree items can form one CAD object; each mesh is Arc-shared.
    pub meshes: Vec<Arc<Vec<Tri>>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphWorkspace {
    pub graph: Graph,
    pub groups: Vec<NodeGroup>,
    pub node_states: Vec<NodeViewState>,
    pub model_links: Vec<ModelOutputLink>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceFile {
    format: String,
    version: u32,
    graph: serde_json::Value,
    groups: Vec<SavedGroup>,
    node_states: Vec<SavedNodeViewState>,
    model_links: Vec<SavedModelOutputLink>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedGroup {
    id: u32,
    name: String,
    members: Vec<u32>,
    collapsed: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedNodeViewState {
    node: u32,
    collapsed: bool,
    selected: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedModelOutputLink {
    node: u32,
    output: usize,
    model_object_id: String,
}

fn invalid(message: impl Into<String>) -> GraphError {
    GraphError::Invalid(message.into())
}

impl GraphWorkspace {
    /// Resolve **all** configured solid outputs without modifying the model.
    /// An errored/missing/non-solid output rejects the entire snapshot so a
    /// host can validate the full set before starting a document transaction.
    pub fn resolved_model_meshes(&self, evaluator: &IncrementalEvaluator) -> Result<Vec<ModelMeshSnapshot>, GraphError> {
        self.validate()?;
        let evaluation = evaluator.current_for(&self.graph).ok_or_else(|| {
            invalid("no completed evaluation matches this graph revision; recompute before updating the CAD model")
        })?;
        let mut snapshots = Vec::with_capacity(self.model_links.len());
        for link in &self.model_links {
            let tree = evaluation.outputs.get(&(link.node, link.output)).ok_or_else(|| {
                invalid(format!("model link '{}' has no successful evaluation output", link.model_object_id))
            })?;
            let mut meshes = Vec::new();
            for value in tree.branches.values().flatten() {
                match value {
                    Value::Solid(triangles) => meshes.push(Arc::clone(triangles)),
                    other => return Err(invalid(format!(
                        "model link '{}' expects Solid, received {}", link.model_object_id, other.kind()
                    ))),
                }
            }
            if meshes.is_empty() {
                return Err(invalid(format!("model link '{}' resolved to an empty output", link.model_object_id)));
            }
            snapshots.push(ModelMeshSnapshot { model_object_id: link.model_object_id.clone(), meshes });
        }
        Ok(snapshots)
    }

    pub fn new(graph: Graph) -> Self {
        Self { graph, ..Self::default() }
    }

    /// Serialize editable data only; cached meshes and computation errors
    /// are intentionally excluded. The original Graph::to_json stays v1.
    pub fn to_json(&self) -> Result<String, GraphError> {
        self.validate()?;
        let graph = serde_json::from_str(&self.graph.to_json()?).map_err(|error| invalid(error.to_string()))?;
        let file = WorkspaceFile {
            format: FORMAT.into(),
            version: VERSION,
            graph,
            groups: self.groups.iter().map(|g| SavedGroup {
                id: g.id, name: g.name.clone(), members: g.members.iter().map(|n| n.0).collect(), collapsed: g.collapsed,
            }).collect(),
            node_states: self.node_states.iter().map(|s| SavedNodeViewState { node: s.node.0, collapsed: s.collapsed, selected: s.selected }).collect(),
            model_links: self.model_links.iter().map(|l| SavedModelOutputLink {
                node: l.node.0, output: l.output, model_object_id: l.model_object_id.clone(),
            }).collect(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(|error| invalid(error.to_string()))?;
        if json.len() > MAX_WORKSPACE_BYTES {
            return Err(invalid("graph workspace exceeds the document budget"));
        }
        Ok(json)
    }

    /// Completely validate a candidate before the caller replaces its open workspace.
    pub fn from_json(json: &str) -> Result<Self, GraphError> {
        if json.len() > MAX_WORKSPACE_BYTES {
            return Err(invalid("graph workspace exceeds the document budget"));
        }
        let file: WorkspaceFile = serde_json::from_str(json).map_err(|error| invalid(format!("JSON: {error}")))?;
        if file.format != FORMAT || file.version != VERSION {
            return Err(invalid("unsupported graph workspace version"));
        }
        // This also applies the original graph's cycle and hostile-input checks.
        let graph_json = serde_json::to_string(&file.graph).map_err(|error| invalid(error.to_string()))?;
        let graph = Graph::from_json(&graph_json)?;
        let workspace = Self {
            graph,
            groups: file.groups.into_iter().map(|g| NodeGroup {
                id: g.id, name: g.name, members: g.members.into_iter().map(NodeId).collect(), collapsed: g.collapsed,
            }).collect(),
            node_states: file.node_states.into_iter().map(|s| NodeViewState {
                node: NodeId(s.node), collapsed: s.collapsed, selected: s.selected,
            }).collect(),
            model_links: file.model_links.into_iter().map(|l| ModelOutputLink {
                node: NodeId(l.node), output: l.output, model_object_id: l.model_object_id,
            }).collect(),
        };
        workspace.validate()?;
        Ok(workspace)
    }

    /// Validate references, sizes and unique identities in editable metadata.
    pub fn validate(&self) -> Result<(), GraphError> {
        self.graph.order()?;
        if self.graph.nodes.len() > MAX_NODES {
            return Err(GraphError::TooManyNodes);
        }
        if self.groups.len() > MAX_GROUPS || self.node_states.len() > MAX_NODES || self.model_links.len() > MAX_NODES {
            return Err(invalid("too many groups, node states or model links"));
        }
        let mut group_ids = BTreeSet::new();
        let mut grouped_nodes = BTreeSet::new();
        for group in &self.groups {
            if !group_ids.insert(group.id) || group.name.is_empty() || group.name.len() > MAX_LABEL_BYTES {
                return Err(invalid("duplicate group ID or invalid group name"));
            }
            for id in &group.members {
                if !self.graph.nodes.contains_key(id) || !grouped_nodes.insert(*id) {
                    return Err(invalid("group references an unknown or already grouped node"));
                }
            }
        }
        let mut states = BTreeSet::new();
        for state in &self.node_states {
            if !self.graph.nodes.contains_key(&state.node) || !states.insert(state.node) {
                return Err(invalid("duplicate or unknown node view state"));
            }
        }
        let mut object_ids = BTreeSet::new();
        for link in &self.model_links {
            let node = self.graph.nodes.get(&link.node).ok_or_else(|| invalid("model link references a missing node"))?;
            if output_type(node.component, link.output) != Some(DataType::Solid) {
                return Err(invalid("model link must reference a solid mesh output"));
            }
            if link.model_object_id.trim().is_empty()
                || link.model_object_id.len() > MAX_LABEL_BYTES
                || !object_ids.insert(&link.model_object_id)
            {
                return Err(invalid("empty, duplicate or oversized model object ID"));
            }
        }
        Ok(())
    }
}
