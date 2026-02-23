# CLAUDE.md

## Build & Test

```bash
cargo build                              # Build the library
cargo test                               # Run all tests (130 integration + 10 doc-tests)
cargo test --features async              # Run all tests including async (130 + 2 async + 10 doc-tests)
cargo clippy --all-features -- -W clippy::all  # Lint (must produce 0 warnings)
cargo doc --no-deps --all-features       # Build docs with doc-tests
cargo bench                              # Run criterion benchmarks
cargo run --example basic                # Run basic example
cargo run --example agent_memory         # Run agent memory example
cargo run --example embedding_search --features async  # Run embedding search example
cargo publish --dry-run --allow-dirty    # Verify publishability
```

## Project Structure

- `src/graph.rs` -- Main `MindGraph` API, `NodeUpdate`/`EdgeUpdate` builders, search/filter/lifecycle/entity methods.
- `src/async_graph.rs` -- `AsyncMindGraph` wrapper for tokio runtimes (behind `async` feature flag).
- `src/storage/cozo.rs` -- `CozoStorage` with all CozoDB Datalog queries. Internal only, exposed via `graph.storage()`.
- `src/storage/migrations.rs` -- CozoDB schema DDL (`:create` statements, `::index` and `::fts` directives).
- `src/schema/` -- Type enums (`NodeType`, `EdgeType`, `Layer`), node/edge structs, and `NodeProps`/`EdgeProps` discriminated unions.
- `src/schema/props/` -- Per-layer property structs (reality, epistemic, intent, action, memory, agent).
- `src/traversal.rs` -- `Direction`, `TraversalOptions`, `PathStep` types (with typed `NodeType`/`EdgeType` fields).
- `src/query.rs` -- Query/result types: `Pagination`, `Page<T>`, `SearchOptions`, `NodeFilter`, `GraphStats`, `DecayResult`, `PropCondition`, `PropOp`, `TypedSnapshot`, `TypedImportResult`, `ValidatedBatch`, `PurgeResult`, `GraphSnapshot`, `ImportResult`, `MergeResult`, `GraphOp`, `BatchResult`, etc.
- `src/types.rs` -- Core value types: `Uid`, `Confidence`, `Salience`, `PrivacyLevel`, `Timestamp`.
- `src/provenance.rs` -- `ProvenanceRecord`, `ExtractionMethod`.
- `src/embeddings.rs` -- `EmbeddingProvider` trait for pluggable embedding backends (sync; see doc comment for async guidance).
- `src/events.rs` -- `GraphEvent` enum and `SubscriptionId` type for event subscriptions.
- `src/openai.rs` -- `OpenAIEmbeddings` provider (behind `openai` feature flag).
- `src/error.rs` -- `Error` enum and `Result<T>` alias.
- `tests/integration.rs` -- 130 integration tests covering all features.
- `tests/async_integration.rs` -- 2 async integration tests (behind `async` feature flag).
- `examples/basic.rs` -- Basic usage example (nodes, edges, queries, traversal).
- `examples/agent_memory.rs` -- Agent memory example (sessions, preferences, summaries, decay).
- `examples/embedding_search.rs` -- Embedding search example (mock provider, semantic search).
- `benches/graph_bench.rs` -- Criterion benchmarks (insert, lookup, search, traversal, export/import).

## Architecture & Conventions

### CozoDB Patterns
- **Named field access**: Use `*node{uid, field1, field2}` for partial reads, positional `*node[f1, f2, ...]` for full row access.
- **Aggregation**: Use `?[count(uid)]` in the output header, not inline expressions.
- **Pagination**: Append `:limit N+1 :offset M` to queries, then check if `rows.len() > limit` for `has_more`.
- **Tombstone sentinel**: `tombstone_at == 0.0` means "live". CozoDB has no nullable numeric columns, so 0.0 is the sentinel.
- **Params**: Use `$param_name` in queries with `BTreeMap<String, DataValue>` params. String values via `str_val()` helper.
- **Batch inserts**: Use multi-row `<- [[$p0_uid, ...], [$p1_uid, ...]]` syntax with indexed params, chunked in groups of 100.
- **FTS**: `::fts create` indices on node label/summary; query via `~node:label_fts{ uid, label | query: $q, k: $k, bind_score: score }`.
- **Hard delete**: Use `:rm relation { key_cols }` syntax for purge operations.
- **Export/Import**: Use CozoDB's `export_relations()` / `import_relations()` Rust API with `NamedRows` conversion.
- **HNSW**: `::hnsw create` for vector indices on `node_embedding`; query via `~node_embedding:semantic_idx{ uid | query: vec([...]), k: K, ef: E, bind_distance: dist }`.
- **Meta store**: `mg_meta` key-value relation for storing config (e.g., embedding dimension). CozoDB rejects underscore-prefixed relation names.

### Traversal
- Graph traversal uses optimized 2-query BFS: (1) fetch all live edges, (2) BFS in-memory, (3) batch-fetch node metadata.
- Recursive Datalog was tested but is unreliable on CozoDB 0.7 — the 2-query approach reduces O(N) queries to exactly 2.
- `traverse_reachable` in `storage/cozo.rs` is the core traversal primitive; `graph.rs` wraps it with semantic methods like `reasoning_chain`.
- `PathStep` includes `parent_uid` for backtracking actual paths via `find_path`.
- `PathStep.node_type` is `NodeType` enum; `PathStep.edge_type` is `Option<EdgeType>` (None for start node).
- `TraversalOptions.weight_threshold` filters edges by minimum weight during traversal.

### Naming
- Storage methods are prefixed: `query_*`, `insert_*`, `count_*`, `validate_*`, `purge_*`, `export_*`, `import_*`.
- Graph-level methods use domain names: `active_goals()`, `reasoning_chain()`, `tombstone_cascade()`, `search()`, `find_nodes()`, `merge_entities()`, `batch_apply()`.
- Builder types: `NodeUpdate<'a>`, `EdgeUpdate<'a>` with `.apply()` terminal.
- Paginated variants are suffixed: `*_paginated`.
- Memory constructors: `add_session()`, `add_preference()`, `add_summary()` (`add_memory()` is deprecated).

### Props System
- `NodeProps` and `EdgeProps` are `#[serde(tag = "_type")]` enums serialized as JSON.
- Each variant wraps a `*Props` struct from `schema/props/`.
- All props structs derive `Default` for ergonomic construction with `..Default::default()`.
- `NodeProps::to_json()` / `NodeProps::from_json()` handle ser/de at the storage boundary.

### Type Conventions
- `Uid` inner field is private; use `Uid::as_str()`, `Uid::from()`, or `Uid::new()`.
- `CreateNode.uid` is `Option<Uid>` — set via `.with_uid()` for pre-assigned UIDs (used by `validate_batch`).
- `GraphEdge.layer` is `Layer` (not `String`).
- `reasoning_chain()` includes the starting node at depth 0.
- `GraphOp::AddNode` and `GraphOp::AddEdge` wrap `Box<CreateNode>` / `Box<CreateEdge>` to keep enum size small.
- `GraphEvent`, `GraphStats`, `DecayResult`, `BatchResult` all implement `Display`.
- `GraphEvent::EdgeTombstoned` is a struct variant with `uid`, `from_uid`, `to_uid`, `edge_type` fields.
- `NodeFilter.node_types` (multi-type OR filter) takes precedence over `node_type` if both set.
- `TypedSnapshot.embeddings` stores `Vec<(Uid, Vec<f32>)>` with `#[serde(default)]` for backward compat.

### Default Agent
- `MindGraph` has a `default_agent` field (default: `"system"`).
- `set_default_agent()` / `default_agent()` get/set the identity.
- `NodeUpdate` and `EdgeUpdate` builders fall back to `default_agent()` when `changed_by` is not set.

### Async
- `AsyncMindGraph` (behind `async` feature) wraps `Arc<MindGraph>` and delegates via `spawn_blocking`.
- Builder types (`NodeUpdate`, `EdgeUpdate`) can't cross await points; use `update_node()` / `update_edge()` directly.
- All methods have async wrappers. Event methods (`on_change`, `unsubscribe`) don't use `spawn_blocking`.
- `spawn_blocking` join errors return `Error::TaskJoin` instead of panicking.

### Tracing
- Optional `tracing` feature flag for observability.
- Key `MindGraph` methods are instrumented with `#[cfg_attr(feature = "tracing", tracing::instrument(skip(self, ...)))]`.
- Instrumented methods: `add_node`, `get_node`, `get_live_node`, `add_edge`, `tombstone`, `tombstone_cascade`, `search`, `find_nodes`, `reachable`, `reasoning_chain`, `semantic_search`, `embed_nodes`, `batch_apply`, `merge_entities`, `decay_salience`, `stats`.

### Embeddings
- `EmbeddingProvider` trait is sync; the `openai` feature uses blocking HTTP via `ureq`.
- `AsyncMindGraph` wraps embedding calls via `spawn_blocking`.
- `embed_nodes()` does bulk embedding: fetches live nodes, calls `embed_batch()`, stores vectors.
- `semantic_search()` over-fetches and retries to compensate for tombstoned nodes (up to k*10).

### Error Handling
- All fallible operations return `crate::Result<T>` (alias for `std::result::Result<T, crate::Error>`).
- Storage helpers (`extract_string`, `extract_float`, etc.) return typed errors on mismatches.
- `Confidence::new()` and `Salience::new()` validate the 0.0-1.0 range.
- `Error::TaskJoin` wraps `tokio::task::JoinError` from `spawn_blocking` (no more panics).
- RwLock poisoning is handled via `.unwrap_or_else(|e| e.into_inner())` — no panics on poisoned locks.

### Testing
- All tests use `MindGraph::open_in_memory()` via the `mem_graph()` helper.
- Helper functions `make_*_node()` create common node types with sensible defaults.
- Tests are grouped by feature area with `// ==== Phase N ====` section comments.
- Async tests require `cargo test --features async`.
