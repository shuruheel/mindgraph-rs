//! Per-agent scoped graph handle for multi-agent systems.

use std::sync::Arc;

use crate::error::Result;
use crate::graph::MindGraph;
use crate::query::TombstoneResult;
use crate::schema::edge::{CreateEdge, GraphEdge};
use crate::schema::edge_props::EdgeProps;
use crate::schema::node::{CreateNode, GraphNode};
use crate::schema::node_props::NodeProps;
use crate::schema::EdgeType;
use crate::types::*;

/// A scoped handle to a [`MindGraph`] with a fixed agent identity.
///
/// All mutation methods automatically set `changed_by` to this agent's identity.
/// Use [`MindGraph::agent`] to create one.
///
/// # Example
/// ```rust
/// use std::sync::Arc;
/// use mindgraph::*;
///
/// let graph = Arc::new(MindGraph::open_in_memory().unwrap());
/// let alice = graph.agent("alice");
/// let node = alice.add_entity("My Entity", "test").unwrap();
///
/// // The version record will show "alice" as changed_by
/// let history = alice.graph().node_history(&node.uid).unwrap();
/// assert_eq!(history[0].changed_by, "alice");
/// ```
pub struct AgentHandle {
    graph: Arc<MindGraph>,
    agent_id: String,
    parent_agent: Option<String>,
}

impl AgentHandle {
    pub(crate) fn new(graph: Arc<MindGraph>, agent_id: String, parent_agent: Option<String>) -> Self {
        Self { graph, agent_id, parent_agent }
    }

    /// Get the agent identity.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Get the parent agent identity, if this is a sub-agent.
    pub fn parent_agent(&self) -> Option<&str> {
        self.parent_agent.as_deref()
    }

    /// Access the underlying graph.
    pub fn graph(&self) -> &MindGraph {
        &self.graph
    }

    /// Get the underlying `Arc<MindGraph>`.
    pub fn graph_arc(&self) -> &Arc<MindGraph> {
        &self.graph
    }

    /// Create a child agent handle.
    pub fn sub_agent(&self, name: impl Into<String>) -> AgentHandle {
        AgentHandle {
            graph: self.graph.clone(),
            agent_id: name.into(),
            parent_agent: Some(self.agent_id.clone()),
        }
    }

    // ---- Mutation methods (auto-set changed_by) ----

    /// Add a node, recording this agent as the creator.
    pub fn add_node(&self, create: CreateNode) -> Result<GraphNode> {
        self.graph.add_node_as(create, &self.agent_id)
    }

    /// Add an edge, recording this agent as the creator.
    pub fn add_edge(&self, create: CreateEdge) -> Result<GraphEdge> {
        self.graph.add_edge_as(create, &self.agent_id)
    }

    /// Tombstone a node, recording this agent.
    pub fn tombstone(&self, uid: &Uid, reason: &str) -> Result<()> {
        self.graph.tombstone(uid, reason, &self.agent_id)
    }

    /// Tombstone a node and its connected edges, recording this agent.
    pub fn tombstone_cascade(&self, uid: &Uid, reason: &str) -> Result<TombstoneResult> {
        self.graph.tombstone_cascade_as(uid, reason, &self.agent_id)
    }

    /// Update a node via the builder pattern.
    #[allow(clippy::too_many_arguments)]
    pub fn update_node(
        &self,
        uid: &Uid,
        label: Option<String>,
        summary: Option<String>,
        confidence: Option<Confidence>,
        salience: Option<Salience>,
        props: Option<NodeProps>,
        reason: &str,
    ) -> Result<GraphNode> {
        self.graph.update_node(uid, label, summary, confidence, salience, props, &self.agent_id, reason)
    }

    // ---- Read methods (delegate directly) ----

    /// Get a node by UID.
    pub fn get_node(&self, uid: &Uid) -> Result<Option<GraphNode>> {
        self.graph.get_node(uid)
    }

    /// Get a live node by UID.
    pub fn get_live_node(&self, uid: &Uid) -> Result<GraphNode> {
        self.graph.get_live_node(uid)
    }

    /// Search nodes by text.
    pub fn search(&self, query: &str, opts: &crate::query::SearchOptions) -> Result<Vec<crate::query::SearchResult>> {
        self.graph.search(query, opts)
    }

    /// Find nodes matching a filter.
    pub fn find_nodes(&self, filter: &crate::query::NodeFilter) -> Result<Vec<GraphNode>> {
        self.graph.find_nodes(filter)
    }

    /// Get all nodes created by this agent.
    pub fn my_nodes(&self) -> Result<Vec<GraphNode>> {
        self.graph.nodes_by_agent(&self.agent_id)
    }

    // ---- Convenience constructors ----

    /// Add a claim node.
    pub fn add_claim(&self, label: &str, content: &str, confidence: f64) -> Result<GraphNode> {
        use crate::schema::props::epistemic::ClaimProps;
        self.add_node(
            CreateNode::new(label, NodeProps::Claim(ClaimProps {
                content: content.to_string(),
                ..Default::default()
            }))
            .confidence(Confidence::new(confidence)?),
        )
    }

    /// Add an entity node.
    pub fn add_entity(&self, label: &str, entity_type: &str) -> Result<GraphNode> {
        use crate::schema::props::reality::EntityProps;
        self.add_node(CreateNode::new(label, NodeProps::Entity(EntityProps {
            entity_type: entity_type.to_string(),
            ..Default::default()
        })))
    }

    /// Add a goal node.
    pub fn add_goal(&self, label: &str, priority: &str) -> Result<GraphNode> {
        use crate::schema::props::intent::GoalProps;
        self.add_node(CreateNode::new(label, NodeProps::Goal(GoalProps {
            priority: Some(priority.to_string()),
            status: Some("active".to_string()),
            ..Default::default()
        })))
    }

    /// Add a link (edge) between two nodes with default props.
    pub fn add_link(&self, from: &Uid, to: &Uid, edge_type: EdgeType) -> Result<GraphEdge> {
        self.add_edge(CreateEdge::new(
            from.clone(),
            to.clone(),
            EdgeProps::default_for(edge_type),
        ))
    }

    /// Add a custom-typed node.
    pub fn add_custom_node<T: crate::schema::CustomNodeType>(
        &self,
        label: &str,
        props: T,
    ) -> Result<GraphNode> {
        let data = serde_json::to_value(&props).map_err(crate::Error::from)?;
        self.add_node(CreateNode::new(
            label,
            NodeProps::Custom {
                type_name: T::type_name().to_string(),
                layer: T::layer(),
                data,
            },
        ))
    }

    /// Create an async filtered event stream for this agent's events.
    #[cfg(feature = "async")]
    pub fn watch(&self, filter: crate::events::EventFilter) -> crate::watch::WatchStream {
        self.graph.watch(filter)
    }

    /// Create an async event stream filtered to events triggered by this agent.
    #[cfg(feature = "async")]
    pub fn watch_mine(&self) -> crate::watch::WatchStream {
        self.graph.watch(crate::events::EventFilter::new().agent(self.agent_id.clone()))
    }
}
