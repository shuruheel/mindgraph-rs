//! Async wrapper for `MindGraph` using `tokio::task::spawn_blocking`.
//!
//! All methods delegate to the synchronous `MindGraph` methods on a blocking thread pool.

use std::path::Path;
use std::sync::Arc;

use crate::error::Result;
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

/// Async wrapper around `MindGraph` for use in tokio runtimes.
///
/// Each method clones the inner `Arc<MindGraph>` and runs the synchronous
/// operation via `tokio::task::spawn_blocking`.
#[derive(Clone)]
pub struct AsyncMindGraph {
    inner: Arc<MindGraph>,
}

impl AsyncMindGraph {
    /// Open a persistent graph at the given path.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_owned();
        let graph = tokio::task::spawn_blocking(move || MindGraph::open(path))
            .await
            .expect("spawn_blocking join")?;
        Ok(Self { inner: Arc::new(graph) })
    }

    /// Create an in-memory graph.
    pub async fn open_in_memory() -> Result<Self> {
        let graph = tokio::task::spawn_blocking(MindGraph::open_in_memory)
            .await
            .expect("spawn_blocking join")?;
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

    pub async fn add_node(&self, create: CreateNode) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_node(create))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn get_node(&self, uid: Uid) -> Result<Option<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_node(&uid))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn get_live_node(&self, uid: Uid) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_live_node(&uid))
            .await
            .expect("spawn_blocking join")
    }

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
        .expect("spawn_blocking join")
    }

    // ---- Edge Operations ----

    pub async fn add_edge(&self, create: CreateEdge) -> Result<GraphEdge> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_edge(create))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn get_edge(&self, uid: Uid) -> Result<Option<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_edge(&uid))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn edges_from(&self, uid: Uid, edge_type: Option<EdgeType>) -> Result<Vec<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.edges_from(&uid, edge_type))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn edges_to(&self, uid: Uid, edge_type: Option<EdgeType>) -> Result<Vec<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.edges_to(&uid, edge_type))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn get_live_edge(&self, uid: Uid) -> Result<GraphEdge> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_live_edge(&uid))
            .await
            .expect("spawn_blocking join")
    }

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
        .expect("spawn_blocking join")
    }

    // ---- Tombstone Operations ----

    pub async fn tombstone(&self, uid: Uid, reason: String, by: String) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.tombstone(&uid, &reason, &by))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn restore(&self, uid: Uid) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.restore(&uid))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn tombstone_edge(&self, uid: Uid, reason: String, by: String) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.tombstone_edge(&uid, &reason, &by))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn restore_edge(&self, uid: Uid) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.restore_edge(&uid))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn tombstone_cascade(&self, uid: Uid, reason: String, by: String) -> Result<TombstoneResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.tombstone_cascade(&uid, &reason, &by))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Provenance ----

    pub async fn add_provenance(&self, record: ProvenanceRecord) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_provenance(&record))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Entity Resolution ----

    pub async fn add_alias(&self, alias_text: String, canonical_uid: Uid, match_score: f64) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_alias(&alias_text, &canonical_uid, match_score))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn resolve_alias(&self, text: String) -> Result<Option<Uid>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.resolve_alias(&text))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Query Patterns ----

    pub async fn active_goals(&self) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.active_goals())
            .await
            .expect("spawn_blocking join")
    }

    pub async fn pending_approvals(&self) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.pending_approvals())
            .await
            .expect("spawn_blocking join")
    }

    pub async fn unresolved_contradictions(&self) -> Result<Vec<Contradiction>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.unresolved_contradictions())
            .await
            .expect("spawn_blocking join")
    }

    pub async fn open_decisions(&self) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.open_decisions())
            .await
            .expect("spawn_blocking join")
    }

    pub async fn open_questions(&self) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.open_questions())
            .await
            .expect("spawn_blocking join")
    }

    pub async fn weak_claims(&self, threshold: f64) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.weak_claims(threshold))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn nodes_in_layer(&self, layer: Layer) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.nodes_in_layer(layer))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Count / Exists ----

    pub async fn count_nodes(&self, node_type: NodeType) -> Result<u64> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.count_nodes(node_type))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn count_nodes_in_layer(&self, layer: Layer) -> Result<u64> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.count_nodes_in_layer(layer))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn count_edges(&self, edge_type: EdgeType) -> Result<u64> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.count_edges(edge_type))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn node_exists(&self, uid: Uid) -> Result<bool> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.node_exists(&uid))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Traversal ----

    pub async fn reachable(&self, start: Uid, opts: TraversalOptions) -> Result<Vec<PathStep>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.reachable(&start, &opts))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn reasoning_chain(&self, claim_uid: Uid, max_depth: u32) -> Result<Vec<PathStep>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.reasoning_chain(&claim_uid, max_depth))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn neighborhood(&self, uid: Uid, depth: u32) -> Result<Vec<PathStep>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.neighborhood(&uid, depth))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn find_path(&self, from: Uid, to: Uid, opts: TraversalOptions) -> Result<Option<Vec<PathStep>>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.find_path(&from, &to, &opts))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn subgraph(
        &self,
        start: Uid,
        opts: TraversalOptions,
    ) -> Result<(Vec<GraphNode>, Vec<GraphEdge>)> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.subgraph(&start, &opts))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Pagination ----

    pub async fn nodes_in_layer_paginated(&self, layer: Layer, page: Pagination) -> Result<Page<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.nodes_in_layer_paginated(layer, page))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn edges_from_paginated(
        &self,
        uid: Uid,
        edge_type: Option<EdgeType>,
        page: Pagination,
    ) -> Result<Page<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.edges_from_paginated(&uid, edge_type, page))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn edges_to_paginated(
        &self,
        uid: Uid,
        edge_type: Option<EdgeType>,
        page: Pagination,
    ) -> Result<Page<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.edges_to_paginated(&uid, edge_type, page))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn weak_claims_paginated(&self, threshold: f64, page: Pagination) -> Result<Page<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.weak_claims_paginated(threshold, page))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn active_goals_paginated(&self, page: Pagination) -> Result<Page<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.active_goals_paginated(page))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Batch Operations ----

    pub async fn add_nodes_batch(&self, creates: Vec<CreateNode>) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_nodes_batch(creates))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn add_edges_batch(&self, creates: Vec<CreateEdge>) -> Result<Vec<GraphEdge>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_edges_batch(creates))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Version History ----

    pub async fn node_history(&self, uid: Uid) -> Result<Vec<VersionRecord>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.node_history(&uid))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn edge_history(&self, uid: Uid) -> Result<Vec<VersionRecord>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.edge_history(&uid))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn node_at_version(&self, uid: Uid, version: i64) -> Result<Option<serde_json::Value>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.node_at_version(&uid, version))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Full-Text Search ----

    pub async fn search(&self, query: String, opts: SearchOptions) -> Result<Vec<SearchResult>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.search(&query, &opts))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Structured Filtering ----

    pub async fn find_nodes(&self, filter: NodeFilter) -> Result<Vec<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.find_nodes(&filter))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn find_nodes_paginated(&self, filter: NodeFilter) -> Result<Page<GraphNode>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.find_nodes_paginated(&filter))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Data Lifecycle ----

    pub async fn purge_tombstoned(&self, older_than: Option<Timestamp>) -> Result<PurgeResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.purge_tombstoned(older_than))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn export(&self) -> Result<GraphSnapshot> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.export())
            .await
            .expect("spawn_blocking join")
    }

    pub async fn import(&self, snapshot: GraphSnapshot) -> Result<ImportResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.import(&snapshot))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Entity Resolution ----

    pub async fn aliases_for(&self, uid: Uid) -> Result<Vec<(String, f64)>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.aliases_for(&uid))
            .await
            .expect("spawn_blocking join")
    }

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
            .expect("spawn_blocking join")
    }

    pub async fn fuzzy_resolve(&self, text: String, limit: u32) -> Result<Vec<(Uid, f64)>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.fuzzy_resolve(&text, limit))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Batch Operations ----

    pub async fn batch_apply(&self, ops: Vec<GraphOp>) -> Result<BatchResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.batch_apply(ops))
            .await
            .expect("spawn_blocking join")
    }

    // ---- Default Agent ----

    pub async fn set_default_agent(&self, name: String) {
        self.inner.set_default_agent(name);
    }

    pub async fn default_agent(&self) -> String {
        self.inner.default_agent()
    }

    // ---- v0.4: Stats ----

    pub async fn stats(&self) -> Result<GraphStats> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.stats())
            .await
            .expect("spawn_blocking join")
    }

    // ---- v0.4: Convenience Constructors ----

    pub async fn add_claim(&self, label: String, content: String, confidence: f64) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_claim(&label, &content, confidence))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn add_entity(&self, label: String, entity_type: String) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_entity(&label, &entity_type))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn add_goal(&self, label: String, priority: String) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_goal(&label, &priority))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn add_observation(&self, label: String, description: String) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_observation(&label, &description))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn add_memory(&self, label: String, content: String) -> Result<GraphNode> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_memory(&label, &content))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn add_link(&self, from: Uid, to: Uid, edge_type: EdgeType) -> Result<GraphEdge> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.add_link(&from, &to, edge_type))
            .await
            .expect("spawn_blocking join")
    }

    // ---- v0.4: Embeddings ----

    pub async fn configure_embeddings(&self, dimension: usize) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.configure_embeddings(dimension))
            .await
            .expect("spawn_blocking join")
    }

    pub fn embedding_dimension(&self) -> Option<usize> {
        self.inner.embedding_dimension()
    }

    pub async fn set_embedding(&self, uid: Uid, embedding: Vec<f32>) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.set_embedding(&uid, &embedding))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn get_embedding(&self, uid: Uid) -> Result<Option<Vec<f32>>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.get_embedding(&uid))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn delete_embedding(&self, uid: Uid) -> Result<()> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.delete_embedding(&uid))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn semantic_search(&self, query_vec: Vec<f32>, k: usize) -> Result<Vec<(GraphNode, f64)>> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.semantic_search(&query_vec, k))
            .await
            .expect("spawn_blocking join")
    }

    // ---- v0.4: Decay ----

    pub async fn decay_salience(&self, half_life_secs: f64) -> Result<DecayResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.decay_salience(half_life_secs))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn auto_tombstone(&self, min_salience: f64, min_age_secs: f64) -> Result<usize> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.auto_tombstone(min_salience, min_age_secs))
            .await
            .expect("spawn_blocking join")
    }

    // ---- v0.4: Events ----

    pub fn on_change(&self, cb: impl Fn(&GraphEvent) + Send + Sync + 'static) -> SubscriptionId {
        self.inner.on_change(cb)
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.inner.unsubscribe(id);
    }

    // ---- v0.4: Typed Export/Import ----

    pub async fn export_typed(&self) -> Result<TypedSnapshot> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.export_typed())
            .await
            .expect("spawn_blocking join")
    }

    pub async fn import_typed(&self, snapshot: TypedSnapshot) -> Result<TypedImportResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.import_typed(&snapshot))
            .await
            .expect("spawn_blocking join")
    }

    // ---- v0.4: Validated Batch ----

    pub async fn validate_batch(&self, ops: Vec<GraphOp>) -> Result<ValidatedBatch> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.validate_batch(ops))
            .await
            .expect("spawn_blocking join")
    }

    pub async fn apply_validated_batch(&self, batch: ValidatedBatch) -> Result<BatchResult> {
        let g = self.inner.clone();
        tokio::task::spawn_blocking(move || g.apply_validated_batch(batch))
            .await
            .expect("spawn_blocking join")
    }
}
