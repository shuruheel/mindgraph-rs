use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::embeddings::EmbeddingProvider;
use crate::error::{Error, Result};
use crate::events::{EventFilter, GraphEvent, SubscriptionId};
use crate::provenance::ProvenanceRecord;
use crate::query::{
    BatchResult, Contradiction, DecayResult, GraphOp, GraphSnapshot, GraphStats, ImportResult,
    MergeResult, NodeFilter, Page, Pagination, PurgeResult, SearchOptions, SearchResult,
    TombstoneResult, TypedImportResult, TypedSnapshot, ValidatedBatch, VersionRecord,
};
use crate::schema::edge::{CreateEdge, GraphEdge};
use crate::schema::edge_props::EdgeProps;
use crate::schema::node::{CreateNode, GraphNode};
use crate::schema::node_props::NodeProps;
use crate::schema::{EdgeType, Layer, NodeType};
use crate::storage::CozoStorage;
use crate::traversal::{Direction, PathStep, TraversalOptions};
use crate::types::*;

#[allow(dead_code)]
const _ASSERT_SEND_SYNC: () = {
    const fn _assert<T: Send + Sync>() {}
    _assert::<MindGraph>();
};

type SubscriberMap = HashMap<u64, Arc<dyn Fn(&GraphEvent) + Send + Sync>>;

/// The main graph database interface.
pub struct MindGraph {
    storage: CozoStorage,
    default_agent: RwLock<String>,
    embedding_dim: RwLock<Option<usize>>,
    next_sub_id: AtomicU64,
    subscribers: RwLock<SubscriberMap>,
    #[cfg(feature = "async")]
    broadcast_tx: tokio::sync::broadcast::Sender<GraphEvent>,
}

impl MindGraph {
    /// Open a persistent graph at the given path (SQLite-backed).
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let storage = CozoStorage::open(path)?;
        let dim = storage.get_embedding_dimension()?;
        Ok(MindGraph {
            storage,
            default_agent: RwLock::new("system".into()),
            embedding_dim: RwLock::new(dim),
            next_sub_id: AtomicU64::new(1),
            subscribers: RwLock::new(HashMap::new()),
            #[cfg(feature = "async")]
            broadcast_tx: tokio::sync::broadcast::channel(1024).0,
        })
    }

    /// Create an in-memory graph (for testing).
    ///
    /// # Examples
    ///
    /// ```
    /// use mindgraph::MindGraph;
    /// let graph = MindGraph::open_in_memory().unwrap();
    /// ```
    pub fn open_in_memory() -> Result<Self> {
        let storage = CozoStorage::open_in_memory()?;
        let dim = storage.get_embedding_dimension()?;
        Ok(MindGraph {
            storage,
            default_agent: RwLock::new("system".into()),
            embedding_dim: RwLock::new(dim),
            next_sub_id: AtomicU64::new(1),
            subscribers: RwLock::new(HashMap::new()),
            #[cfg(feature = "async")]
            broadcast_tx: tokio::sync::broadcast::channel(1024).0,
        })
    }

    /// Open a persistent graph with a custom broadcast channel capacity.
    /// Requires the `async` feature.
    #[cfg(feature = "async")]
    pub fn open_with_broadcast_capacity(path: impl AsRef<Path>, capacity: usize) -> Result<Self> {
        let storage = CozoStorage::open(path)?;
        let dim = storage.get_embedding_dimension()?;
        Ok(MindGraph {
            storage,
            default_agent: RwLock::new("system".into()),
            embedding_dim: RwLock::new(dim),
            next_sub_id: AtomicU64::new(1),
            subscribers: RwLock::new(HashMap::new()),
            broadcast_tx: tokio::sync::broadcast::channel(capacity).0,
        })
    }

    /// Create an in-memory graph with a custom broadcast channel capacity.
    /// Requires the `async` feature.
    #[cfg(feature = "async")]
    pub fn open_in_memory_with_broadcast_capacity(capacity: usize) -> Result<Self> {
        let storage = CozoStorage::open_in_memory()?;
        let dim = storage.get_embedding_dimension()?;
        Ok(MindGraph {
            storage,
            default_agent: RwLock::new("system".into()),
            embedding_dim: RwLock::new(dim),
            next_sub_id: AtomicU64::new(1),
            subscribers: RwLock::new(HashMap::new()),
            broadcast_tx: tokio::sync::broadcast::channel(capacity).0,
        })
    }

    /// Wrap this graph in an `Arc` for sharing across threads.
    pub fn into_shared(self) -> Arc<MindGraph> {
        Arc::new(self)
    }

    /// Set the default agent identity for operations that don't specify one.
    pub fn set_default_agent(&self, name: impl Into<String>) {
        *self
            .default_agent
            .write()
            .unwrap_or_else(|e| e.into_inner()) = name.into();
    }

    /// Get the current default agent identity.
    pub fn default_agent(&self) -> String {
        self.default_agent
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Access the underlying storage for advanced queries.
    pub fn storage(&self) -> &CozoStorage {
        &self.storage
    }

    // ---- Node Operations ----

    /// Add a new node to the graph.
    ///
    /// # Examples
    ///
    /// ```
    /// use mindgraph::*;
    /// let graph = MindGraph::open_in_memory().unwrap();
    /// let node = graph.add_node(
    ///     CreateNode::new("My entity", NodeProps::Entity(EntityProps {
    ///         entity_type: "thing".into(),
    ///         ..Default::default()
    ///     }))
    /// ).unwrap();
    /// assert_eq!(node.node_type, NodeType::Entity);
    /// ```
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, create)))]
    pub fn add_node(&self, create: CreateNode) -> Result<GraphNode> {
        let layer = create.props.layer();
        let node_type = create.props.node_type();
        let ts = now();

        let node = GraphNode {
            uid: create.uid.unwrap_or_default(),
            node_type,
            layer,
            label: create.label,
            summary: create.summary,
            created_at: ts,
            updated_at: ts,
            version: 1,
            confidence: create.confidence,
            salience: create.salience,
            privacy_level: create.privacy_level,
            embedding_ref: None,
            tombstone_at: None,
            tombstone_reason: None,
            tombstone_by: None,
            props: create.props,
        };

        self.storage.insert_node(&node)?;

        let agent = self.default_agent();
        let snapshot = serde_json::to_value(&node)?;
        self.storage
            .insert_node_version(&node.uid, 1, snapshot, &agent, "create", "")?;

        self.emit(GraphEvent::NodeAdded {
            node: Box::new(node.clone()),
            changed_by: agent,
        });

        Ok(node)
    }

    /// Get a node by UID. Returns None if not found.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn get_node(&self, uid: &Uid) -> Result<Option<GraphNode>> {
        self.storage.get_node(uid)
    }

    /// Get a node by UID, returning an error if not found or tombstoned.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn get_live_node(&self, uid: &Uid) -> Result<GraphNode> {
        let node = self
            .storage
            .get_node(uid)?
            .ok_or_else(|| Error::NodeNotFound(uid.to_string()))?;
        if node.tombstone_at.is_some() {
            return Err(Error::Tombstoned(uid.to_string()));
        }
        Ok(node)
    }

    /// Update a node's mutable fields. Creates a new version.
    #[allow(clippy::too_many_arguments)]
    pub fn update_node(
        &self,
        uid: &Uid,
        label: Option<String>,
        summary: Option<String>,
        confidence: Option<Confidence>,
        salience: Option<Salience>,
        props: Option<NodeProps>,
        changed_by: &str,
        reason: &str,
    ) -> Result<GraphNode> {
        let mut node = self.get_live_node(uid)?;

        if let Some(l) = label {
            node.label = l;
        }
        if let Some(s) = summary {
            node.summary = s;
        }
        if let Some(c) = confidence {
            node.confidence = c;
        }
        if let Some(s) = salience {
            node.salience = s;
        }
        if let Some(p) = props {
            if p.node_type() != node.node_type {
                return Err(Error::TypeMismatch {
                    expected: node.node_type.to_string(),
                    got: p.node_type().to_string(),
                });
            }
            node.props = p;
        }

        node.version += 1;
        node.updated_at = now();

        self.storage.insert_node(&node)?;

        let snapshot = serde_json::to_value(&node)?;
        self.storage.insert_node_version(
            &node.uid,
            node.version,
            snapshot,
            changed_by,
            "update",
            reason,
        )?;

        self.emit(GraphEvent::NodeUpdated {
            uid: node.uid.clone(),
            version: node.version,
            node_type: node.node_type.clone(),
            layer: node.layer,
            changed_by: changed_by.to_string(),
        });

        Ok(node)
    }

    /// Begin building an update for a node. Call `.apply()` to execute.
    ///
    /// # Examples
    ///
    /// ```
    /// use mindgraph::*;
    /// let graph = MindGraph::open_in_memory().unwrap();
    /// let node = graph.add_entity("Alice", "person").unwrap();
    /// let updated = graph.update(&node.uid)
    ///     .label("Alice Smith")
    ///     .reason("Added last name")
    ///     .apply()
    ///     .unwrap();
    /// assert_eq!(updated.label, "Alice Smith");
    /// assert_eq!(updated.version, 2);
    /// ```
    pub fn update<'a>(&'a self, uid: &'a Uid) -> NodeUpdate<'a> {
        NodeUpdate {
            graph: self,
            uid,
            label: None,
            summary: None,
            confidence: None,
            salience: None,
            props: None,
            changed_by: None,
            reason: None,
        }
    }

    /// Begin building an update for an edge. Call `.apply()` to execute.
    pub fn update_edge_builder<'a>(&'a self, uid: &'a Uid) -> EdgeUpdate<'a> {
        EdgeUpdate {
            graph: self,
            uid,
            confidence: None,
            weight: None,
            props: None,
            changed_by: None,
            reason: None,
        }
    }

    // ---- Edge Operations ----

    /// Add a new edge to the graph. Validates that both endpoints are live.
    ///
    /// # Examples
    ///
    /// ```
    /// use mindgraph::*;
    /// let graph = MindGraph::open_in_memory().unwrap();
    /// let a = graph.add_entity("A", "test").unwrap();
    /// let b = graph.add_entity("B", "test").unwrap();
    /// let edge = graph.add_edge(CreateEdge::new(
    ///     a.uid.clone(), b.uid.clone(),
    ///     EdgeProps::Supports { strength: Some(0.9), support_type: None },
    /// )).unwrap();
    /// assert_eq!(edge.edge_type, EdgeType::Supports);
    /// ```
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, create)))]
    pub fn add_edge(&self, create: CreateEdge) -> Result<GraphEdge> {
        let edge_type = create.props.edge_type();
        let ts = now();

        let from_node = self.get_live_node(&create.from_uid)?;
        self.get_live_node(&create.to_uid)?;
        let layer = from_node.layer;

        let edge = GraphEdge {
            uid: Uid::new(),
            from_uid: create.from_uid,
            to_uid: create.to_uid,
            edge_type,
            layer,
            created_at: ts,
            updated_at: ts,
            version: 1,
            confidence: create.confidence,
            weight: create.weight,
            tombstone_at: None,
            props: create.props,
        };

        self.storage.insert_edge(&edge)?;

        let agent = self.default_agent();
        let snapshot = serde_json::to_value(&edge)?;
        self.storage
            .insert_edge_version(&edge.uid, 1, snapshot, &agent, "create", "")?;

        self.emit(GraphEvent::EdgeAdded {
            edge: Box::new(edge.clone()),
            changed_by: agent,
        });

        Ok(edge)
    }

    /// Get an edge by UID.
    pub fn get_edge(&self, uid: &Uid) -> Result<Option<GraphEdge>> {
        self.storage.get_edge(uid)
    }

    /// Get all edges from a node.
    pub fn edges_from(&self, uid: &Uid, edge_type: Option<EdgeType>) -> Result<Vec<GraphEdge>> {
        self.storage.query_edges_from(uid, edge_type)
    }

    /// Get all edges to a node.
    pub fn edges_to(&self, uid: &Uid, edge_type: Option<EdgeType>) -> Result<Vec<GraphEdge>> {
        self.storage.query_edges_to(uid, edge_type)
    }

    /// Get an edge by UID, returning an error if not found or tombstoned.
    pub fn get_live_edge(&self, uid: &Uid) -> Result<GraphEdge> {
        let edge = self
            .storage
            .get_edge(uid)?
            .ok_or_else(|| Error::EdgeNotFound(uid.to_string()))?;
        if edge.tombstone_at.is_some() {
            return Err(Error::Tombstoned(uid.to_string()));
        }
        Ok(edge)
    }

    /// Update an edge's mutable fields. Creates a new version.
    pub fn update_edge(
        &self,
        uid: &Uid,
        confidence: Option<Confidence>,
        weight: Option<f64>,
        props: Option<EdgeProps>,
        changed_by: &str,
        reason: &str,
    ) -> Result<GraphEdge> {
        let mut edge = self.get_live_edge(uid)?;

        if let Some(c) = confidence {
            edge.confidence = c;
        }
        if let Some(w) = weight {
            edge.weight = w;
        }
        if let Some(p) = props {
            if p.edge_type() != edge.edge_type {
                return Err(Error::TypeMismatch {
                    expected: edge.edge_type.to_string(),
                    got: p.edge_type().to_string(),
                });
            }
            edge.props = p;
        }

        edge.version += 1;
        edge.updated_at = now();

        self.storage.insert_edge(&edge)?;

        let snapshot = serde_json::to_value(&edge)?;
        self.storage.insert_edge_version(
            &edge.uid,
            edge.version,
            snapshot,
            changed_by,
            "update",
            reason,
        )?;

        Ok(edge)
    }

    // ---- Tombstone Operations ----

    /// Soft-delete a node.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn tombstone(&self, uid: &Uid, reason: &str, by: &str) -> Result<()> {
        let mut node = self.get_live_node(uid)?;
        node.tombstone_at = Some(now());
        node.tombstone_reason = Some(reason.to_string());
        node.tombstone_by = Some(by.to_string());
        node.version += 1;
        node.updated_at = now();

        self.storage.insert_node(&node)?;

        let snapshot = serde_json::to_value(&node)?;
        self.storage.insert_node_version(
            &node.uid,
            node.version,
            snapshot,
            by,
            "tombstone",
            reason,
        )?;

        self.emit(GraphEvent::NodeTombstoned {
            uid: uid.clone(),
            node_type: node.node_type.clone(),
            layer: node.layer,
            changed_by: by.to_string(),
        });

        Ok(())
    }

    /// Restore a tombstoned node.
    pub fn restore(&self, uid: &Uid) -> Result<()> {
        let mut node = self
            .storage
            .get_node(uid)?
            .ok_or_else(|| Error::NodeNotFound(uid.to_string()))?;

        if node.tombstone_at.is_none() {
            return Ok(());
        }

        node.tombstone_at = None;
        node.tombstone_reason = None;
        node.tombstone_by = None;
        node.version += 1;
        node.updated_at = now();

        self.storage.insert_node(&node)?;

        let snapshot = serde_json::to_value(&node)?;
        self.storage.insert_node_version(
            &node.uid,
            node.version,
            snapshot,
            "system",
            "restore",
            "",
        )?;

        Ok(())
    }

    /// Soft-delete an edge.
    pub fn tombstone_edge(&self, uid: &Uid, reason: &str, by: &str) -> Result<()> {
        let mut edge = self.get_live_edge(uid)?;
        let from_uid = edge.from_uid.clone();
        let to_uid = edge.to_uid.clone();
        let edge_type = edge.edge_type.clone();
        edge.tombstone_at = Some(now());
        edge.version += 1;
        edge.updated_at = now();

        self.storage.insert_edge(&edge)?;

        let snapshot = serde_json::to_value(&edge)?;
        self.storage.insert_edge_version(
            &edge.uid,
            edge.version,
            snapshot,
            by,
            "tombstone",
            reason,
        )?;

        self.emit(GraphEvent::EdgeTombstoned {
            uid: uid.clone(),
            from_uid,
            to_uid,
            edge_type,
            changed_by: by.to_string(),
        });

        Ok(())
    }

    /// Restore a tombstoned edge.
    pub fn restore_edge(&self, uid: &Uid) -> Result<()> {
        let mut edge = self
            .storage
            .get_edge(uid)?
            .ok_or_else(|| Error::EdgeNotFound(uid.to_string()))?;

        if edge.tombstone_at.is_none() {
            return Ok(());
        }

        edge.tombstone_at = None;
        edge.version += 1;
        edge.updated_at = now();

        self.storage.insert_edge(&edge)?;

        let snapshot = serde_json::to_value(&edge)?;
        self.storage.insert_edge_version(
            &edge.uid,
            edge.version,
            snapshot,
            "system",
            "restore",
            "",
        )?;

        Ok(())
    }

    /// Tombstone a node and all its connected edges.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn tombstone_cascade(&self, uid: &Uid, reason: &str, by: &str) -> Result<TombstoneResult> {
        // First tombstone all connected edges
        let connected_edges = self.storage.query_edges_connected(uid)?;
        let mut edges_tombstoned = 0;
        for edge in &connected_edges {
            self.tombstone_edge(&edge.uid, reason, by)?;
            edges_tombstoned += 1;
        }

        // Then tombstone the node itself
        self.tombstone(uid, reason, by)?;

        Ok(TombstoneResult { edges_tombstoned })
    }

    // ---- Provenance ----

    /// Add a provenance record linking a node to its source.
    pub fn add_provenance(&self, record: &ProvenanceRecord) -> Result<()> {
        self.storage.insert_provenance(record)
    }

    // ---- Entity Resolution ----

    /// Register an alias for an entity.
    pub fn add_alias(&self, alias_text: &str, canonical_uid: &Uid, match_score: f64) -> Result<()> {
        self.storage
            .insert_alias(alias_text, canonical_uid, match_score)
    }

    /// Resolve a text string to a canonical entity UID.
    pub fn resolve_alias(&self, text: &str) -> Result<Option<Uid>> {
        self.storage.resolve_alias(text)
    }

    // ---- Query Patterns ----

    /// Get all active goals, ranked by priority.
    pub fn active_goals(&self) -> Result<Vec<GraphNode>> {
        let mut goals =
            self.storage
                .query_nodes_by_type_and_prop(NodeType::Goal, "status", "active")?;
        goals.sort_by_key(|g| {
            let p = g
                .props
                .to_json()
                .get("priority")
                .and_then(|v| v.as_str())
                .unwrap_or("low")
                .to_string();
            match p.as_str() {
                "critical" => 0,
                "high" => 1,
                "medium" => 2,
                _ => 3,
            }
        });
        Ok(goals)
    }

    /// Get all pending approvals.
    pub fn pending_approvals(&self) -> Result<Vec<GraphNode>> {
        let mut approvals =
            self.storage
                .query_nodes_by_type_and_prop(NodeType::Approval, "status", "pending")?;
        approvals.sort_by(|a, b| {
            let at_a = a
                .props
                .to_json()
                .get("requested_at")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let at_b = b
                .props
                .to_json()
                .get("requested_at")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            at_a.partial_cmp(&at_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(approvals)
    }

    /// Get unresolved contradictions.
    pub fn unresolved_contradictions(&self) -> Result<Vec<Contradiction>> {
        use crate::storage::cozo::extract_string;

        let script = r#"
            ?[edge_uid, from_uid, to_uid, from_label, to_label, description, contradiction_type] :=
                *edge[edge_uid, from_uid, to_uid, edge_type, _, _, _, _, _, _, tombstone_at, props],
                edge_type == 'CONTRADICTS',
                tombstone_at == 0.0,
                description = get(props, 'description', ''),
                contradiction_type = get(props, 'contradiction_type', ''),
                resolution_status = get(props, 'resolution_status', 'unresolved'),
                resolution_status == 'unresolved',
                *node[from_uid, _, _, from_label, _, _, _, _, _, _, _, _, _, _, _, _],
                *node[to_uid, _, _, to_label, _, _, _, _, _, _, _, _, _, _, _, _]
        "#;

        let result = self.storage.run_query(script, BTreeMap::new())?;
        let mut contradictions = Vec::new();
        for row in &result.rows {
            contradictions.push(Contradiction {
                edge_uid: Uid::from(extract_string(&row[0])?.as_str()),
                node_a_uid: Uid::from(extract_string(&row[1])?.as_str()),
                node_b_uid: Uid::from(extract_string(&row[2])?.as_str()),
                node_a_label: extract_string(&row[3])?,
                node_b_label: extract_string(&row[4])?,
                description: {
                    let s = extract_string(&row[5])?;
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                },
                contradiction_type: {
                    let s = extract_string(&row[6])?;
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                },
            });
        }
        Ok(contradictions)
    }

    /// Get open decisions.
    pub fn open_decisions(&self) -> Result<Vec<GraphNode>> {
        self.storage.query_nodes_by_type_and_prop_in(
            NodeType::Decision,
            "status",
            &["open", "deliberating"],
        )
    }

    /// Get open questions.
    pub fn open_questions(&self) -> Result<Vec<GraphNode>> {
        self.storage.query_nodes_by_type_and_prop_in(
            NodeType::OpenQuestion,
            "status",
            &["open", "partially_addressed"],
        )
    }

    /// Get claims with confidence below a threshold, sorted ascending.
    pub fn weak_claims(&self, threshold: f64) -> Result<Vec<GraphNode>> {
        self.storage
            .query_nodes_by_type_below_confidence(NodeType::Claim, threshold)
    }

    /// Get all nodes in a specific layer.
    pub fn nodes_in_layer(&self, layer: Layer) -> Result<Vec<GraphNode>> {
        self.storage.query_nodes_by_layer(layer)
    }

    // ---- Count / Exists ----

    /// Count live nodes of a given type.
    pub fn count_nodes(&self, node_type: NodeType) -> Result<u64> {
        self.storage.count_nodes_by_type(node_type)
    }

    /// Count live nodes in a given layer.
    pub fn count_nodes_in_layer(&self, layer: Layer) -> Result<u64> {
        self.storage.count_nodes_by_layer(layer)
    }

    /// Count live edges of a given type.
    pub fn count_edges(&self, edge_type: EdgeType) -> Result<u64> {
        self.storage.count_edges_by_type(edge_type)
    }

    /// Check if a live (non-tombstoned) node exists.
    pub fn node_exists(&self, uid: &Uid) -> Result<bool> {
        self.storage.node_exists(uid)
    }

    // ---- Traversal ----

    /// Get all nodes reachable from a starting node.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, opts)))]
    pub fn reachable(&self, start: &Uid, opts: &TraversalOptions) -> Result<Vec<PathStep>> {
        self.storage.traverse_reachable(
            start,
            &opts.direction,
            &opts.edge_types,
            opts.max_depth,
            opts.weight_threshold,
        )
    }

    /// Follow a reasoning chain from a claim node through epistemic edges (both directions).
    /// The first element is the starting node at depth 0.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn reasoning_chain(&self, claim_uid: &Uid, max_depth: u32) -> Result<Vec<PathStep>> {
        let start_node = self.get_live_node(claim_uid)?;
        let opts = TraversalOptions {
            direction: Direction::Both,
            edge_types: Some(vec![
                EdgeType::Supports,
                EdgeType::Refutes,
                EdgeType::Justifies,
                EdgeType::HasPremise,
                EdgeType::HasConclusion,
                EdgeType::HasWarrant,
                EdgeType::DerivedFrom,
                EdgeType::ExtractedFrom,
            ]),
            max_depth,
            weight_threshold: None,
        };
        let mut chain = vec![PathStep {
            node_uid: claim_uid.clone(),
            label: start_node.label,
            node_type: start_node.node_type,
            edge_type: None,
            depth: 0,
            parent_uid: None,
        }];
        chain.extend(self.reachable(claim_uid, &opts)?);
        Ok(chain)
    }

    /// Get the neighborhood of a node up to a given depth (both directions).
    pub fn neighborhood(&self, uid: &Uid, depth: u32) -> Result<Vec<PathStep>> {
        let opts = TraversalOptions {
            direction: Direction::Both,
            edge_types: None,
            max_depth: depth,
            weight_threshold: None,
        };
        self.reachable(uid, &opts)
    }

    /// Find a path between two nodes by backtracking parent pointers from target to source.
    /// Returns only the nodes on the actual path (excluding the start node).
    pub fn find_path(
        &self,
        from: &Uid,
        to: &Uid,
        opts: &TraversalOptions,
    ) -> Result<Option<Vec<PathStep>>> {
        let steps = self.reachable(from, opts)?;

        // Check if 'to' is reachable
        if !steps.iter().any(|s| s.node_uid == *to) {
            return Ok(None);
        }

        // Build a lookup map from node_uid -> PathStep
        let step_map: std::collections::HashMap<Uid, &PathStep> =
            steps.iter().map(|s| (s.node_uid.clone(), s)).collect();

        // Backtrack from target to source using parent_uid
        let mut path = Vec::new();
        let mut current = to.clone();
        while current != *from {
            let step = step_map
                .get(&current)
                .ok_or_else(|| Error::NodeNotFound(current.to_string()))?;
            path.push((*step).clone());
            current = step
                .parent_uid
                .clone()
                .ok_or_else(|| Error::Storage("broken parent chain in path".into()))?;
        }

        path.reverse();
        Ok(Some(path))
    }

    /// Extract a subgraph reachable from a starting node.
    pub fn subgraph(
        &self,
        start: &Uid,
        opts: &TraversalOptions,
    ) -> Result<(Vec<GraphNode>, Vec<GraphEdge>)> {
        self.storage.subgraph(
            start,
            &opts.direction,
            &opts.edge_types,
            opts.max_depth,
            opts.weight_threshold,
        )
    }

    // ---- Pagination ----

    /// Get nodes in a layer with pagination.
    pub fn nodes_in_layer_paginated(
        &self,
        layer: Layer,
        page: Pagination,
    ) -> Result<Page<GraphNode>> {
        let (items, has_more) =
            self.storage
                .query_nodes_by_layer_paginated(layer, page.limit, page.offset)?;
        Ok(Page {
            items,
            offset: page.offset,
            limit: page.limit,
            has_more,
        })
    }

    /// Get edges from a node with pagination.
    pub fn edges_from_paginated(
        &self,
        uid: &Uid,
        edge_type: Option<EdgeType>,
        page: Pagination,
    ) -> Result<Page<GraphEdge>> {
        let (items, has_more) =
            self.storage
                .query_edges_from_paginated(uid, edge_type, page.limit, page.offset)?;
        Ok(Page {
            items,
            offset: page.offset,
            limit: page.limit,
            has_more,
        })
    }

    /// Get weak claims with pagination.
    pub fn weak_claims_paginated(
        &self,
        threshold: f64,
        page: Pagination,
    ) -> Result<Page<GraphNode>> {
        let (items, has_more) = self
            .storage
            .query_nodes_by_type_below_confidence_paginated(
                NodeType::Claim,
                threshold,
                page.limit,
                page.offset,
            )?;
        Ok(Page {
            items,
            offset: page.offset,
            limit: page.limit,
            has_more,
        })
    }

    /// Get active goals with pagination, sorted by priority in the database.
    pub fn active_goals_paginated(&self, page: Pagination) -> Result<Page<GraphNode>> {
        let (items, has_more) = self
            .storage
            .query_active_goals_paginated(page.limit, page.offset)?;
        Ok(Page {
            items,
            offset: page.offset,
            limit: page.limit,
            has_more,
        })
    }

    /// Get edges to a node with pagination.
    pub fn edges_to_paginated(
        &self,
        uid: &Uid,
        edge_type: Option<EdgeType>,
        page: Pagination,
    ) -> Result<Page<GraphEdge>> {
        let (items, has_more) =
            self.storage
                .query_edges_to_paginated(uid, edge_type, page.limit, page.offset)?;
        Ok(Page {
            items,
            offset: page.offset,
            limit: page.limit,
            has_more,
        })
    }

    // ---- Batch Operations ----

    /// Add multiple nodes in a single batch operation.
    pub fn add_nodes_batch(&self, creates: Vec<CreateNode>) -> Result<Vec<GraphNode>> {
        let mut nodes = Vec::with_capacity(creates.len());

        for create in creates {
            let layer = create.props.layer();
            let node_type = create.props.node_type();
            let ts = now();

            let node = GraphNode {
                uid: create.uid.unwrap_or_default(),
                node_type,
                layer,
                label: create.label,
                summary: create.summary,
                created_at: ts,
                updated_at: ts,
                version: 1,
                confidence: create.confidence,
                salience: create.salience,
                privacy_level: create.privacy_level,
                embedding_ref: None,
                tombstone_at: None,
                tombstone_reason: None,
                tombstone_by: None,
                props: create.props,
            };

            nodes.push(node);
        }

        self.storage.insert_nodes_batch(&nodes)?;

        let agent = self.default_agent();
        for node in &nodes {
            let snapshot = serde_json::to_value(node)?;
            self.storage
                .insert_node_version(&node.uid, 1, snapshot, &agent, "create", "")?;
        }

        Ok(nodes)
    }

    /// Add multiple edges in a single batch operation.
    pub fn add_edges_batch(&self, creates: Vec<CreateEdge>) -> Result<Vec<GraphEdge>> {
        use std::collections::HashMap;

        // Single pass: collect unique from-node UIDs and validate all endpoints
        let mut from_node_cache: HashMap<Uid, GraphNode> = HashMap::new();
        for create in &creates {
            // Validate and cache from-nodes
            if !from_node_cache.contains_key(&create.from_uid) {
                let node = self.get_live_node(&create.from_uid)?;
                from_node_cache.insert(create.from_uid.clone(), node);
            }
            // Validate to-nodes exist and are live
            if !from_node_cache.contains_key(&create.to_uid) {
                self.get_live_node(&create.to_uid)?;
            }
        }

        let mut edges = Vec::with_capacity(creates.len());
        for create in creates {
            let edge_type = create.props.edge_type();
            let layer = from_node_cache[&create.from_uid].layer;
            let ts = now();

            let edge = GraphEdge {
                uid: Uid::new(),
                from_uid: create.from_uid,
                to_uid: create.to_uid,
                edge_type,
                layer,
                created_at: ts,
                updated_at: ts,
                version: 1,
                confidence: create.confidence,
                weight: create.weight,
                tombstone_at: None,
                props: create.props,
            };

            edges.push(edge);
        }

        self.storage.insert_edges_batch(&edges)?;

        let agent = self.default_agent();
        for edge in &edges {
            let snapshot = serde_json::to_value(edge)?;
            self.storage
                .insert_edge_version(&edge.uid, 1, snapshot, &agent, "create", "")?;
        }

        Ok(edges)
    }

    // ---- Version History ----

    /// Get the full version history for a node.
    pub fn node_history(&self, uid: &Uid) -> Result<Vec<VersionRecord>> {
        self.storage.node_versions(uid)
    }

    /// Get the full version history for an edge.
    pub fn edge_history(&self, uid: &Uid) -> Result<Vec<VersionRecord>> {
        self.storage.edge_versions(uid)
    }

    /// Get a node's snapshot at a specific version.
    pub fn node_at_version(&self, uid: &Uid, version: i64) -> Result<Option<serde_json::Value>> {
        self.storage.node_at_version(uid, version)
    }

    // ---- Full-Text Search ----

    /// Search nodes by text query using full-text search indices.
    ///
    /// # Examples
    ///
    /// ```
    /// use mindgraph::*;
    /// let graph = MindGraph::open_in_memory().unwrap();
    /// graph.add_claim("Rust is fast", "native compilation", 0.9).unwrap();
    /// let results = graph.search("fast", &SearchOptions::new()).unwrap();
    /// assert_eq!(results.len(), 1);
    /// ```
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, opts)))]
    pub fn search(&self, query: &str, opts: &SearchOptions) -> Result<Vec<SearchResult>> {
        self.storage.query_fts_search(query, opts)
    }

    /// Hybrid search combining FTS (BM25) and vector similarity using
    /// Reciprocal Rank Fusion (RRF).
    ///
    /// Runs both FTS and semantic search, then fuses results with:
    /// `score = sum(1 / (k + rank_i))` where k=60 (standard RRF constant).
    ///
    /// Returns up to `limit` results sorted by fused score. Falls back to
    /// FTS-only if embeddings are not configured.
    pub fn hybrid_search(
        &self,
        query: &str,
        query_vec: Option<&[f32]>,
        limit: usize,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        const RRF_K: f64 = 60.0;

        // FTS results
        let fts_opts = SearchOptions {
            limit: Some((limit * 3) as u32),
            ..opts.clone()
        };
        let fts_results = self.search(query, &fts_opts)?;

        // Vector results (if available)
        let vec_results = if let Some(qv) = query_vec {
            if self.embedding_dimension().is_some() {
                self.semantic_search(qv, limit * 3).ok().unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // If no vector results, just return FTS
        if vec_results.is_empty() {
            let mut results = fts_results;
            results.truncate(limit);
            return Ok(results);
        }

        // Build RRF scores
        let mut scores: HashMap<Uid, (f64, Option<GraphNode>)> = HashMap::new();

        for (rank, sr) in fts_results.iter().enumerate() {
            let rrf = 1.0 / (RRF_K + rank as f64 + 1.0);
            let entry = scores
                .entry(sr.node.uid.clone())
                .or_insert((0.0, Some(sr.node.clone())));
            entry.0 += rrf;
        }

        for (rank, (node, _dist)) in vec_results.iter().enumerate() {
            let rrf = 1.0 / (RRF_K + rank as f64 + 1.0);
            let entry = scores
                .entry(node.uid.clone())
                .or_insert((0.0, Some(node.clone())));
            entry.0 += rrf;
        }

        let mut fused: Vec<SearchResult> = scores
            .into_values()
            .filter_map(|(score, node)| node.map(|n| SearchResult { node: n, score }))
            .collect();
        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        fused.truncate(limit);

        Ok(fused)
    }

    // ---- Structured Filtering ----

    /// Find nodes matching structured filter criteria.
    ///
    /// # Examples
    ///
    /// ```
    /// use mindgraph::*;
    /// let graph = MindGraph::open_in_memory().unwrap();
    /// graph.add_entity("Alice", "person").unwrap();
    /// graph.add_entity("Bob", "person").unwrap();
    /// graph.add_claim("Sky is blue", "color", 0.9).unwrap();
    /// let filter = NodeFilter { node_type: Some(NodeType::Entity), ..Default::default() };
    /// let results = graph.find_nodes(&filter).unwrap();
    /// assert_eq!(results.len(), 2);
    /// ```
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, filter)))]
    pub fn find_nodes(&self, filter: &NodeFilter) -> Result<Vec<GraphNode>> {
        let mut results = {
            let (nodes, _has_more) = self.storage.query_nodes_filtered(filter)?;
            nodes
        };

        // Handle connected_to: post-filter to only nodes connected to the target
        if let Some(ref target_uid) = filter.connected_to {
            let connected = self.storage.query_connected_uids(target_uid)?;
            let connected_set: std::collections::HashSet<_> = connected.into_iter().collect();
            results.retain(|n| connected_set.contains(&n.uid));
        }

        // Handle OR filters: union results from sub-filters
        if let Some(ref or_filters) = filter.or_filters {
            let mut uid_set: std::collections::HashSet<Uid> =
                results.iter().map(|n| n.uid.clone()).collect();
            for sub in or_filters {
                let sub_results = self.find_nodes(sub)?;
                for node in sub_results {
                    if uid_set.insert(node.uid.clone()) {
                        results.push(node);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Find nodes matching structured filter criteria, with pagination metadata.
    pub fn find_nodes_paginated(&self, filter: &NodeFilter) -> Result<Page<GraphNode>> {
        let items = self.find_nodes(filter)?;
        let limit = filter.limit.unwrap_or(100) as usize;
        let has_more = items.len() > limit;
        let items = if has_more {
            items[..limit].to_vec()
        } else {
            items
        };
        Ok(Page {
            items,
            offset: filter.offset.unwrap_or(0),
            limit: filter.limit.unwrap_or(100),
            has_more,
        })
    }

    // ---- Data Lifecycle ----

    /// Permanently delete tombstoned nodes, edges, and their associated data.
    /// If `older_than` is None, purge all tombstoned data.
    pub fn purge_tombstoned(&self, older_than: Option<Timestamp>) -> Result<PurgeResult> {
        self.storage.purge_tombstoned(older_than)
    }

    /// Export the entire graph as a snapshot.
    pub fn export(&self) -> Result<GraphSnapshot> {
        let relations = self.storage.export_all()?;
        Ok(GraphSnapshot {
            relations,
            exported_at: now(),
            mindgraph_version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    /// Import a graph snapshot (additive merge).
    pub fn import(&self, snapshot: &GraphSnapshot) -> Result<ImportResult> {
        let count = self.storage.import_snapshot(&snapshot.relations)?;
        Ok(ImportResult {
            relations_imported: count,
        })
    }

    /// Backup the database to a file.
    pub fn backup(&self, path: impl AsRef<Path>) -> Result<()> {
        self.storage.backup(path.as_ref())
    }

    /// Restore the database from a backup file.
    pub fn restore_backup(&self, path: impl AsRef<Path>) -> Result<()> {
        self.storage.restore(path.as_ref())
    }

    // ---- Entity Resolution (extended) ----

    /// Get all aliases registered for a canonical entity UID.
    pub fn aliases_for(&self, uid: &Uid) -> Result<Vec<(String, f64)>> {
        self.storage.aliases_for(uid)
    }

    /// Merge two entities: retarget all edges and aliases from `merge_uid` to `keep_uid`,
    /// then tombstone the merged entity.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn merge_entities(
        &self,
        keep_uid: &Uid,
        merge_uid: &Uid,
        reason: &str,
        by: &str,
    ) -> Result<MergeResult> {
        // Validate both are live
        let merge_node = self.get_live_node(merge_uid)?;
        self.get_live_node(keep_uid)?;

        // Retarget edges
        let edges_retargeted = self.storage.retarget_edges(merge_uid, keep_uid)?;

        // Retarget aliases
        let aliases_merged = self.storage.retarget_aliases(merge_uid, keep_uid)?;

        // Add the merged node's label as an alias for the keep node
        self.add_alias(&merge_node.label, keep_uid, 0.8)?;

        // Tombstone the merged node
        self.tombstone(merge_uid, reason, by)?;

        Ok(MergeResult {
            edges_retargeted,
            aliases_merged,
        })
    }

    /// Fuzzy-match text against registered aliases.
    pub fn fuzzy_resolve(&self, text: &str, limit: u32) -> Result<Vec<(Uid, f64)>> {
        self.storage.fuzzy_resolve_alias(text, limit)
    }

    // ---- Batch Operations (GraphOp) ----

    /// Execute a batch of graph operations atomically.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, ops)))]
    pub fn batch_apply(&self, ops: Vec<GraphOp>) -> Result<BatchResult> {
        let mut result = BatchResult {
            nodes_added: 0,
            edges_added: 0,
            nodes_tombstoned: 0,
            edges_tombstoned: 0,
        };

        for op in ops {
            match op {
                GraphOp::AddNode(create) => {
                    self.add_node(*create)?;
                    result.nodes_added += 1;
                }
                GraphOp::AddEdge(create) => {
                    self.add_edge(*create)?;
                    result.edges_added += 1;
                }
                GraphOp::Tombstone { uid, reason, by } => {
                    self.tombstone(&uid, &reason, &by)?;
                    result.nodes_tombstoned += 1;
                }
                GraphOp::TombstoneEdge { uid, reason, by } => {
                    self.tombstone_edge(&uid, &reason, &by)?;
                    result.edges_tombstoned += 1;
                }
            }
        }

        Ok(result)
    }

    // ---- v0.4: Stats ----

    /// Get graph-wide statistics.
    ///
    /// # Examples
    ///
    /// ```
    /// use mindgraph::*;
    /// let graph = MindGraph::open_in_memory().unwrap();
    /// graph.add_entity("A", "test").unwrap();
    /// graph.add_entity("B", "test").unwrap();
    /// let stats = graph.stats().unwrap();
    /// assert_eq!(stats.total_nodes, 2);
    /// ```
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn stats(&self) -> Result<GraphStats> {
        let dim = self.embedding_dimension();
        self.storage.query_stats(dim)
    }

    // ---- v0.4: Convenience Constructors ----

    /// Add a claim node with content and confidence.
    ///
    /// # Examples
    ///
    /// ```
    /// use mindgraph::*;
    /// let graph = MindGraph::open_in_memory().unwrap();
    /// let claim = graph.add_claim("Earth is round", "spherical shape", 0.99).unwrap();
    /// assert_eq!(claim.node_type, NodeType::Claim);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use mindgraph::*;
    /// let graph = MindGraph::open_in_memory().unwrap();
    /// let entity = graph.add_entity("Rust", "language").unwrap();
    /// assert_eq!(entity.node_type, NodeType::Entity);
    /// assert_eq!(entity.label, "Rust");
    /// ```
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

    /// Find an existing entity by alias match, or create a new one.
    ///
    /// 1. Checks exact alias match for the label (case-sensitive).
    /// 2. If found and the entity is live, returns it.
    /// 3. Otherwise, creates a new Entity node and registers the label as an alias.
    ///
    /// Returns `(node, created)` — `created` is `true` if a new entity was made.
    pub fn find_or_create_entity(
        &self,
        label: &str,
        entity_type: &str,
    ) -> Result<(GraphNode, bool)> {
        // Try exact alias resolution first
        if let Some(uid) = self.resolve_alias(label)? {
            if let Some(node) = self.storage.get_node(&uid)? {
                if node.tombstone_at.is_none() && node.node_type == NodeType::Entity {
                    return Ok((node, false));
                }
            }
        }

        // Also try case-insensitive label search for Entity nodes
        let lower = label.to_lowercase();
        let filter = NodeFilter {
            node_type: Some(NodeType::Entity),
            ..Default::default()
        };
        let existing = self.find_nodes(&filter)?;
        for node in &existing {
            if node.label.to_lowercase() == lower && node.tombstone_at.is_none() {
                // Register the alias so future lookups are fast
                self.add_alias(label, &node.uid, 1.0)?;
                return Ok((node.clone(), false));
            }
        }

        // No match — create new entity
        let node = self.add_entity(label, entity_type)?;
        // Register the label as an alias for this entity
        self.add_alias(label, &node.uid, 1.0)?;
        Ok((node, true))
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

    /// Add a memory (session) node.
    #[deprecated(since = "0.4.1", note = "Use add_session() instead")]
    pub fn add_memory(&self, label: &str, content: &str) -> Result<GraphNode> {
        self.add_session(label, content)
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

    /// Add a node with a user-defined custom type.
    ///
    /// # Example
    /// ```rust
    /// use serde::{Serialize, Deserialize};
    /// use mindgraph::*;
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct CodeSnippet { language: String, code: String }
    ///
    /// impl CustomNodeType for CodeSnippet {
    ///     fn type_name() -> &'static str { "CodeSnippet" }
    ///     fn layer() -> Layer { Layer::Reality }
    /// }
    ///
    /// let graph = MindGraph::open_in_memory().unwrap();
    /// let node = graph.add_custom_node("hello.rs", CodeSnippet {
    ///     language: "rust".into(),
    ///     code: "fn main() {}".into(),
    /// }).unwrap();
    /// assert_eq!(node.node_type, NodeType::Custom("CodeSnippet".into()));
    /// let props: CodeSnippet = node.custom_props().unwrap().unwrap();
    /// assert_eq!(props.language, "rust");
    /// ```
    pub fn add_custom_node<T: crate::schema::CustomNodeType>(
        &self,
        label: &str,
        props: T,
    ) -> Result<GraphNode> {
        let data = serde_json::to_value(&props)?;
        self.add_node(CreateNode::new(
            label,
            NodeProps::Custom {
                type_name: T::type_name().to_string(),
                layer: T::layer(),
                data,
            },
        ))
    }

    /// Add a link (edge) between two nodes with default props for the given type.
    pub fn add_link(&self, from: &Uid, to: &Uid, edge_type: EdgeType) -> Result<GraphEdge> {
        self.add_edge(CreateEdge::new(
            from.clone(),
            to.clone(),
            EdgeProps::default_for(edge_type),
        ))
    }

    // ---- v0.4: Embeddings ----

    /// Configure embedding support with the given vector dimension.
    /// Idempotent if the same dimension is used; errors on dimension mismatch.
    /// Drop existing embedding schema and reset dimension. Used before reconfigure.
    pub fn clear_embeddings(&self) -> Result<()> {
        self.storage.drop_embedding_schema()?;
        *self
            .embedding_dim
            .write()
            .unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }

    pub fn configure_embeddings(&self, dimension: usize) -> Result<()> {
        let current = self.embedding_dimension();
        if let Some(existing) = current {
            if existing == dimension {
                return Ok(());
            }
            return Err(Error::EmbeddingDimensionMismatch {
                expected: existing,
                got: dimension,
            });
        }
        self.storage.create_embedding_schema(dimension)?;
        *self
            .embedding_dim
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(dimension);
        Ok(())
    }

    /// Get the configured embedding dimension, if any.
    pub fn embedding_dimension(&self) -> Option<usize> {
        *self.embedding_dim.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Set the embedding vector for a node.
    pub fn set_embedding(&self, uid: &Uid, embedding: &[f32]) -> Result<()> {
        let dim = self
            .embedding_dimension()
            .ok_or(Error::EmbeddingNotConfigured)?;
        if embedding.len() != dim {
            return Err(Error::EmbeddingDimensionMismatch {
                expected: dim,
                got: embedding.len(),
            });
        }
        self.storage.upsert_embedding(uid, embedding)
    }

    /// Get the embedding vector for a node.
    pub fn get_embedding(&self, uid: &Uid) -> Result<Option<Vec<f32>>> {
        if self.embedding_dimension().is_none() {
            return Err(Error::EmbeddingNotConfigured);
        }
        self.storage.get_embedding(uid)
    }

    /// Delete the embedding for a node.
    pub fn delete_embedding(&self, uid: &Uid) -> Result<()> {
        if self.embedding_dimension().is_none() {
            return Err(Error::EmbeddingNotConfigured);
        }
        self.storage.delete_embedding(uid)
    }

    /// Search for semantically similar nodes using a query vector.
    /// Returns (node, distance) pairs sorted by distance ascending.
    /// Automatically over-fetches and retries to compensate for tombstoned nodes.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, query_vec)))]
    pub fn semantic_search(&self, query_vec: &[f32], k: usize) -> Result<Vec<(GraphNode, f64)>> {
        if self.embedding_dimension().is_none() {
            return Err(Error::EmbeddingNotConfigured);
        }

        let max_fetch = k * 10;
        let mut fetch_size = k * 2;
        let mut results = Vec::new();

        loop {
            let raw = self
                .storage
                .semantic_search_raw(query_vec, fetch_size, fetch_size)?;
            let raw_len = raw.len();
            results.clear();
            for (uid, dist) in raw {
                if let Some(node) = self.storage.get_node(&uid)? {
                    if node.tombstone_at.is_none() {
                        results.push((node, dist));
                        if results.len() >= k {
                            break;
                        }
                    }
                }
            }

            // If we have enough results, or HNSW didn't return a full batch, stop
            if results.len() >= k || raw_len < fetch_size || fetch_size >= max_fetch {
                break;
            }

            fetch_size = (fetch_size * 2).min(max_fetch);
        }

        results.truncate(k);
        Ok(results)
    }

    /// Embed a node's label+summary text using the given provider.
    pub fn embed_node(&self, uid: &Uid, provider: &dyn EmbeddingProvider) -> Result<()> {
        let node = self.get_live_node(uid)?;
        let text = if node.summary.is_empty() {
            node.label.clone()
        } else {
            format!("{} {}", node.label, node.summary)
        };
        let embedding = provider.embed(&text)?;
        self.set_embedding(uid, &embedding)
    }

    /// Embed multiple nodes' label+summary text using the given provider.
    /// Skips tombstoned nodes. Returns the count of nodes successfully embedded.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, uids, provider)))]
    pub fn embed_nodes(&self, uids: &[Uid], provider: &dyn EmbeddingProvider) -> Result<usize> {
        let mut texts = Vec::new();
        let mut live_uids = Vec::new();

        for uid in uids {
            if let Some(node) = self.storage.get_node(uid)? {
                if node.tombstone_at.is_none() {
                    let text = if node.summary.is_empty() {
                        node.label.clone()
                    } else {
                        format!("{} {}", node.label, node.summary)
                    };
                    texts.push(text);
                    live_uids.push(uid.clone());
                }
            }
        }

        if texts.is_empty() {
            return Ok(0);
        }

        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let embeddings = provider.embed_batch(&text_refs)?;

        for (uid, embedding) in live_uids.iter().zip(embeddings.iter()) {
            self.set_embedding(uid, embedding)?;
        }

        Ok(live_uids.len())
    }

    /// Search by text using the given provider to embed the query.
    pub fn semantic_search_text(
        &self,
        query: &str,
        k: usize,
        provider: &dyn EmbeddingProvider,
    ) -> Result<Vec<(GraphNode, f64)>> {
        let query_vec = provider.embed(query)?;
        self.semantic_search(&query_vec, k)
    }

    // ---- v0.4: Salience Decay ----

    /// Apply exponential salience decay to all live nodes.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn decay_salience(&self, half_life_secs: f64) -> Result<DecayResult> {
        let current_time = now();
        let changed = self
            .storage
            .apply_salience_decay(half_life_secs, current_time)?;

        let below_threshold = changed.iter().filter(|(_, _, new_s)| *new_s < 0.1).count();

        Ok(DecayResult {
            nodes_decayed: changed.len(),
            below_threshold,
        })
    }

    /// Auto-tombstone nodes with salience below a threshold and older than min_age_secs.
    pub fn auto_tombstone(&self, min_salience: f64, min_age_secs: f64) -> Result<usize> {
        let cutoff = now() - min_age_secs;
        let uids = self
            .storage
            .query_low_salience_old_nodes(min_salience, cutoff)?;
        let count = uids.len();
        for uid in &uids {
            self.tombstone(uid, "auto_tombstone: low salience", "system")?;
        }
        Ok(count)
    }

    // ---- v0.4: Subscription / Events ----

    /// Register a callback for graph change events.
    pub fn on_change(&self, cb: impl Fn(&GraphEvent) + Send + Sync + 'static) -> SubscriptionId {
        let id = self.next_sub_id.fetch_add(1, Ordering::Relaxed);
        self.subscribers
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, Arc::new(cb));
        id
    }

    /// Remove a subscription.
    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.subscribers
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
    }

    /// Register a filtered callback for graph change events.
    /// Only events matching the filter will be delivered.
    pub fn on_change_filtered(
        &self,
        filter: EventFilter,
        cb: impl Fn(&GraphEvent) + Send + Sync + 'static,
    ) -> SubscriptionId {
        self.on_change(move |event| {
            if filter.matches(event) {
                cb(event);
            }
        })
    }

    /// Create an async filtered event stream. Requires the `async` feature.
    #[cfg(feature = "async")]
    pub fn watch(&self, filter: EventFilter) -> crate::watch::WatchStream {
        let rx = self.broadcast_tx.subscribe();
        crate::watch::WatchStream::new(rx, filter)
    }

    /// Emit a graph event to all subscribers.
    fn emit(&self, event: GraphEvent) {
        let subs = {
            let subs_lock = self.subscribers.read().unwrap_or_else(|e| e.into_inner());
            subs_lock.values().cloned().collect::<Vec<_>>()
        };

        for cb in subs {
            cb(&event);
        }

        #[cfg(feature = "async")]
        {
            // Ignore send error (no receivers)
            let _ = self.broadcast_tx.send(event);
        }
    }

    // ---- v0.4: Typed Export / Import ----

    /// Export all live nodes and edges as a typed snapshot, including embeddings.
    pub fn export_typed(&self) -> Result<TypedSnapshot> {
        let nodes = self.storage.export_all_live_nodes()?;
        let edges = self.storage.export_all_live_edges()?;
        let embeddings = self.storage.export_all_embeddings()?;
        Ok(TypedSnapshot {
            nodes,
            edges,
            embeddings,
            exported_at: now(),
            mindgraph_version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    /// Import a typed snapshot (additive merge, skips existing UIDs).
    pub fn import_typed(&self, snapshot: &TypedSnapshot) -> Result<TypedImportResult> {
        let mut nodes_imported = 0;
        let mut nodes_skipped = 0;
        let mut edges_imported = 0;
        let mut edges_skipped = 0;
        let mut embeddings_imported = 0;

        for node in &snapshot.nodes {
            if self.storage.get_node(&node.uid)?.is_some() {
                nodes_skipped += 1;
            } else {
                self.storage.insert_node(node)?;
                nodes_imported += 1;
            }
        }

        for edge in &snapshot.edges {
            if self.storage.get_edge(&edge.uid)?.is_some() {
                edges_skipped += 1;
            } else {
                self.storage.insert_edge(edge)?;
                edges_imported += 1;
            }
        }

        // Import embeddings if present and dimension matches
        if !snapshot.embeddings.is_empty() {
            if let Some(dim) = self.embedding_dimension() {
                for (uid, vec) in &snapshot.embeddings {
                    if vec.len() == dim {
                        self.storage.upsert_embedding(uid, vec)?;
                        embeddings_imported += 1;
                    }
                }
            }
        }

        Ok(TypedImportResult {
            nodes_imported,
            edges_imported,
            nodes_skipped,
            edges_skipped,
            embeddings_imported,
        })
    }

    // ---- v0.4: Validated Batch ----

    /// Pre-validate a batch of operations without executing them.
    /// `CreateNode` ops without a pre-assigned UID will have one assigned automatically.
    pub fn validate_batch(&self, ops: Vec<GraphOp>) -> Result<ValidatedBatch> {
        let mut batch = ValidatedBatch {
            nodes_to_add: Vec::new(),
            edges_to_add: Vec::new(),
            tombstone_nodes: Vec::new(),
            tombstone_edges: Vec::new(),
        };

        // Track UIDs that will be added in this batch
        let mut pending_node_uids: std::collections::HashSet<Uid> =
            std::collections::HashSet::new();

        for op in ops {
            match op {
                GraphOp::AddNode(mut create) => {
                    // Assign a UID if not already set, so edges can reference it
                    if create.uid.is_none() {
                        create.uid = Some(Uid::new());
                    }
                    pending_node_uids.insert(create.uid.clone().unwrap());
                    batch.nodes_to_add.push(*create);
                }
                GraphOp::AddEdge(create) => {
                    // Validate endpoints exist (or will be created in batch)
                    let from_exists = self.storage.node_exists(&create.from_uid)?
                        || pending_node_uids.contains(&create.from_uid);
                    let to_exists = self.storage.node_exists(&create.to_uid)?
                        || pending_node_uids.contains(&create.to_uid);
                    if !from_exists {
                        return Err(Error::NodeNotFound(create.from_uid.to_string()));
                    }
                    if !to_exists {
                        return Err(Error::NodeNotFound(create.to_uid.to_string()));
                    }
                    batch.edges_to_add.push(*create);
                }
                GraphOp::Tombstone { uid, reason, by } => {
                    // Validate node exists and is live
                    self.get_live_node(&uid)?;
                    batch.tombstone_nodes.push((uid, reason, by));
                }
                GraphOp::TombstoneEdge { uid, reason, by } => {
                    self.get_live_edge(&uid)?;
                    batch.tombstone_edges.push((uid, reason, by));
                }
            }
        }

        Ok(batch)
    }

    /// Apply a pre-validated batch.
    pub fn apply_validated_batch(&self, batch: ValidatedBatch) -> Result<BatchResult> {
        let mut result = BatchResult {
            nodes_added: 0,
            edges_added: 0,
            nodes_tombstoned: 0,
            edges_tombstoned: 0,
        };

        // Add nodes
        if !batch.nodes_to_add.is_empty() {
            let nodes = self.add_nodes_batch(batch.nodes_to_add)?;
            result.nodes_added = nodes.len();
        }

        // Add edges
        if !batch.edges_to_add.is_empty() {
            let edges = self.add_edges_batch(batch.edges_to_add)?;
            result.edges_added = edges.len();
        }

        // Tombstone nodes
        for (uid, reason, by) in batch.tombstone_nodes {
            self.tombstone(&uid, &reason, &by)?;
            result.nodes_tombstoned += 1;
        }

        // Tombstone edges
        for (uid, reason, by) in batch.tombstone_edges {
            self.tombstone_edge(&uid, &reason, &by)?;
            result.edges_tombstoned += 1;
        }

        Ok(result)
    }

    // ---- v0.5: New Methods ----

    /// Get live edges between two specific nodes, optionally filtered by edge type.
    pub fn get_edge_between(
        &self,
        from_uid: &Uid,
        to_uid: &Uid,
        edge_type: Option<EdgeType>,
    ) -> Result<Vec<GraphEdge>> {
        self.storage
            .query_edges_between(from_uid, to_uid, edge_type)
    }

    /// List all live nodes with pagination.
    pub fn list_nodes(&self, page: Pagination) -> Result<Page<GraphNode>> {
        let (items, has_more) = self
            .storage
            .query_all_live_nodes_paginated(page.limit, page.offset)?;
        Ok(Page {
            items,
            offset: page.offset,
            limit: page.limit,
            has_more,
        })
    }

    // ---- v0.6: Multi-Agent ----

    /// Create a scoped agent handle bound to this graph.
    /// Requires the graph to be wrapped in an `Arc`.
    pub fn agent(self: &Arc<Self>, name: impl Into<String>) -> crate::agent::AgentHandle {
        crate::agent::AgentHandle::new(self.clone(), name.into(), None)
    }

    /// Add a node with an explicit agent identity in the version record.
    pub fn add_node_as(&self, create: CreateNode, changed_by: &str) -> Result<GraphNode> {
        let layer = create.props.layer();
        let node_type = create.props.node_type();
        let ts = now();

        let node = GraphNode {
            uid: create.uid.unwrap_or_default(),
            node_type,
            layer,
            label: create.label,
            summary: create.summary,
            created_at: ts,
            updated_at: ts,
            version: 1,
            confidence: create.confidence,
            salience: create.salience,
            privacy_level: create.privacy_level,
            embedding_ref: None,
            tombstone_at: None,
            tombstone_reason: None,
            tombstone_by: None,
            props: create.props,
        };

        self.storage.insert_node(&node)?;

        let snapshot = serde_json::to_value(&node)?;
        self.storage
            .insert_node_version(&node.uid, 1, snapshot, changed_by, "create", "")?;

        self.emit(GraphEvent::NodeAdded {
            node: Box::new(node.clone()),
            changed_by: changed_by.to_string(),
        });

        Ok(node)
    }

    /// Add an edge with an explicit agent identity in the version record.
    pub fn add_edge_as(&self, create: CreateEdge, changed_by: &str) -> Result<GraphEdge> {
        let edge_type = create.props.edge_type();
        let ts = now();

        let from_node = self.get_live_node(&create.from_uid)?;
        self.get_live_node(&create.to_uid)?;
        let layer = from_node.layer;

        let edge = GraphEdge {
            uid: Uid::new(),
            from_uid: create.from_uid,
            to_uid: create.to_uid,
            edge_type,
            layer,
            created_at: ts,
            updated_at: ts,
            version: 1,
            confidence: create.confidence,
            weight: create.weight,
            tombstone_at: None,
            props: create.props,
        };

        self.storage.insert_edge(&edge)?;

        let snapshot = serde_json::to_value(&edge)?;
        self.storage
            .insert_edge_version(&edge.uid, 1, snapshot, changed_by, "create", "")?;

        self.emit(GraphEvent::EdgeAdded {
            edge: Box::new(edge.clone()),
            changed_by: changed_by.to_string(),
        });

        Ok(edge)
    }

    /// Update an edge with an explicit agent identity in the version record.
    pub fn update_edge_as(
        &self,
        uid: &Uid,
        confidence: Option<Confidence>,
        weight: Option<f64>,
        props: Option<EdgeProps>,
        changed_by: &str,
        reason: &str,
    ) -> Result<GraphEdge> {
        self.update_edge(uid, confidence, weight, props, changed_by, reason)
    }

    /// Tombstone-cascade with an explicit agent identity.
    pub fn tombstone_cascade_as(
        &self,
        uid: &Uid,
        reason: &str,
        by: &str,
    ) -> Result<TombstoneResult> {
        let connected_edges = self.storage.query_edges_connected(uid)?;
        let mut edges_tombstoned = 0;
        for edge in &connected_edges {
            self.tombstone_edge(&edge.uid, reason, by)?;
            edges_tombstoned += 1;
        }
        self.tombstone(uid, reason, by)?;
        Ok(TombstoneResult { edges_tombstoned })
    }

    /// Get all live nodes created by a specific agent.
    pub fn nodes_by_agent(&self, agent_id: &str) -> Result<Vec<GraphNode>> {
        self.storage.query_nodes_by_agent(agent_id)
    }

    /// Delete all data from the graph. **Destructive operation** intended for testing and reset.
    pub fn clear(&self) -> Result<()> {
        self.storage.clear_all()?;
        *self
            .embedding_dim
            .write()
            .unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }
}

/// Builder for ergonomic node updates.
pub struct NodeUpdate<'a> {
    graph: &'a MindGraph,
    uid: &'a Uid,
    label: Option<String>,
    summary: Option<String>,
    confidence: Option<Confidence>,
    salience: Option<Salience>,
    props: Option<NodeProps>,
    changed_by: Option<String>,
    reason: Option<String>,
}

impl<'a> NodeUpdate<'a> {
    /// Set the new label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the new summary.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Set the new confidence.
    pub fn confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = Some(confidence);
        self
    }

    /// Set the new salience.
    pub fn salience(mut self, salience: Salience) -> Self {
        self.salience = Some(salience);
        self
    }

    /// Set the new props.
    pub fn props(mut self, props: NodeProps) -> Self {
        self.props = Some(props);
        self
    }

    /// Set who is making this change.
    pub fn changed_by(mut self, by: impl Into<String>) -> Self {
        self.changed_by = Some(by.into());
        self
    }

    /// Set the reason for this change.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Apply the update and return the updated node.
    pub fn apply(self) -> Result<GraphNode> {
        let default_agent = self.graph.default_agent();
        self.graph.update_node(
            self.uid,
            self.label,
            self.summary,
            self.confidence,
            self.salience,
            self.props,
            self.changed_by.as_deref().unwrap_or(&default_agent),
            self.reason.as_deref().unwrap_or(""),
        )
    }
}

/// Builder for ergonomic edge updates.
pub struct EdgeUpdate<'a> {
    graph: &'a MindGraph,
    uid: &'a Uid,
    confidence: Option<Confidence>,
    weight: Option<f64>,
    props: Option<EdgeProps>,
    changed_by: Option<String>,
    reason: Option<String>,
}

impl<'a> EdgeUpdate<'a> {
    /// Set the new confidence.
    pub fn confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = Some(confidence);
        self
    }

    /// Set the new weight.
    pub fn weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Set the new props.
    pub fn props(mut self, props: EdgeProps) -> Self {
        self.props = Some(props);
        self
    }

    /// Set who is making this change.
    pub fn changed_by(mut self, by: impl Into<String>) -> Self {
        self.changed_by = Some(by.into());
        self
    }

    /// Set the reason for this change.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Apply the update and return the updated edge.
    pub fn apply(self) -> Result<GraphEdge> {
        let default_agent = self.graph.default_agent();
        self.graph.update_edge(
            self.uid,
            self.confidence,
            self.weight,
            self.props,
            self.changed_by.as_deref().unwrap_or(&default_agent),
            self.reason.as_deref().unwrap_or(""),
        )
    }
}
