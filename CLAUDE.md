# CLAUDE.md

## Build & Test

```bash
# Library crate
cargo build                              # Build the library
cargo test                               # Run all tests (158 integration + 13 doc-tests)
cargo test --features async              # Run all tests including async (158 + 7 async + 13 doc-tests)
cargo clippy --all-features -- -W clippy::all  # Lint (must produce 0 warnings)
cargo doc --no-deps --all-features       # Build docs with doc-tests
cargo bench                              # Run criterion benchmarks
cargo run --example basic                # Run basic example
cargo run --example agent_memory         # Run agent memory example
cargo run --example embedding_search --features async  # Run embedding search example
cargo publish -p mindgraph --dry-run --allow-dirty    # Verify publishability (library only)

# Server crate
cargo build -p mindgraph-server          # Build the server
cargo clippy -p mindgraph-server -- -W clippy::all  # Lint server (must produce 0 warnings)
MINDGRAPH_DB_PATH=:memory: MINDGRAPH_TOKEN=test cargo run -p mindgraph-server  # Run server in-memory
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
- `src/embeddings.rs` -- `EmbeddingProvider` trait (sync) and `AsyncEmbeddingProvider` trait (behind `async` feature), plus `SyncProviderAdapter`.
- `src/events.rs` -- `GraphEvent` enum, `SubscriptionId`, `EventKind`, `EventFilter` for filtered subscriptions.
- `src/watch.rs` -- `WatchStream` async filtered event stream (behind `async` feature).
- `src/agent.rs` -- `AgentHandle` scoped per-agent graph handle for multi-agent systems.
- `src/openai.rs` -- `OpenAIEmbeddings` provider (behind `openai` feature flag).
- `src/error.rs` -- `Error` enum and `Result<T>` alias.
- `tests/integration.rs` -- 146 integration tests covering all features.
- `tests/async_integration.rs` -- 5 async integration tests (behind `async` feature flag).
- `examples/basic.rs` -- Basic usage example (nodes, edges, queries, traversal).
- `examples/agent_memory.rs` -- Agent memory example (sessions, preferences, summaries, decay).
- `examples/embedding_search.rs` -- Embedding search example (mock provider, semantic search).
- `benches/graph_bench.rs` -- Criterion benchmarks (insert, lookup, search, traversal, export/import).
- `mindgraph-server/src/main.rs` -- Axum app setup: routes, auth middleware, `AppState`, shared helpers/parsers (31 CRUD endpoints).
- `mindgraph-server/src/handlers.rs` -- Cognitive layer handlers: 18 higher-level endpoints that compose multiple graph ops.
- `mindgraph-server/Cargo.toml` -- Server binary crate manifest (depends on `mindgraph` with `async` feature).

## Workspace Layout

This repo is a Cargo workspace with two members:
- **`mindgraph`** (root) -- Library crate, published to crates.io.
- **`mindgraph-server`** -- Binary crate, not published. Thin Axum HTTP layer over `AsyncMindGraph`.

`cargo publish -p mindgraph` publishes only the library. The server is a separate workspace member and is not included in the published crate tarball.

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
- All `GraphEvent` variants are struct variants with a `changed_by: String` field.
- `GraphEvent::NodeUpdated` and `NodeTombstoned` carry `node_type` and `layer` for filtering.
- `GraphEvent::changed_by()` accessor returns the agent identity that triggered the event.
- `NodeFilter.node_types` (multi-type OR filter) takes precedence over `node_type` if both set.
- `TypedSnapshot.embeddings` stores `Vec<(Uid, Vec<f32>)>` with `#[serde(default)]` for backward compat.

### Default Agent
- `MindGraph` has a `default_agent` field (default: `"system"`).
- `set_default_agent()` / `default_agent()` get/set the identity.
- `add_node()`, `add_edge()`, and batch methods use `default_agent()` for version records.
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

### Custom Types
- `NodeType::Custom(String)` and `EdgeType::Custom(String)` allow user-defined types without forking the crate.
- `CustomNodeType` trait: implement `type_name()` and `layer()` to register a custom type with typed ser/de.
- `MindGraph::add_custom_node<T>(label, props)` wraps typed data into `NodeProps::Custom`.
- `GraphNode::custom_props::<T>()` deserializes back to the original type.
- **Breaking change (v0.6):** `NodeType` and `EdgeType` no longer implement `Copy` (they implement `Clone`).
- Storage parsers fall back to `Custom(name)` for unknown type strings, enabling forward compatibility.

### Multi-Agent
- `AgentHandle` and `AsyncAgentHandle` derive `Clone`.
- `AgentHandle` provides a scoped graph handle with a fixed agent identity.
- Created via `graph.agent("alice")` (requires `Arc<MindGraph>`).
- All mutation methods (`add_node`, `add_edge`, `tombstone`, `tombstone_edge`, `update_edge`, etc.) auto-set `changed_by`.
- Read methods: `get_node`, `get_live_node`, `get_edge`, `get_edge_between`, `edges_from`, `edges_to`, `search`, `find_nodes`.
- Traversal methods: `reachable`, `reasoning_chain`, `neighborhood`.
- History/stats: `node_history`, `count_nodes`, `node_exists`, `stats`.
- Convenience constructors: `add_entity`, `add_claim`, `add_goal`, `add_observation`, `add_session`, `add_preference`, `add_summary`, `add_link`, `add_custom_node`.
- `sub_agent("sub")` creates child handles with `parent_agent` tracking.
- `my_nodes()` queries nodes created by this agent (via `node_version` where version=1).
- `AsyncAgentHandle` wraps `AgentHandle` for async contexts with all corresponding async methods.
- Internal `_as` methods: `add_node_as`, `add_edge_as`, `update_edge_as`, `tombstone_cascade_as`.

### Filtered Events & Streaming
- `EventKind` enum: `NodeAdded`, `NodeUpdated`, `NodeTombstoned`, `EdgeAdded`, `EdgeTombstoned`.
- `EventFilter` with builder methods: `.node_types()`, `.edge_types()`, `.layers()`, `.event_kinds()`, `.agent()`.
- `EventFilter::matches()` checks all fields including `agent_id` against `event.changed_by()`.
- `on_change_filtered(filter, cb)` for sync filtered subscriptions.
- `MindGraph::watch(filter) -> WatchStream` (behind `async` feature) for async streaming via `tokio::sync::broadcast`.
- `WatchStream` implements `futures_core::Stream` for use with `StreamExt`, `select!`, etc.
- `WatchStream::recv()` loops on broadcast receiver, applies filter, handles `Lagged` by continuing.
- `WatchStream::lagged_count()` returns total events dropped due to broadcast lag.
- `open_with_broadcast_capacity()` and `open_in_memory_with_broadcast_capacity()` (behind `async` feature) allow custom channel size.
- `GraphNode::custom_props::<T>()` returns `Result<Option<T>>` (not `Option<T>`).

### Embeddings
- `EmbeddingProvider` trait is sync; the `openai` feature uses blocking HTTP via `ureq`.
- `AsyncEmbeddingProvider` trait (behind `async` feature) for native async embedding without `spawn_blocking`.
- `SyncProviderAdapter` wraps a sync `EmbeddingProvider` as an `AsyncEmbeddingProvider` via `spawn_blocking`.
- `AsyncMindGraph` has both sync wrappers (`embed_node`, `semantic_search_text`) and native async methods (`embed_node_async`, `embed_nodes_async`, `semantic_search_text_async`).
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

### mindgraph-server Conventions
- **Auth**: Bearer token middleware via `axum::middleware::from_fn_with_state`. Applied as `route_layer` on all routes except `/health`. New routes are automatically protected — no per-handler auth calls needed.
- **Agent identity**: Each mutation handler extracts `agent_id` from the request body (defaults to `MINDGRAPH_DEFAULT_AGENT` env var or `"system"`). Mutations use `AsyncAgentHandle` via `graph.agent(&agent_id)`.
- **Error handling**: `map_err_500()` logs via `tracing::error!` and returns JSON `{"error": "..."}`. `bad_request()` and `not_found()` for 4xx. All handlers return `Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)>`.
- **Enum parsing**: `parse_node_type()`, `parse_edge_type()`, `parse_layer()` helper functions convert strings to enums at handler boundaries. Unknown node/edge types fall through to `Custom(name)`. Unknown layers return `None` (400 error).
- **UID conversion**: Incoming strings converted via `Uid::from(s.as_str())` at handler boundaries. Outgoing UIDs serialized automatically via serde.
- **POST /node**: `props` field is `NodeProps` (serde-tagged union, not raw JSON). Callers include `{"_type": "Entity", ...}` in the props object.
- **POST /link vs POST /edge**: `/link` takes an edge type string and uses `EdgeProps::default_for(edge_type)`. `/edge` takes full `EdgeProps` for setting specific edge fields.
- **GET /nodes**: Returns `serde_json::Value` to unify two code paths — `Vec<GraphNode>` when filtered by agent (via `my_nodes()`) and `Page<GraphNode>` when using `find_nodes_paginated`.
- **Defaults**: `ClaimRequest.confidence` defaults to 0.5 (not 1.0). `SessionRequest.focus` is `Option<String>` (unwrapped to empty string).
- **Cognitive layer (`handlers.rs`)**: All 18 cognitive endpoints live in `src/handlers.rs` as a `pub(crate)` module. Shared helpers (`AppState`, `ErrorResponse`, `default_agent`, `map_err_500`, `bad_request`, `not_found`, `parse_*`) are re-exported as `pub(crate)` from `main.rs`. `parse_direction()` and `create_link()` are private helpers local to `handlers.rs`. Bundle endpoints (e.g., `/epistemic/argument`) use best-effort atomicity — partial failures leave earlier nodes persisted; callers can clean up via `POST /evolve { action: "tombstone_cascade" }`.
