# mindgraph

[![Crates.io](https://img.shields.io/crates/v/mindgraph.svg)](https://crates.io/crates/mindgraph)
[![Documentation](https://docs.rs/mindgraph/badge.svg)](https://docs.rs/mindgraph)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/shuruheel/mindgraph-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/shuruheel/mindgraph-rs/actions/workflows/ci.yml)

A structured semantic memory graph for agentic systems, built in Rust with [CozoDB](https://www.cozodb.org/) as the embedded Datalog storage engine.

## Overview

`mindgraph` provides a typed, versioned knowledge graph organized into six conceptual layers:

| Layer | Purpose | Example Node Types |
|-------|---------|-------------------|
| **Reality** | Raw observations & sources | Source, Snippet, Entity, Observation |
| **Epistemic** | Reasoning & knowledge | Claim, Evidence, Hypothesis, Theory, Concept |
| **Intent** | Goals & decisions | Goal, Project, Decision, Option, Constraint |
| **Action** | Affordances & workflows | Affordance, Flow, FlowStep, Control |
| **Memory** | Persistence & recall | Session, Trace, Summary, Preference |
| **Agent** | Control plane | Agent, Task, Plan, Approval, Policy |

The graph supports **48 node types** and **70 edge types**, each with type-safe property structs.

## Features

- **Type-safe schema** -- 48 node types and 70 edge types as Rust enums with typed props
- **CozoDB storage** -- Embedded Datalog database with SQLite persistence or in-memory mode
- **Full-text search** -- FTS indices on node labels and summaries with scoring and type/layer filters
- **Structured filtering** -- `NodeFilter` builder for type (single or multi-type), layer, label substring, prop value, and confidence range queries
- **Graph traversal** -- Optimized 2-query BFS, reasoning chains, neighborhoods, path finding, subgraph extraction, weight threshold filtering
- **Builder pattern** -- Ergonomic fluent API for node and edge updates
- **Pagination** -- Bounded result sets with `has_more` detection for production use
- **Batch operations** -- Multi-row inserts (chunked at 100) and `GraphOp`-based batch apply
- **Versioning** -- Append-only version history for both nodes and edges, with point-in-time snapshots
- **Tombstone cascade** -- Soft-delete a node and all connected edges in one call
- **Data lifecycle** -- `purge_tombstoned()` for hard-deleting old data; `export()`/`import()` for graph snapshots; `backup()`/`restore_backup()` for file-level backups
- **Provenance tracking** -- Link extracted knowledge to its sources
- **Entity resolution** -- Alias table, fuzzy matching, `merge_entities()` for deduplication
- **Default agent identity** -- `set_default_agent()` reduces boilerplate in builder patterns
- **Confidence & salience** -- Validated 0.0-1.0 scores on all nodes and edges
- **Thread safety** -- `MindGraph` is `Send + Sync`, safe to share via `Arc<MindGraph>` or `into_shared()`
- **Async support** -- Optional `AsyncMindGraph` wrapper for tokio runtimes (feature flag: `async`) with all methods
- **Server-side query filtering** -- Query patterns push filtering into CozoDB Datalog for efficient large-graph queries
- **Embedding/vector search** -- Pluggable `EmbeddingProvider` trait (sync; `AsyncMindGraph` wraps via `spawn_blocking`), CozoDB HNSW indices, `semantic_search()` with cosine distance
- **Salience decay** -- Exponential decay with configurable half-life via `decay_salience()`, plus `auto_tombstone()` for cleanup
- **Event subscriptions** -- `on_change()` callback system for reactive patterns on node/edge mutations
- **Convenience constructors** -- `add_claim()`, `add_entity()`, `add_goal()`, `add_observation()`, `add_session()`, `add_preference()`, `add_summary()`, `add_link()`
- **Graph statistics** -- `stats()` returns comprehensive `GraphStats` with counts by type/layer
- **Enhanced query composition** -- OR filters, time ranges, salience ranges, prop conditions, graph-aware `connected_to` filter
- **Typed export/import** -- `export_typed()` / `import_typed()` with `TypedSnapshot` for structured graph transfer
- **Validated batch** -- `validate_batch()` pre-validates operations before `apply_validated_batch()`
- **OpenAI embeddings** -- Optional `openai` feature flag for `OpenAIEmbeddings` provider via `ureq`
- **Tracing integration** -- Optional `tracing` feature flag for observability instrumentation on key graph methods
- **Production-safe async** -- `AsyncMindGraph` returns `Error::TaskJoin` instead of panicking on spawn failures

## Quick Start

```rust
use mindgraph::*;

fn main() -> Result<()> {
    // Open a persistent graph (SQLite-backed)
    let graph = MindGraph::open("my_graph.db")?;
    // Or in-memory for testing:
    // let graph = MindGraph::open_in_memory()?;

    // Add a claim node
    let claim = graph.add_node(
        CreateNode::new("Rust is memory safe", NodeProps::Claim(ClaimProps {
            content: "Rust is memory safe".into(),
            claim_type: Some("factual".into()),
            ..Default::default()
        }))
        .confidence(Confidence::new(0.95)?)
    )?;

    // Add supporting evidence
    let evidence = graph.add_node(
        CreateNode::new("Borrow checker", NodeProps::Evidence(EvidenceProps {
            description: "Borrow checker prevents dangling pointers".into(),
            ..Default::default()
        }))
    )?;

    // Connect with a typed edge (evidence supports claim)
    graph.add_edge(CreateEdge::new(
        evidence.uid.clone(),
        claim.uid.clone(),
        EdgeProps::Supports { strength: Some(0.9), support_type: Some("empirical".into()) },
    ))?;

    // Update using the builder pattern
    graph.update(&claim.uid)
        .confidence(Confidence::new(0.99)?)
        .changed_by("agent-1")
        .reason("strong supporting evidence")
        .apply()?;

    // Traverse the reasoning chain (includes start node at depth 0)
    let chain = graph.reasoning_chain(&claim.uid, 5)?;
    assert_eq!(chain[0].node_uid, claim.uid); // start node
    assert_eq!(chain[0].depth, 0);

    Ok(())
}
```

## Async Usage

Enable the `async` feature for tokio integration:

```toml
[dependencies]
mindgraph = { version = "0.5", features = ["async"] }
```

```rust
use mindgraph::*;

#[tokio::main]
async fn main() -> Result<()> {
    let graph = AsyncMindGraph::open_in_memory().await?;

    let node = graph.add_node(
        CreateNode::new("Async claim", NodeProps::Claim(ClaimProps {
            content: "Works in async contexts".into(),
            ..Default::default()
        }))
    ).await?;

    // AsyncMindGraph is Clone (wraps Arc<MindGraph>),
    // so it can be shared across tasks
    let g = graph.clone();
    let handle = tokio::spawn(async move {
        g.count_nodes(NodeType::Claim).await
    });
    assert_eq!(handle.await.unwrap()?, 1);

    // For updates, use update_node/update_edge directly
    // (builder types hold references and can't cross await points)
    graph.update_node(
        node.uid,
        Some("Updated claim".into()),
        None, None, None, None,
        "agent".into(), "async update".into(),
    ).await?;

    Ok(())
}
```

## Tracing

Enable the `tracing` feature for observability:

```toml
[dependencies]
mindgraph = { version = "0.5", features = ["tracing"] }
```

Key methods (`add_node`, `search`, `find_nodes`, `reachable`, `stats`, etc.) are instrumented with `tracing::instrument`. Combine with `tracing-subscriber` to get structured logs.

## API Reference

### MindGraph

The main entry point. All operations go through this struct. It is `Send + Sync` and can be shared across threads via `Arc<MindGraph>`.

**Construction:**

| Method | Description |
|--------|-------------|
| `MindGraph::open(path)` | Open a persistent SQLite-backed graph |
| `MindGraph::open_in_memory()` | Create an in-memory graph (for testing) |
| `into_shared()` | Wrap in `Arc<MindGraph>` for sharing across threads |
| `set_default_agent(name)` | Set default agent identity for builder fallbacks |
| `default_agent()` | Get current default agent identity |
| `storage()` | Access the underlying `CozoStorage` for advanced Datalog queries |

**Convenience constructors:**

| Method | Description |
|--------|-------------|
| `add_claim(label, content, confidence)` | Add a Claim node with defaults |
| `add_entity(label, entity_type)` | Add an Entity node with defaults |
| `add_goal(label, priority)` | Add a Goal node with defaults |
| `add_observation(label, description)` | Add an Observation node with defaults |
| `add_session(label, focus)` | Add a Session node with defaults |
| `add_preference(label, key, value)` | Add a Preference node with defaults |
| `add_summary(label, content)` | Add a Summary node with defaults |
| `add_memory(label, content)` | **Deprecated** -- use `add_session()` instead |
| `add_link(from, to, edge_type)` | Add an edge with default props for the edge type |

**Node operations:**

| Method | Description |
|--------|-------------|
| `add_node(CreateNode)` | Add a new node (auto-assigns UID, version 1) |
| `add_nodes_batch(Vec<CreateNode>)` | Bulk insert multiple nodes (multi-row, chunked at 100) |
| `get_node(uid)` | Get a node by UID, returns `None` if not found |
| `get_live_node(uid)` | Get a node, errors if not found or tombstoned |
| `update_node(uid, ...)` | Update fields directly (increments version) |
| `update(uid)` | Begin a builder-pattern update, finalize with `.apply()` |
| `node_exists(uid)` | Check if a live node exists (O(1), no deserialization) |
| `count_nodes(node_type)` | Count live nodes of a given type |
| `count_nodes_in_layer(layer)` | Count live nodes in a given layer |

**Edge operations:**

| Method | Description |
|--------|-------------|
| `add_edge(CreateEdge)` | Add a new edge (validates both endpoints are live) |
| `add_edges_batch(Vec<CreateEdge>)` | Bulk insert edges (validates all endpoints first) |
| `get_edge(uid)` | Get an edge by UID, returns `None` if not found |
| `get_live_edge(uid)` | Get an edge, errors if not found or tombstoned |
| `update_edge(uid, ...)` | Update fields directly (increments version) |
| `update_edge_builder(uid)` | Begin a builder-pattern update, finalize with `.apply()` |
| `edges_from(uid, edge_type?)` | Get all live edges from a node, optionally filtered by type |
| `edges_to(uid, edge_type?)` | Get all live edges to a node, optionally filtered by type |
| `count_edges(edge_type)` | Count live edges of a given type |
| `get_edge_between(from, to, edge_type?)` | Find edges between two nodes, optionally by type |

**Traversal:**

| Method | Description |
|--------|-------------|
| `reachable(uid, opts)` | BFS to find all nodes reachable through filtered edge types |
| `reasoning_chain(uid, max_depth)` | Traverse epistemic edges; returns start node at depth 0 |
| `neighborhood(uid, depth)` | Get all nodes within `depth` hops in any direction |
| `find_path(from, to, opts)` | Find the actual shortest path between two nodes |
| `subgraph(uid, opts)` | Extract all reachable nodes and their interconnecting edges |

**Tombstone operations:**

| Method | Description |
|--------|-------------|
| `tombstone(uid, reason, by)` | Soft-delete a node with audit trail |
| `restore(uid)` | Restore a tombstoned node |
| `tombstone_edge(uid, reason, by)` | Soft-delete an edge with audit trail |
| `restore_edge(uid)` | Restore a tombstoned edge |
| `tombstone_cascade(uid, reason, by)` | Tombstone a node **and** all connected edges |

**Version history:**

| Method | Description |
|--------|-------------|
| `node_history(uid)` | Get full version history (create, updates, tombstone) |
| `edge_history(uid)` | Get full version history for an edge |
| `node_at_version(uid, version)` | Get the JSON snapshot at a specific version number |

**Search & filtering:**

| Method | Description |
|--------|-------------|
| `search(query, opts)` | Full-text search across labels/summaries with FTS scoring |
| `find_nodes(filter)` | Structured filtering by type, layer, label, props, confidence |
| `find_nodes_paginated(filter)` | Same as above with `Page<GraphNode>` pagination metadata |

**Data lifecycle:**

| Method | Description |
|--------|-------------|
| `purge_tombstoned(older_than)` | Hard-delete tombstoned data (and associated versions/aliases/provenance) |
| `export()` | Export entire graph as a `GraphSnapshot` |
| `import(snapshot)` | Import a graph snapshot (additive merge) |
| `backup(path)` | Backup database to a file |
| `restore_backup(path)` | Restore database from a backup file |

**Provenance & entity resolution:**

| Method | Description |
|--------|-------------|
| `add_provenance(record)` | Link a node to its extraction source |
| `add_alias(text, canonical_uid, score)` | Register an alias for entity resolution |
| `resolve_alias(text)` | Resolve text to a canonical entity UID |
| `aliases_for(uid)` | List all aliases for a canonical entity, sorted by score |
| `merge_entities(keep, merge, reason, by)` | Merge two entities: retarget edges/aliases, tombstone duplicate |
| `fuzzy_resolve(text, limit)` | Substring match on alias text |

**Embedding/vector search:**

| Method | Description |
|--------|-------------|
| `configure_embeddings(dimension)` | Initialize HNSW index for semantic search |
| `embedding_dimension()` | Get configured embedding dimension (None if not configured) |
| `set_embedding(uid, vec)` | Store an embedding vector for a node |
| `get_embedding(uid)` | Retrieve a node's embedding vector |
| `delete_embedding(uid)` | Remove a node's embedding |
| `semantic_search(query_vec, k)` | Find k nearest neighbors by cosine distance (auto-compensates for tombstoned nodes) |
| `embed_node(uid, provider)` | Generate and store embedding via `EmbeddingProvider` |
| `embed_nodes(uids, provider)` | Bulk embed multiple nodes via `embed_batch()`, skips tombstoned |
| `semantic_search_text(query, k, provider)` | Embed query text and search |

**Salience decay:**

| Method | Description |
|--------|-------------|
| `decay_salience(half_life_secs)` | Apply exponential decay to all live nodes |
| `auto_tombstone(min_salience, min_age_secs)` | Tombstone old nodes below salience threshold |

**Event subscriptions:**

| Method | Description |
|--------|-------------|
| `on_change(callback)` | Subscribe to graph mutation events, returns `SubscriptionId` |
| `unsubscribe(id)` | Remove a subscription |

**Statistics:**

| Method | Description |
|--------|-------------|
| `stats()` | Get comprehensive `GraphStats` (counts by type, layer, embeddings, etc.) |

**Utility:**

| Method | Description |
|--------|-------------|
| `list_nodes(pagination)` | List all live nodes with pagination |
| `clear()` | Delete all data from all relations (for testing/reset) |

**Typed export/import:**

| Method | Description |
|--------|-------------|
| `export_typed()` | Export live graph as `TypedSnapshot` with structured nodes/edges/embeddings |
| `import_typed(snapshot)` | Import a typed snapshot (additive merge, skips existing UIDs, restores embeddings) |

**Batch operations (GraphOp):**

| Method | Description |
|--------|-------------|
| `batch_apply(ops)` | Execute a batch of AddNode/AddEdge/Tombstone operations |
| `validate_batch(ops)` | Pre-validate a batch (auto-assigns UIDs, tracks cross-refs), returns `ValidatedBatch` |
| `apply_validated_batch(batch)` | Apply a pre-validated batch |

**Query patterns (server-side filtered via CozoDB Datalog):**

| Method | Description |
|--------|-------------|
| `active_goals()` | Goals with `status == "active"`, ranked by priority |
| `pending_approvals()` | Approvals with `status == "pending"`, sorted by requested_at |
| `unresolved_contradictions()` | CONTRADICTS edges with `resolution_status == "unresolved"` |
| `open_decisions()` | Decisions with status `"open"` or `"deliberating"` |
| `open_questions()` | OpenQuestions with status `"open"` or `"partially_addressed"` |
| `weak_claims(threshold)` | Claims with `confidence < threshold`, sorted ascending |
| `nodes_in_layer(layer)` | All live nodes in a given layer |

**Paginated variants:**

| Method | Description |
|--------|-------------|
| `nodes_in_layer_paginated(layer, page)` | Paginated nodes in a layer |
| `edges_from_paginated(uid, edge_type?, page)` | Paginated edges from a node |
| `edges_to_paginated(uid, edge_type?, page)` | Paginated edges to a node |
| `weak_claims_paginated(threshold, page)` | Paginated weak claims |
| `active_goals_paginated(page)` | Paginated active goals, sorted by priority in DB |

### AsyncMindGraph

Available behind the `async` feature flag. Wraps `Arc<MindGraph>` and exposes async versions of all methods via `tokio::task::spawn_blocking`.

| Method | Description |
|--------|-------------|
| `AsyncMindGraph::open(path)` | Async open |
| `AsyncMindGraph::open_in_memory()` | Async in-memory open |
| `AsyncMindGraph::from_sync(graph)` | Wrap an existing `MindGraph` |
| `inner()` | Access the underlying `&MindGraph` |

`AsyncMindGraph` is `Clone` and can be shared across tokio tasks. All methods from `MindGraph` are available as async variants, taking owned arguments instead of references.

**Note:** The builder types (`NodeUpdate`, `EdgeUpdate`) hold references and cannot cross `.await` points. Use `update_node()` / `update_edge()` directly in async code.

### Builders

**CreateNode** -- built with `CreateNode::new(label, props)`, with optional chained methods:
- `.summary(text)` -- set the node summary
- `.confidence(Confidence)` -- set epistemic certainty (default 1.0)
- `.salience(Salience)` -- set contextual relevance (default 0.5)
- `.privacy(PrivacyLevel)` -- set privacy level (default Private)
- `.with_uid(Uid)` -- pre-assign a UID (for cross-referencing in `validate_batch`)

**CreateEdge** -- built with `CreateEdge::new(from_uid, to_uid, props)`, with optional chained methods:
- `.confidence(Confidence)` -- set edge confidence (default 1.0)
- `.weight(f64)` -- set edge weight (default 0.5)

**NodeUpdate** -- started with `graph.update(uid)`:
```rust
graph.update(&uid)
    .label("Updated label")
    .summary("New summary")
    .confidence(Confidence::new(0.9)?)
    .salience(Salience::new(0.8)?)
    .changed_by("agent-1")
    .reason("new evidence")
    .apply()?;
```

**EdgeUpdate** -- started with `graph.update_edge_builder(uid)`:
```rust
graph.update_edge_builder(&edge_uid)
    .weight(0.95)
    .confidence(Confidence::new(0.9)?)
    .changed_by("agent-2")
    .reason("re-evaluated")
    .apply()?;
```

### Traversal

Control traversal behavior with `TraversalOptions`:

```rust
use mindgraph::*;

let opts = TraversalOptions {
    direction: Direction::Both,         // Outgoing, Incoming, or Both
    edge_types: Some(vec![              // None = follow all edge types
        EdgeType::Supports,
        EdgeType::Refutes,
    ]),
    max_depth: 5,                       // BFS depth limit
    weight_threshold: Some(0.5),        // None = no weight filter
};

let steps = graph.reachable(&start_uid, &opts)?;
for step in &steps {
    // node_type is NodeType enum, edge_type is Option<EdgeType>
    println!("depth {}: {} ({:?}) via {:?}, parent: {:?}",
        step.depth, step.label, step.node_type, step.edge_type, step.parent_uid);
}
```

`PathStep` includes `parent_uid` for backtracking. `find_path` uses this to return only the nodes on the actual shortest path (not all reachable nodes).

### Pagination

Use `Pagination` for bounded result sets:

```rust
use mindgraph::*;

// First page of 10 items
let page1 = graph.nodes_in_layer_paginated(Layer::Epistemic, Pagination::first(10))?;
assert!(page1.items.len() <= 10);

// Next page
if page1.has_more {
    let page2 = graph.nodes_in_layer_paginated(
        Layer::Epistemic,
        Pagination { limit: 10, offset: 10 },
    )?;
}
```

### Core Types

| Type | Description |
|------|-------------|
| `Uid` | UUID v4 identifier for nodes and edges (inner field is private) |
| `Confidence` | Validated f64 in 0.0-1.0 (epistemic certainty) |
| `Salience` | Validated f64 in 0.0-1.0 (contextual relevance, decays over time) |
| `PrivacyLevel` | `Private`, `Shared`, or `Public` |
| `Timestamp` | Unix timestamp as f64 |
| `NodeProps` | Discriminated union of all 48 node property structs |
| `EdgeProps` | Discriminated union of all 70 edge property structs |

### Schema

**48 node types** across 6 layers:

| Layer | Node Types |
|-------|-----------|
| Reality (4) | Source, Snippet, Entity, Observation |
| Epistemic (24) | Claim, Evidence, Warrant, Argument, Hypothesis, Theory, Paradigm, Anomaly, Method, Experiment, Concept, Assumption, Question, OpenQuestion, Analogy, Pattern, Mechanism, Model, ModelEvaluation, InferenceChain, SensitivityAnalysis, ReasoningStrategy, Theorem, Equation |
| Intent (6) | Goal, Project, Decision, Option, Constraint, Milestone |
| Action (5) | Affordance, Flow, FlowStep, Control, RiskAssessment |
| Memory (5) | Session, Trace, Summary, Preference, MemoryPolicy |
| Agent (8) | Agent, Task, Plan, PlanStep, Approval, Policy, Execution, SafetyBudget |

**70 edge types** across categories:

| Category | Edge Types |
|----------|-----------|
| Structural (5) | ExtractedFrom, PartOf, HasPart, InstanceOf, Contains |
| Epistemic (31) | Supports, Refutes, Justifies, HasPremise, HasConclusion, HasWarrant, Rebuts, Assumes, Tests, Produces, UsesMethod, Addresses, Generates, Extends, Supersedes, Contradicts, AnomalousTo, AnalogousTo, Instantiates, TransfersTo, Evaluates, Outperforms, FailsOn, HasChainStep, PropagatesUncertaintyTo, SensitiveTo, RobustAcross, Describes, DerivedFrom, ReliesOn, ProvenBy |
| Provenance (5) | ProposedBy, AuthoredBy, CitedBy, BelievedBy, ConsensusIn |
| Intent (9) | DecomposesInto, MotivatedBy, HasOption, DecidedOn, ConstrainedBy, Blocks, Informs, RelevantTo, DependsOn |
| Action (5) | AvailableOn, ComposedOf, StepUses, RiskAssessedBy, Controls |
| Memory (5) | CapturedIn, TraceEntry, Summarizes, Recalls, GovernedBy |
| Agent (10) | AssignedTo, PlannedBy, HasStep, Targets, RequiresApproval, ExecutedBy, ExecutionOf, ProducesNode, GovernedByPolicy, BudgetFor |

## Architecture

```
mindgraph
├── graph.rs          -- MindGraph: the main public API + NodeUpdate/EdgeUpdate builders
├── async_graph.rs    -- AsyncMindGraph: tokio wrapper (behind "async" feature)
├── storage/
│   ├── cozo.rs       -- CozoStorage: CozoDB CRUD, traversal, pagination, batch ops
│   └── migrations.rs -- Schema DDL (CozoDB :create statements + indices)
├── schema/
│   ├── mod.rs        -- Layer, NodeType (48), EdgeType (70) enums
│   ├── node.rs       -- GraphNode, CreateNode
│   ├── edge.rs       -- GraphEdge, CreateEdge
│   ├── node_props.rs -- NodeProps discriminated union
│   ├── edge_props.rs -- EdgeProps discriminated union
│   └── props/        -- Per-layer property structs
│       ├── reality.rs    (4 structs)
│       ├── epistemic.rs  (24 structs)
│       ├── intent.rs     (6 structs)
│       ├── action.rs     (5 structs)
│       ├── memory.rs     (5 structs)
│       └── agent.rs      (8 structs)
├── traversal.rs      -- Direction, TraversalOptions, PathStep
├── query.rs          -- Pagination, Page<T>, GraphStats, DecayResult, TypedSnapshot, etc.
├── types.rs          -- Uid, Confidence, Salience, PrivacyLevel, Timestamp
├── provenance.rs     -- ProvenanceRecord, ExtractionMethod
├── embeddings.rs     -- EmbeddingProvider trait
├── events.rs         -- GraphEvent enum, SubscriptionId
├── openai.rs         -- OpenAIEmbeddings (behind "openai" feature)
└── error.rs          -- Error types + Result alias
```

## Storage

CozoDB is used as the embedded storage engine. It runs Datalog queries over relations stored in SQLite (persistent) or in-memory (testing). The schema defines six core relations:

| Relation | Purpose | Key |
|----------|---------|-----|
| `node` | All graph nodes with universal metadata | `uid` |
| `edge` | All graph edges with typed properties | `uid` |
| `node_version` | Append-only node version snapshots | `(node_uid, version)` |
| `edge_version` | Append-only edge version snapshots | `(edge_uid, version)` |
| `provenance` | Extraction lineage records | `(node_uid, source_uid)` |
| `alias` | Entity resolution mappings | `(alias_text, canonical_uid)` |
| `mg_meta` | Key-value config store (e.g., embedding dimension) | `key` |
| `node_embedding` | Vector embeddings with HNSW index (created on demand) | `uid` |

Indices are created for edge traversal (`from_uid`, `to_uid`), node lookup (`node_type`, `layer`), provenance queries, and alias resolution.

## Design Decisions

- **Props as JSON columns** -- Node and edge properties are stored as JSON in CozoDB, with `NodeProps`/`EdgeProps` Rust enums providing type safety at the API boundary. This allows CozoDB Datalog to filter on props fields using `get(props, 'field', default)` without schema migration.
- **Tombstoning over deletion** -- Soft-delete preserves audit trails. Tombstoned entities are excluded from live queries but remain accessible for forensic review. `tombstone_cascade` removes a node and all its edges atomically.
- **Append-only versioning** -- Every mutation to a node or edge creates a new version snapshot, enabling full history reconstruction and point-in-time queries via `node_at_version`.
- **2-query BFS traversal** -- Graph traversal fetches all live edges in one query, runs BFS in-memory, then batch-fetches node metadata in a second query. This reduces traversal from O(N) database queries to exactly 2, regardless of graph size. Recursive CozoDB Datalog was tested but found unreliable across versions.
- **Server-side filtering** -- Query patterns like `active_goals()` and `weak_claims()` push filtering into CozoDB Datalog rather than loading all nodes into memory. Paginated variants (e.g., `active_goals_paginated`) sort in the database before applying `:limit`/`:offset`.
- **Tombstone sentinel** -- `tombstone_at` uses `0.0` as the sentinel value for "not tombstoned" since CozoDB columns use fixed types. All live-query filters check `tombstone_at == 0.0`.
- **Thread safety** -- `MindGraph` is `Send + Sync`. CozoDB's `DbInstance` uses internal locking, so `Arc<MindGraph>` works safely across threads.
- **Async via spawn_blocking** -- `AsyncMindGraph` wraps `Arc<MindGraph>` and delegates all operations to `tokio::task::spawn_blocking`. This avoids blocking the tokio runtime while leveraging CozoDB's synchronous API.
- **Private Uid inner field** -- `Uid(String)` keeps its inner field private to prevent accidental construction of invalid UIDs. Use `Uid::new()`, `Uid::from()`, or `Uid::as_str()`.

## License

MIT
