//! Async wrapper for `MindGraph` using `tokio::task::spawn_blocking`.
//!
//! All methods delegate to the synchronous `MindGraph` methods on a blocking thread pool.

use std::path::Path;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::events::{GraphEvent, SubscriptionId};
use crate::graph::MindGraph;
use crate::provenance::ProvenanceRecord;
use crate::query::{
    BatchResult, Contradiction, DecayResult, GraphOp, GraphSnapshot, GraphStats,
    ImportResult, MergeResult, NodeFilter, Page, Pagination, PurgeResult,
    SearchOptions, SearchResult, TombstoneResult, TypedImportResult, TypedSnapshot,
    ValidatedBatch, VersionRecord,
};
use crate::schema::edge::{CreateEdge, GraphEdge};
use crate::schema::edge_props::EdgeProps;
use crate::schema::node::{CreateNode, GraphNode};
use crate::schema::node_props::NodeProps;
use crate::schema::{EdgeType, Layer, NodeType};
use crate::traversal::{PathStep, TraversalOptions};
use crate::types::*;

/// Convert a `JoinError` from `spawn_blocking` into our `Error` type.
fn join_err(e: tokio::task::JoinError) -> Error {
    Error::TaskJoin(e.to_string())
}

/// Async wrapper around [`MindGraph`] for use in tokio runtimes.
///
/// Each method clones the inner `Arc<MindGraph>` and runs the synchronous
/// operation via `tokio::task::spawn_blocking`.
#[derive(Clone)]
pub struct AsyncMindGraph {
    inner: Arc<MindGraph>,
}

impl AsyncMindGraph {
    /// Async version of [`MindGraph::open`].
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_owned();
        let graph = tokio::task::spawn_blocking(move || MindGraph::open(path))
            .await
            .map_err(join_err)??;
        Ok(Self { inner: Arc::new(graph) })
    }

    /// Async version of [`MindGraph::open_in_memory`].
    pub async fn open_in_memory() -> Result<Self> {
        let graph = tokio::task::spawn_blocking(MindGraph::open_in_memory)
            .await
            .map_err(join_err)??;
        Ok(Self { inner: Arc::new(graph) })
    }

    /// Wrap an existing synchronous `MindGraph`.
    pub fn from_sync(graph: MindGraph) -> Self {
        Self { inner: Arc::new(graph) }
    }

    /// Access the underlying synchronous graph.
    pub fn inner(&self) -> &MindGraph {
        &self.inner
    }

    // ---- Node Operations ----

    /// Async version of [`MindGraph::add_node`].
    pub async fn add_node(&self, create: CreateNode) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_node(create))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::get_node`].
    pub async fn get_node(&self, uid: Uid) -> Result<Option<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_node(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::get_live_node`].
    pub async fn get_live_node(&self, uid: Uid) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_live_node(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::update_node`].
    #[allow(clippy::too_many_arguments)]
    pub async fn update_node(
        &self,
        uid: Uid,
        label: Option<String>,
        summary: Option<String>,
        confidence: Option<Confidence>,
        salience: Option<Salience>,
        props: Option<NodeProps>,
        changed_by: String,
        reason: String,
    ) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            g.update_node(&uid, label, summary, confidence, salience, props, &changed_by, &reason)
        })
        .await
        .map_err(join_err)?
    }

    // ---- Edge Operations ----

    /// Async version of [`MindGraph::add_edge`].
    pub async fn add_edge(&self, create: CreateEdge) -> Result<GraphEdge> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_edge(create))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::get_edge`].
    pub async fn get_edge(&self, uid: Uid) -> Result<Option<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_edge(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::edges_from`].
    pub async fn edges_from(&self, uid: Uid, edge_type: Option<EdgeType>) -> Result<Vec<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.edges_from(&uid, edge_type))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::edges_to`].
    pub async fn edges_to(&self, uid: Uid, edge_type: Option<EdgeType>) -> Result<Vec<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.edges_to(&uid, edge_type))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::get_live_edge`].
    pub async fn get_live_edge(&self, uid: Uid) -> Result<GraphEdge> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_live_edge(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::update_edge`].
    pub async fn update_edge(
        &self,
        uid: Uid,
        confidence: Option<Confidence>,
        weight: Option<f64>,
        props: Option<EdgeProps>,
        changed_by: String,
        reason: String,
    ) -> Result<GraphEdge> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            g.update_edge(&uid, confidence, weight, props, &changed_by, &reason)
        })
        .await
        .map_err(join_err)?
    }

    // ---- Tombstone Operations ----

    /// Async version of [`MindGraph::tombstone`].
    pub async fn tombstone(&self, uid: Uid, reason: String, by: String) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.tombstone(&uid, &reason, &by))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::restore`].
    pub async fn restore(&self, uid: Uid) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.restore(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::tombstone_edge`].
    pub async fn tombstone_edge(&self, uid: Uid, reason: String, by: String) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.tombstone_edge(&uid, &reason, &by))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::restore_edge`].
    pub async fn restore_edge(&self, uid: Uid) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.restore_edge(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::tombstone_cascade`].
    pub async fn tombstone_cascade(&self, uid: Uid, reason: String, by: String) -> Result<TombstoneResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.tombstone_cascade(&uid, &reason, &by))
            .await
            .map_err(join_err)?
    }

    // ---- Provenance ----

    /// Async version of [`MindGraph::add_provenance`].
    pub async fn add_provenance(&self, record: ProvenanceRecord) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_provenance(&record))
            .await
            .map_err(join_err)?
    }

    // ---- Entity Resolution ----

    /// Async version of [`MindGraph::add_alias`].
    pub async fn add_alias(&self, alias_text: String, canonical_uid: Uid, match_score: f64) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_alias(&alias_text, &canonical_uid, match_score))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::resolve_alias`].
    pub async fn resolve_alias(&self, text: String) -> Result<Option<Uid>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.resolve_alias(&text))
            .await
            .map_err(join_err)?
    }

    // ---- Query Patterns ----

    /// Async version of [`MindGraph::active_goals`].
    pub async fn active_goals(&self) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.active_goals())
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::pending_approvals`].
    pub async fn pending_approvals(&self) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.pending_approvals())
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::unresolved_contradictions`].
    pub async fn unresolved_contradictions(&self) -> Result<Vec<Contradiction>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.unresolved_contradictions())
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::open_decisions`].
    pub async fn open_decisions(&self) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.open_decisions())
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::open_questions`].
    pub async fn open_questions(&self) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.open_questions())
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::weak_claims`].
    pub async fn weak_claims(&self, threshold: f64) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.weak_claims(threshold))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::nodes_in_layer`].
    pub async fn nodes_in_layer(&self, layer: Layer) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.nodes_in_layer(layer))
            .await
            .map_err(join_err)?
    }

    // ---- Count / Exists ----

    /// Async version of [`MindGraph::count_nodes`].
    pub async fn count_nodes(&self, node_type: NodeType) -> Result<u64> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.count_nodes(node_type))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::count_nodes_in_layer`].
    pub async fn count_nodes_in_layer(&self, layer: Layer) -> Result<u64> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.count_nodes_in_layer(layer))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::count_edges`].
    pub async fn count_edges(&self, edge_type: EdgeType) -> Result<u64> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.count_edges(edge_type))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::node_exists`].
    pub async fn node_exists(&self, uid: Uid) -> Result<bool> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.node_exists(&uid))
            .await
            .map_err(join_err)?
    }

    // ---- Traversal ----

    /// Async version of [`MindGraph::reachable`].
    pub async fn reachable(&self, start: Uid, opts: TraversalOptions) -> Result<Vec<PathStep>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.reachable(&start, &opts))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::reasoning_chain`].
    pub async fn reasoning_chain(&self, claim_uid: Uid, max_depth: u32) -> Result<Vec<PathStep>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.reasoning_chain(&claim_uid, max_depth))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::neighborhood`].
    pub async fn neighborhood(&self, uid: Uid, depth: u32) -> Result<Vec<PathStep>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.neighborhood(&uid, depth))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::find_path`].
    pub async fn find_path(&self, from: Uid, to: Uid, opts: TraversalOptions) -> Result<Option<Vec<PathStep>>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.find_path(&from, &to, &opts))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::subgraph`].
    pub async fn subgraph(
        &self,
        start: Uid,
        opts: TraversalOptions,
    ) -> Result<(Vec<GraphNode>, Vec<GraphEdge>)> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.subgraph(&start, &opts))
            .await
            .map_err(join_err)?
    }

    // ---- Pagination ----

    /// Async version of [`MindGraph::nodes_in_layer_paginated`].
    pub async fn nodes_in_layer_paginated(&self, layer: Layer, page: Pagination) -> Result<Page<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.nodes_in_layer_paginated(layer, page))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::edges_from_paginated`].
    pub async fn edges_from_paginated(
        &self,
        uid: Uid,
        edge_type: Option<EdgeType>,
        page: Pagination,
    ) -> Result<Page<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.edges_from_paginated(&uid, edge_type, page))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::edges_to_paginated`].
    pub async fn edges_to_paginated(
        &self,
        uid: Uid,
        edge_type: Option<EdgeType>,
        page: Pagination,
    ) -> Result<Page<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.edges_to_paginated(&uid, edge_type, page))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::weak_claims_paginated`].
    pub async fn weak_claims_paginated(&self, threshold: f64, page: Pagination) -> Result<Page<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.weak_claims_paginated(threshold, page))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::active_goals_paginated`].
    pub async fn active_goals_paginated(&self, page: Pagination) -> Result<Page<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.active_goals_paginated(page))
            .await
            .map_err(join_err)?
    }

    // ---- Batch Operations ----

    /// Async version of [`MindGraph::add_nodes_batch`].
    pub async fn add_nodes_batch(&self, creates: Vec<CreateNode>) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_nodes_batch(creates))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::add_edges_batch`].
    pub async fn add_edges_batch(&self, creates: Vec<CreateEdge>) -> Result<Vec<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_edges_batch(creates))
            .await
            .map_err(join_err)?
    }

    // ---- Version History ----

    /// Async version of [`MindGraph::node_history`].
    pub async fn node_history(&self, uid: Uid) -> Result<Vec<VersionRecord>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.node_history(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::edge_history`].
    pub async fn edge_history(&self, uid: Uid) -> Result<Vec<VersionRecord>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.edge_history(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::node_at_version`].
    pub async fn node_at_version(&self, uid: Uid, version: i64) -> Result<Option<serde_json::Value>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.node_at_version(&uid, version))
            .await
            .map_err(join_err)?
    }

    // ---- Full-Text Search ----

    /// Async version of [`MindGraph::search`].
    pub async fn search(&self, query: String, opts: SearchOptions) -> Result<Vec<SearchResult>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.search(&query, &opts))
            .await
            .map_err(join_err)?
    }

    // ---- Structured Filtering ----

    /// Async version of [`MindGraph::find_nodes`].
    pub async fn find_nodes(&self, filter: NodeFilter) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.find_nodes(&filter))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::find_nodes_paginated`].
    pub async fn find_nodes_paginated(&self, filter: NodeFilter) -> Result<Page<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.find_nodes_paginated(&filter))
            .await
            .map_err(join_err)?
    }

    // ---- Data Lifecycle ----

    /// Async version of [`MindGraph::purge_tombstoned`].
    pub async fn purge_tombstoned(&self, older_than: Option<Timestamp>) -> Result<PurgeResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.purge_tombstoned(older_than))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::export`].
    pub async fn export(&self) -> Result<GraphSnapshot> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.export())
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::import`].
    pub async fn import(&self, snapshot: GraphSnapshot) -> Result<ImportResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.import(&snapshot))
            .await
            .map_err(join_err)?
    }

    // ---- Entity Resolution ----

    /// Async version of [`MindGraph::aliases_for`].
    pub async fn aliases_for(&self, uid: Uid) -> Result<Vec<(String, f64)>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.aliases_for(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::merge_entities`].
    pub async fn merge_entities(
        &self,
        keep_uid: Uid,
        merge_uid: Uid,
        reason: String,
        by: String,
    ) -> Result<MergeResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.merge_entities(&keep_uid, &merge_uid, &reason, &by))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::fuzzy_resolve`].
    pub async fn fuzzy_resolve(&self, text: String, limit: u32) -> Result<Vec<(Uid, f64)>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.fuzzy_resolve(&text, limit))
            .await
            .map_err(join_err)?
    }

    // ---- Batch Operations ----

    /// Async version of [`MindGraph::batch_apply`].
    pub async fn batch_apply(&self, ops: Vec<GraphOp>) -> Result<BatchResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.batch_apply(ops))
            .await
            .map_err(join_err)?
    }

    // ---- Default Agent ----

    /// Async version of [`MindGraph::set_default_agent`].
    pub async fn set_default_agent(&self, name: String) {
        self.inner.set_default_agent(name);
    }

    /// Async version of [`MindGraph::default_agent`].
    pub async fn default_agent(&self) -> String {
        self.inner.default_agent()
    }

    // ---- v0.4: Stats ----

    /// Async version of [`MindGraph::stats`].
    pub async fn stats(&self) -> Result<GraphStats> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.stats())
            .await
            .map_err(join_err)?
    }

    // ---- v0.4: Convenience Constructors ----

    /// Async version of [`MindGraph::add_claim`].
    pub async fn add_claim(&self, label: String, content: String, confidence: f64) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_claim(&label, &content, confidence))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::add_entity`].
    pub async fn add_entity(&self, label: String, entity_type: String) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_entity(&label, &entity_type))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::add_goal`].
    pub async fn add_goal(&self, label: String, priority: String) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_goal(&label, &priority))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::add_observation`].
    pub async fn add_observation(&self, label: String, description: String) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_observation(&label, &description))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::add_memory`].
    #[deprecated(since = "0.4.1", note = "Use add_session() instead")]
    pub async fn add_memory(&self, label: String, content: String) -> Result<GraphNode> {
        self.add_session(label, content).await
    }

    /// Async version of [`MindGraph::add_session`].
    pub async fn add_session(&self, label: String, focus: String) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_session(&label, &focus))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::add_preference`].
    pub async fn add_preference(&self, label: String, key: String, value: String) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_preference(&label, &key, &value))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::add_summary`].
    pub async fn add_summary(&self, label: String, content: String) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_summary(&label, &content))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::add_link`].
    pub async fn add_link(&self, from: Uid, to: Uid, edge_type: EdgeType) -> Result<GraphEdge> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_link(&from, &to, edge_type))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::add_custom_node`].
    pub async fn add_custom_node<T: crate::schema::CustomNodeType + 'static>(
        &self,
        label: String,
        props: T,
    ) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_custom_node(&label, props))
            .await
            .map_err(join_err)?
    }

    // ---- v0.4: Embeddings ----

    /// Async version of [`MindGraph::configure_embeddings`].
    pub async fn configure_embeddings(&self, dimension: usize) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.configure_embeddings(dimension))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::embedding_dimension`].
    pub fn embedding_dimension(&self) -> Option<usize> {
        self.inner.embedding_dimension()
    }

    /// Async version of [`MindGraph::embed_nodes`].
    pub async fn embed_nodes(&self, uids: Vec<Uid>, provider: Arc<dyn crate::embeddings::EmbeddingProvider>) -> Result<usize> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.embed_nodes(&uids, provider.as_ref()))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::set_embedding`].
    pub async fn set_embedding(&self, uid: Uid, embedding: Vec<f32>) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.set_embedding(&uid, &embedding))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::get_embedding`].
    pub async fn get_embedding(&self, uid: Uid) -> Result<Option<Vec<f32>>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_embedding(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::delete_embedding`].
    pub async fn delete_embedding(&self, uid: Uid) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.delete_embedding(&uid))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::semantic_search`].
    pub async fn semantic_search(&self, query_vec: Vec<f32>, k: usize) -> Result<Vec<(GraphNode, f64)>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.semantic_search(&query_vec, k))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::embed_node`] (sync provider via spawn_blocking).
    pub async fn embed_node(&self, uid: Uid, provider: Arc<dyn crate::embeddings::EmbeddingProvider>) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.embed_node(&uid, provider.as_ref()))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::semantic_search_text`] (sync provider via spawn_blocking).
    pub async fn semantic_search_text(
        &self,
        query: String,
        k: usize,
        provider: Arc<dyn crate::embeddings::EmbeddingProvider>,
    ) -> Result<Vec<(GraphNode, f64)>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.semantic_search_text(&query, k, provider.as_ref()))
            .await
            .map_err(join_err)?
    }

    /// Embed a single node using a native [`AsyncEmbeddingProvider`](crate::embeddings::AsyncEmbeddingProvider).
    pub async fn embed_node_async(
        &self,
        uid: &Uid,
        provider: &dyn crate::embeddings::AsyncEmbeddingProvider,
    ) -> Result<()> {
        let node = self.get_live_node(uid.clone()).await?;
        let text = if node.summary.is_empty() {
            node.label.clone()
        } else {
            format!("{} {}", node.label, node.summary)
        };
        let embedding = provider.embed(&text).await?;
        self.set_embedding(uid.clone(), embedding).await
    }

    /// Embed multiple nodes using a native [`AsyncEmbeddingProvider`](crate::embeddings::AsyncEmbeddingProvider).
    /// Returns the count of nodes successfully embedded.
    pub async fn embed_nodes_async(
        &self,
        uids: &[Uid],
        provider: &dyn crate::embeddings::AsyncEmbeddingProvider,
    ) -> Result<usize> {
        let mut texts = Vec::new();
        let mut live_uids = Vec::new();

        for uid in uids {
            if let Some(node) = self.get_node(uid.clone()).await? {
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

        let embeddings = provider.embed_batch(&texts).await?;

        for (uid, embedding) in live_uids.iter().zip(embeddings.iter()) {
            self.set_embedding(uid.clone(), embedding.clone()).await?;
        }

        Ok(live_uids.len())
    }

    /// Search by text using a native [`AsyncEmbeddingProvider`](crate::embeddings::AsyncEmbeddingProvider).
    pub async fn semantic_search_text_async(
        &self,
        query: &str,
        k: usize,
        provider: &dyn crate::embeddings::AsyncEmbeddingProvider,
    ) -> Result<Vec<(GraphNode, f64)>> {
        let query_vec = provider.embed(query).await?;
        self.semantic_search(query_vec, k).await
    }

    // ---- v0.4: Decay ----

    /// Async version of [`MindGraph::decay_salience`].
    pub async fn decay_salience(&self, half_life_secs: f64) -> Result<DecayResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.decay_salience(half_life_secs))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::auto_tombstone`].
    pub async fn auto_tombstone(&self, min_salience: f64, min_age_secs: f64) -> Result<usize> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.auto_tombstone(min_salience, min_age_secs))
            .await
            .map_err(join_err)?
    }

    // ---- v0.4: Events ----

    /// Subscribe to graph change events. See [`MindGraph::on_change`].
    pub fn on_change(&self, cb: impl Fn(&GraphEvent) + Send + Sync + 'static) -> SubscriptionId {
        self.inner.on_change(cb)
    }

    /// Remove a subscription. See [`MindGraph::unsubscribe`].
    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.inner.unsubscribe(id);
    }

    /// Subscribe to filtered graph change events. See [`MindGraph::on_change_filtered`].
    pub fn on_change_filtered(
        &self,
        filter: crate::events::EventFilter,
        cb: impl Fn(&GraphEvent) + Send + Sync + 'static,
    ) -> SubscriptionId {
        self.inner.on_change_filtered(filter, cb)
    }

    /// Create an async filtered event stream. See [`MindGraph::watch`].
    pub fn watch(&self, filter: crate::events::EventFilter) -> crate::watch::WatchStream {
        self.inner.watch(filter)
    }

    // ---- v0.4: Typed Export/Import ----

    /// Async version of [`MindGraph::export_typed`].
    pub async fn export_typed(&self) -> Result<TypedSnapshot> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.export_typed())
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::import_typed`].
    pub async fn import_typed(&self, snapshot: TypedSnapshot) -> Result<TypedImportResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.import_typed(&snapshot))
            .await
            .map_err(join_err)?
    }

    // ---- v0.4: Validated Batch ----

    /// Async version of [`MindGraph::validate_batch`].
    pub async fn validate_batch(&self, ops: Vec<GraphOp>) -> Result<ValidatedBatch> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.validate_batch(ops))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::apply_validated_batch`].
    pub async fn apply_validated_batch(&self, batch: ValidatedBatch) -> Result<BatchResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.apply_validated_batch(batch))
            .await
            .map_err(join_err)?
    }

    // ---- v0.5: New Methods ----

    /// Async version of [`MindGraph::get_edge_between`].
    pub async fn get_edge_between(
        &self,
        from_uid: Uid,
        to_uid: Uid,
        edge_type: Option<EdgeType>,
    ) -> Result<Vec<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_edge_between(&from_uid, &to_uid, edge_type))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::list_nodes`].
    pub async fn list_nodes(&self, page: Pagination) -> Result<Page<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.list_nodes(page))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`MindGraph::clear`].
    pub async fn clear(&self) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.clear())
            .await
            .map_err(join_err)?
    }

    /// Create a scoped async agent handle bound to this graph.
    pub fn agent(&self, name: impl Into<String>) -> AsyncAgentHandle {
        AsyncAgentHandle {
            handle: crate::agent::AgentHandle::new(self.inner.clone(), name.into(), None),
        }
    }
}

/// Async wrapper around [`AgentHandle`](crate::agent::AgentHandle).
///
/// All mutation methods are async via `spawn_blocking` and automatically
/// set `changed_by` to this agent's identity.
pub struct AsyncAgentHandle {
    handle: crate::agent::AgentHandle,
}

impl AsyncAgentHandle {
    /// Get the agent identity.
    pub fn agent_id(&self) -> &str {
        self.handle.agent_id()
    }

    /// Get the parent agent identity, if this is a sub-agent.
    pub fn parent_agent(&self) -> Option<&str> {
        self.handle.parent_agent()
    }

    /// Access the underlying sync agent handle.
    pub fn inner(&self) -> &crate::agent::AgentHandle {
        &self.handle
    }

    /// Create a child async agent handle.
    pub fn sub_agent(&self, name: impl Into<String>) -> AsyncAgentHandle {
        AsyncAgentHandle {
            handle: self.handle.sub_agent(name),
        }
    }

    /// Async version of [`AgentHandle::add_node`](crate::agent::AgentHandle::add_node).
    pub async fn add_node(&self, create: CreateNode) -> Result<GraphNode> {
        let g = self.handle.graph_arc().clone();
        let agent = self.handle.agent_id().to_string();
        tokio::task::spawn_blocking(move || g.add_node_as(create, &agent))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`AgentHandle::add_edge`](crate::agent::AgentHandle::add_edge).
    pub async fn add_edge(&self, create: CreateEdge) -> Result<GraphEdge> {
        let g = self.handle.graph_arc().clone();
        let agent = self.handle.agent_id().to_string();
        tokio::task::spawn_blocking(move || g.add_edge_as(create, &agent))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`AgentHandle::tombstone`](crate::agent::AgentHandle::tombstone).
    pub async fn tombstone(&self, uid: Uid, reason: String) -> Result<()> {
        let g = self.handle.graph_arc().clone();
        let agent = self.handle.agent_id().to_string();
        tokio::task::spawn_blocking(move || g.tombstone(&uid, &reason, &agent))
            .await
            .map_err(join_err)?
    }

    /// Async version of [`AgentHandle::add_entity`](crate::agent::AgentHandle::add_entity).
    pub async fn add_entity(&self, label: String, entity_type: String) -> Result<GraphNode> {
        let g = self.handle.graph_arc().clone();
        let agent = self.handle.agent_id().to_string();
        tokio::task::spawn_blocking(move || g.add_node_as(
            CreateNode::new(&label, NodeProps::Entity(crate::schema::props::reality::EntityProps {
                entity_type,
                ..Default::default()
            })),
            &agent,
        ))
        .await
        .map_err(join_err)?
    }

    /// Async version of [`AgentHandle::add_claim`](crate::agent::AgentHandle::add_claim).
    pub async fn add_claim(&self, label: String, content: String, confidence: f64) -> Result<GraphNode> {
        let g = self.handle.graph_arc().clone();
        let agent = self.handle.agent_id().to_string();
        let conf = Confidence::new(confidence)?;
        tokio::task::spawn_blocking(move || g.add_node_as(
            CreateNode::new(&label, NodeProps::Claim(crate::schema::props::epistemic::ClaimProps {
                content,
                ..Default::default()
            })).confidence(conf),
            &agent,
        ))
        .await
        .map_err(join_err)?
    }

    /// Get a node by UID.
    pub async fn get_node(&self, uid: Uid) -> Result<Option<GraphNode>> {
        let g = self.handle.graph_arc().clone();
        tokio::task::spawn_blocking(move || g.get_node(&uid))
            .await
            .map_err(join_err)?
    }

    /// Get all live nodes created by this agent.
    pub async fn my_nodes(&self) -> Result<Vec<GraphNode>> {
        let g = self.handle.graph_arc().clone();
        let agent = self.handle.agent_id().to_string();
        tokio::task::spawn_blocking(move || g.nodes_by_agent(&agent))
            .await
            .map_err(join_err)?
    }

    /// Create an async filtered event stream.
    pub fn watch(&self, filter: crate::events::EventFilter) -> crate::watch::WatchStream {
        self.handle.watch(filter)
    }

    /// Create an async event stream filtered to events triggered by this agent.
    pub fn watch_mine(&self) -> crate::watch::WatchStream {
        self.handle.watch_mine()
    }
}
