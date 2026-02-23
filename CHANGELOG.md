# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2026-02-23

### Added
- `get_edge_between(from, to, edge_type?)` to query edges between specific nodes
- `list_nodes(pagination)` for paginated listing of all live nodes
- `clear()` method for resetting the graph (testing/development)
- `PartialEq` derive on all props structs, `GraphNode`, `GraphEdge`, `PathStep`, `GraphEvent`, `NodeProps`, `EdgeProps`
- `Serialize`/`Deserialize` on `Pagination`, `Page<T>`, `PropCondition`, `PropOp`, `NodeFilter`, `SearchOptions`
- `Serialize`/`Deserialize` on result types: `PurgeResult`, `MergeResult`, `ImportResult`, `TombstoneResult`, `BatchResult`, `DecayResult`, `TypedImportResult`
- `Display` impl for `PurgeResult`, `MergeResult`, `ImportResult`, `TombstoneResult`, `TypedImportResult`
- `Debug` derive on `GraphOp`
- `Default` impl for `Pagination` (limit: 100, offset: 0)
- Optional `tracing` feature for instrumented graph operations
- `TaskJoin` error variant for graceful async error handling
- Criterion benchmarks (`benches/graph_bench.rs`)
- Three runnable examples: `basic`, `agent_memory`, `embedding_search`
- GitHub Actions CI workflow

### Changed
- `AsyncMindGraph` methods now return `Error::TaskJoin` instead of panicking on `spawn_blocking` failures
- `RwLock` poisoning in `MindGraph` now uses `unwrap_or_else(|e| e.into_inner())` instead of panicking
- Doc comments added to all async methods and builder methods

### Fixed
- 77 potential panics in `async_graph.rs` replaced with proper error propagation
- 7 potential panics from RwLock poisoning in `graph.rs` replaced with graceful recovery

## [0.4.1] - 2025-01-15

### Added
- `add_session()`, `add_preference()`, `add_summary()` convenience constructors
- `add_link()` for creating edges with default props
- Deprecated `add_memory()` in favor of `add_session()`
- Async wrappers for all v0.4 methods
- `TypedSnapshot.embeddings` with `#[serde(default)]` for backward compatibility

### Changed
- Memory constructors renamed: `add_memory()` -> `add_session()`

## [0.4.0] - 2025-01-10

### Added
- Embedding support: `configure_embeddings()`, `set_embedding()`, `get_embedding()`, `semantic_search()`
- `EmbeddingProvider` trait with `OpenAIEmbeddings` implementation (behind `openai` feature)
- CozoDB HNSW vector indices for semantic search
- Salience decay: `decay_salience()`, `auto_tombstone()`
- Event subscriptions: `on_change()`, `unsubscribe()`
- Graph statistics: `stats()` returning `GraphStats`
- Convenience constructors: `add_claim()`, `add_entity()`, `add_goal()`, `add_observation()`
- Default agent identity: `set_default_agent()`, `default_agent()`
- `AsyncMindGraph` wrapper (behind `async` feature)
- Typed export/import: `export_typed()`, `import_typed()`
- Validated batch: `validate_batch()`, `apply_validated_batch()`
- Enhanced query composition: OR filters, time ranges, salience ranges, `connected_to`
- Full-text search on label and summary fields
- Node builder pattern: `update()` with fluent API
- Edge builder pattern: `update_edge_builder()` with fluent API
- Pagination support across all query methods
- Batch node/edge insertion with chunked multi-row inserts
- Version history: `node_history()`, `edge_history()`, `node_at_version()`
- Entity resolution: `merge_entities()`, `fuzzy_resolve()`
- Data lifecycle: `purge_tombstoned()`, `export()`, `import()`, `backup()`, `restore_backup()`
- 48 node types, 70 edge types with typed property structs
- 125+ integration tests
