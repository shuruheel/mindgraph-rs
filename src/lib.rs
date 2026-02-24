//! # mindgraph
//!
//! A structured semantic memory graph for agentic systems.
//!
//! mindgraph provides a typed knowledge graph with 48 node types across 6 conceptual layers
//! (Reality, Epistemic, Intent, Action, Memory, Agent) and 70 edge types. It is backed by
//! CozoDB for storage and supports versioning, provenance tracking, and soft-deletion.
//!
//! ## Quick Start
//!
//! ```rust
//! use mindgraph::*;
//!
//! // Create an in-memory graph
//! let graph = MindGraph::open_in_memory().unwrap();
//!
//! // Add a claim node
//! let claim = graph.add_node(
//!     CreateNode::new("Rust is memory safe", NodeProps::Claim(ClaimProps {
//!         content: "Rust is memory safe".into(),
//!         claim_type: Some("factual".into()),
//!         ..Default::default()
//!     }))
//!     .confidence(Confidence::new(0.95).unwrap())
//! ).unwrap();
//!
//! // Add supporting evidence
//! let evidence = graph.add_node(
//!     CreateNode::new("Borrow checker", NodeProps::Evidence(EvidenceProps {
//!         description: "Borrow checker prevents dangling pointers".into(),
//!         ..Default::default()
//!     }))
//! ).unwrap();
//!
//! // Connect them with a SUPPORTS edge
//! graph.add_edge(CreateEdge::new(
//!     evidence.uid.clone(),
//!     claim.uid.clone(),
//!     EdgeProps::Supports { strength: Some(0.9), support_type: Some("empirical".into()) },
//! )).unwrap();
//!
//! // Update using the builder pattern
//! graph.update(&claim.uid)
//!     .confidence(Confidence::new(0.99).unwrap())
//!     .changed_by("agent-1")
//!     .reason("strong supporting evidence")
//!     .apply()
//!     .unwrap();
//!
//! // Query: how many claims exist?
//! assert_eq!(graph.count_nodes(NodeType::Claim).unwrap(), 1);
//!
//! // Traverse the reasoning chain (includes start node at depth 0)
//! let chain = graph.reasoning_chain(&claim.uid, 5).unwrap();
//! assert!(!chain.is_empty());
//! assert_eq!(chain[0].depth, 0);
//! ```

pub mod agent;
pub mod embeddings;
pub mod error;
pub mod events;
pub mod graph;
pub mod provenance;
pub mod query;
pub mod schema;
pub mod storage;
pub mod traversal;
pub mod types;

#[cfg(feature = "async")]
pub mod async_graph;
#[cfg(feature = "async")]
pub mod watch;
#[cfg(feature = "async")]
pub use async_graph::{AsyncAgentHandle, AsyncMindGraph};
#[cfg(feature = "async")]
pub use watch::WatchStream;

#[cfg(feature = "openai")]
pub mod openai;
#[cfg(feature = "openai")]
pub use openai::OpenAIEmbeddings;

// Re-export the most commonly used types
pub use agent::AgentHandle;
pub use embeddings::EmbeddingProvider;
#[cfg(feature = "async")]
pub use embeddings::{AsyncEmbeddingProvider, SyncProviderAdapter};
pub use error::{Error, Result};
pub use events::{EventFilter, EventKind, GraphEvent, SubscriptionId};
pub use graph::MindGraph;
pub use provenance::{ExtractionMethod, ProvenanceEntry, ProvenanceRecord};
pub use query::{
    BatchResult, Contradiction, DecayResult, GraphOp, GraphSnapshot, GraphStats,
    ImportResult, MergeResult, NodeFilter, Page, Pagination, PropCondition, PropOp,
    PurgeResult, SearchOptions, SearchResult, TombstoneResult, TypedImportResult,
    TypedSnapshot, ValidatedBatch, VersionRecord, WeakClaim,
};
pub use schema::edge::{CreateEdge, GraphEdge};
pub use schema::edge_props::EdgeProps;
pub use schema::node::{CreateNode, GraphNode};
pub use schema::node_props::NodeProps;
pub use schema::{CustomNodeType, EdgeType, Layer, NodeType};
pub use traversal::{Direction, PathStep, TraversalOptions};
pub use types::{now, Confidence, PrivacyLevel, Salience, Timestamp, Uid};

// Re-export all props types for convenience
pub use schema::props::action::*;
pub use schema::props::agent::*;
pub use schema::props::epistemic::*;
pub use schema::props::intent::*;
pub use schema::props::memory::*;
pub use schema::props::reality::*;
