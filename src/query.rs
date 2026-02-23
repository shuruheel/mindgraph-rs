use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::schema::edge::{CreateEdge, GraphEdge};
use crate::schema::node::{CreateNode, GraphNode};
use crate::schema::{Layer, NodeType};
use crate::types::{Timestamp, Uid};

/// A contradiction found between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub edge_uid: Uid,
    pub node_a_uid: Uid,
    pub node_a_label: String,
    pub node_b_uid: Uid,
    pub node_b_label: String,
    pub description: Option<String>,
    pub contradiction_type: Option<String>,
}

/// Result from semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub node: GraphNode,
    pub score: f64,
}

/// A weak claim: a claim with low confidence that informs an active decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeakClaim {
    pub claim: GraphNode,
    pub decision_uid: Uid,
    pub decision_label: String,
}

/// Pagination parameters.
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub limit: u32,
    pub offset: u32,
}

impl Pagination {
    /// Create pagination requesting the first `n` results.
    pub fn first(n: u32) -> Self {
        Pagination { limit: n, offset: 0 }
    }
}

/// A paginated result set.
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub offset: u32,
    pub limit: u32,
    pub has_more: bool,
}

/// Result of a tombstone cascade operation.
#[derive(Debug, Clone)]
pub struct TombstoneResult {
    /// Number of edges that were tombstoned as part of the cascade.
    pub edges_tombstoned: usize,
}

/// A version history record for a node or edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRecord {
    pub version: i64,
    pub changed_by: String,
    pub changed_at: f64,
    pub change_type: String,
    pub change_reason: String,
    pub snapshot: serde_json::Value,
}

// ==== Phase 1: Search & Filter Types ====

/// Options for full-text search.
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Filter results to a specific node type.
    pub node_type: Option<NodeType>,
    /// Filter results to a specific layer.
    pub layer: Option<Layer>,
    /// Maximum number of results (default 20).
    pub limit: Option<u32>,
    /// Minimum FTS score threshold (default 0.0).
    pub min_score: Option<f64>,
    /// Whether to also search the summary field (default true).
    pub search_summary: bool,
}

impl SearchOptions {
    pub fn new() -> Self {
        Self {
            search_summary: true,
            ..Default::default()
        }
    }
}

/// Structured filter for finding nodes.
#[derive(Debug, Clone, Default)]
pub struct NodeFilter {
    /// Filter by node type.
    pub node_type: Option<NodeType>,
    /// Filter by layer.
    pub layer: Option<Layer>,
    /// Substring match on the label field.
    pub label_contains: Option<String>,
    /// Match a JSON props field to an exact value.
    pub prop_equals: Option<(String, String)>,
    /// Match a JSON props field to one of several values.
    pub prop_in: Option<(String, Vec<String>)>,
    /// Minimum confidence (inclusive).
    pub confidence_min: Option<f64>,
    /// Maximum confidence (inclusive).
    pub confidence_max: Option<f64>,
    /// Include tombstoned nodes (default false).
    pub include_tombstoned: bool,
    /// Maximum number of results (default 100).
    pub limit: Option<u32>,
    /// Offset for pagination.
    pub offset: Option<u32>,
    /// Multiple property conditions, AND'd together.
    pub prop_conditions: Vec<PropCondition>,
    /// OR composition: results are the union of all sub-filters.
    pub or_filters: Option<Vec<NodeFilter>>,
    /// Graph-aware: only nodes connected to this UID.
    pub connected_to: Option<Uid>,
    /// Only nodes created after this timestamp.
    pub created_after: Option<Timestamp>,
    /// Only nodes created before this timestamp.
    pub created_before: Option<Timestamp>,
    /// Minimum salience (inclusive).
    pub salience_min: Option<f64>,
    /// Maximum salience (inclusive).
    pub salience_max: Option<f64>,
}

impl NodeFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_type(mut self, nt: NodeType) -> Self {
        self.node_type = Some(nt);
        self
    }

    pub fn layer(mut self, l: Layer) -> Self {
        self.layer = Some(l);
        self
    }

    pub fn label_contains(mut self, term: impl Into<String>) -> Self {
        self.label_contains = Some(term.into());
        self
    }

    pub fn prop_equals(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
        self.prop_equals = Some((field.into(), value.into()));
        self
    }

    pub fn prop_in(mut self, field: impl Into<String>, values: Vec<String>) -> Self {
        self.prop_in = Some((field.into(), values));
        self
    }

    pub fn confidence_range(mut self, min: f64, max: f64) -> Self {
        self.confidence_min = Some(min);
        self.confidence_max = Some(max);
        self
    }

    pub fn prop_condition(mut self, field: impl Into<String>, op: PropOp) -> Self {
        self.prop_conditions.push(PropCondition { field: field.into(), op });
        self
    }

    pub fn or(mut self, filters: Vec<NodeFilter>) -> Self {
        self.or_filters = Some(filters);
        self
    }

    pub fn connected_to(mut self, uid: Uid) -> Self {
        self.connected_to = Some(uid);
        self
    }

    pub fn created_after(mut self, ts: Timestamp) -> Self {
        self.created_after = Some(ts);
        self
    }

    pub fn created_before(mut self, ts: Timestamp) -> Self {
        self.created_before = Some(ts);
        self
    }

    pub fn salience_range(mut self, min: f64, max: f64) -> Self {
        self.salience_min = Some(min);
        self.salience_max = Some(max);
        self
    }
}

// ==== Phase 2: Data Lifecycle Types ====

/// Result of a purge operation.
#[derive(Debug, Clone)]
pub struct PurgeResult {
    pub nodes_purged: usize,
    pub edges_purged: usize,
    pub versions_purged: usize,
}

/// A snapshot of the entire graph for export/import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub relations: BTreeMap<String, serde_json::Value>,
    pub exported_at: Timestamp,
    pub mindgraph_version: String,
}

/// Result of an import operation.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub relations_imported: usize,
}

// ==== Phase 4: Entity Resolution & Batch Types ====

/// Result of merging two entities.
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub edges_retargeted: usize,
    pub aliases_merged: usize,
}

/// A single operation in a batch.
pub enum GraphOp {
    AddNode(Box<CreateNode>),
    AddEdge(Box<CreateEdge>),
    Tombstone { uid: Uid, reason: String, by: String },
    TombstoneEdge { uid: Uid, reason: String, by: String },
}

/// Result of a batch operation.
#[derive(Debug, Clone)]
pub struct BatchResult {
    pub nodes_added: usize,
    pub edges_added: usize,
    pub nodes_tombstoned: usize,
    pub edges_tombstoned: usize,
}

// ==== v0.4 Types ====

/// Graph-wide statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_nodes: u64,
    pub total_edges: u64,
    pub live_nodes: u64,
    pub live_edges: u64,
    pub nodes_by_type: BTreeMap<String, u64>,
    pub nodes_by_layer: BTreeMap<String, u64>,
    pub edges_by_type: BTreeMap<String, u64>,
    pub tombstoned_nodes: u64,
    pub tombstoned_edges: u64,
    pub total_versions: u64,
    pub total_aliases: u64,
    pub embedding_count: u64,
    pub embedding_dimension: Option<usize>,
}

/// Result of a salience decay operation.
#[derive(Debug, Clone)]
pub struct DecayResult {
    pub nodes_decayed: usize,
    pub below_threshold: usize,
}

/// A property filter condition.
#[derive(Debug, Clone)]
pub struct PropCondition {
    pub field: String,
    pub op: PropOp,
}

/// Property filter operations.
#[derive(Debug, Clone)]
pub enum PropOp {
    Equals(String),
    NotEquals(String),
    In(Vec<String>),
    Contains(String),
    GreaterThan(f64),
    LessThan(f64),
}

/// A typed graph snapshot for export/import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedSnapshot {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub exported_at: Timestamp,
    pub mindgraph_version: String,
}

/// Result of a typed import operation.
#[derive(Debug, Clone)]
pub struct TypedImportResult {
    pub nodes_imported: usize,
    pub edges_imported: usize,
    pub nodes_skipped: usize,
    pub edges_skipped: usize,
}

/// A pre-validated batch of operations.
#[derive(Debug)]
pub struct ValidatedBatch {
    pub(crate) nodes_to_add: Vec<CreateNode>,
    pub(crate) edges_to_add: Vec<CreateEdge>,
    pub(crate) tombstone_nodes: Vec<(Uid, String, String)>,
    pub(crate) tombstone_edges: Vec<(Uid, String, String)>,
}
