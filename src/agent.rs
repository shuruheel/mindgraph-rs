//! Per-agent scoped graph handle for multi-agent systems.

use std::sync::Arc;

use crate::error::Result;
use crate::graph::MindGraph;
use crate::query::{GraphStats, SearchOptions, SearchResult, TombstoneResult, VersionRecord};
use crate::schema::edge::{CreateEdge, GraphEdge};
use crate::schema::edge_props::EdgeProps;
use crate::schema::node::{CreateNode, GraphNode};
use crate::schema::node_props::NodeProps;
use crate::schema::{EdgeType, NodeType};
use crate::traversal::{PathStep, TraversalOptions};
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
#[derive(Clone)]
pub struct AgentHandle {
    graph: Arc<MindGraph>,
    agent_id: String,
    parent_agent: Option<String>,
}

impl AgentHandle {
    pub(crate) fn new(
        graph: Arc<MindGraph>,
        agent_id: String,
        parent_agent: Option<String>,
    ) -> Self {
        Self {
            graph,
            agent_id,
            parent_agent,
        }
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

    /// Tombstone an edge, recording this agent.
    pub fn tombstone_edge(&self, uid: &Uid, reason: &str) -> Result<()> {
        self.graph.tombstone_edge(uid, reason, &self.agent_id)
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
        self.graph.update_node(
            uid,
            label,
            summary,
            confidence,
            salience,
            props,
            &self.agent_id,
            reason,
        )
    }

    /// Update an edge, recording this agent.
    pub fn update_edge(
        &self,
        uid: &Uid,
        confidence: Option<Confidence>,
        weight: Option<f64>,
        props: Option<EdgeProps>,
        reason: &str,
    ) -> Result<GraphEdge> {
        self.graph
            .update_edge_as(uid, confidence, weight, props, &self.agent_id, reason)
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

    /// Get an edge by UID.
    pub fn get_edge(&self, uid: &Uid) -> Result<Option<GraphEdge>> {
        self.graph.get_edge(uid)
    }

    /// Get live edges between two specific nodes.
    pub fn get_edge_between(
        &self,
        from_uid: &Uid,
        to_uid: &Uid,
        edge_type: Option<EdgeType>,
    ) -> Result<Vec<GraphEdge>> {
        self.graph.get_edge_between(from_uid, to_uid, edge_type)
    }

    /// Get all edges from a node.
    pub fn edges_from(&self, uid: &Uid, edge_type: Option<EdgeType>) -> Result<Vec<GraphEdge>> {
        self.graph.edges_from(uid, edge_type)
    }

    /// Get all edges to a node.
    pub fn edges_to(&self, uid: &Uid, edge_type: Option<EdgeType>) -> Result<Vec<GraphEdge>> {
        self.graph.edges_to(uid, edge_type)
    }

    /// Search nodes by text.
    pub fn search(&self, query: &str, opts: &SearchOptions) -> Result<Vec<SearchResult>> {
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

    // ---- Traversal ----

    /// Get all nodes reachable from a starting node.
    pub fn reachable(&self, start: &Uid, opts: &TraversalOptions) -> Result<Vec<PathStep>> {
        self.graph.reachable(start, opts)
    }

    /// Follow a reasoning chain from a claim node.
    pub fn reasoning_chain(&self, claim_uid: &Uid, max_depth: u32) -> Result<Vec<PathStep>> {
        self.graph.reasoning_chain(claim_uid, max_depth)
    }

    /// Get the neighborhood of a node up to a given depth.
    pub fn neighborhood(&self, uid: &Uid, depth: u32) -> Result<Vec<PathStep>> {
        self.graph.neighborhood(uid, depth)
    }

    // ---- History ----

    /// Get the full version history for a node.
    pub fn node_history(&self, uid: &Uid) -> Result<Vec<VersionRecord>> {
        self.graph.node_history(uid)
    }

    // ---- Count / Exists ----

    /// Count live nodes of a given type.
    pub fn count_nodes(&self, node_type: NodeType) -> Result<u64> {
        self.graph.count_nodes(node_type)
    }

    /// Check if a live (non-tombstoned) node exists.
    pub fn node_exists(&self, uid: &Uid) -> Result<bool> {
        self.graph.node_exists(uid)
    }

    // ---- Stats ----

    /// Get graph-wide statistics.
    pub fn stats(&self) -> Result<GraphStats> {
        self.graph.stats()
    }

    // ---- Convenience constructors ----

    /// Add a claim node.
    pub fn add_claim(&self, label: &str, content: &str, confidence: f64) -> Result<GraphNode> {
        use crate::schema::props::epistemic::ClaimProps;
        self.add_node(
            CreateNode::new(
                label,
                NodeProps::Claim(ClaimProps {
                    content: content.to_string(),
                    ..Default::default()
                }),
            )
            .confidence(Confidence::new(confidence)?),
        )
    }

    /// Add an entity node.
    pub fn add_entity(&self, label: &str, entity_type: &str) -> Result<GraphNode> {
        use crate::schema::props::reality::EntityProps;
        self.add_node(CreateNode::new(
            label,
            NodeProps::Entity(EntityProps {
                entity_type: entity_type.to_string(),
                ..Default::default()
            }),
        ))
    }

    /// Add a goal node.
    pub fn add_goal(&self, label: &str, priority: &str) -> Result<GraphNode> {
        use crate::schema::props::intent::GoalProps;
        self.add_node(CreateNode::new(
            label,
            NodeProps::Goal(GoalProps {
                priority: Some(priority.to_string()),
                status: Some("active".to_string()),
                ..Default::default()
            }),
        ))
    }

    /// Add an observation node.
    pub fn add_observation(&self, label: &str, description: &str) -> Result<GraphNode> {
        use crate::schema::props::reality::ObservationProps;
        self.add_node(
            CreateNode::new(
                label,
                NodeProps::Observation(ObservationProps {
                    content: description.to_string(),
                    ..Default::default()
                }),
            )
            .summary(description),
        )
    }

    /// Add a session node.
    pub fn add_session(&self, label: &str, focus: &str) -> Result<GraphNode> {
        use crate::schema::props::memory::SessionProps;
        self.add_node(
            CreateNode::new(
                label,
                NodeProps::Session(SessionProps {
                    focus_summary: Some(focus.to_string()),
                    ..Default::default()
                }),
            )
            .summary(focus),
        )
    }

    /// Add a preference node.
    pub fn add_preference(&self, label: &str, key: &str, value: &str) -> Result<GraphNode> {
        use crate::schema::props::memory::PreferenceProps;
        self.add_node(CreateNode::new(
            label,
            NodeProps::Preference(PreferenceProps {
                key: key.to_string(),
                value: value.to_string(),
                ..Default::default()
            }),
        ))
    }

    /// Add a summary node.
    pub fn add_summary(&self, label: &str, content: &str) -> Result<GraphNode> {
        use crate::schema::props::memory::SummaryProps;
        self.add_node(
            CreateNode::new(
                label,
                NodeProps::Summary(SummaryProps {
                    content: content.to_string(),
                    ..Default::default()
                }),
            )
            .summary(content),
        )
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
        self.graph
            .watch(crate::events::EventFilter::new().agent(self.agent_id.clone()))
    }
}
