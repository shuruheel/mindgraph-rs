use thiserror::Error;

/// All errors that can occur in mindgraph operations.
#[derive(Error, Debug)]
pub enum Error {
    /// CozoDB storage layer error.
    #[error("Storage error: {0}")]
    Storage(String),

    /// The requested node UID was not found.
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// The requested edge UID was not found.
    #[error("Edge not found: {0}")]
    EdgeNotFound(String),

    /// An unrecognized node type string was encountered.
    #[error("Invalid node type: {0}")]
    InvalidNodeType(String),

    /// An unrecognized edge type string was encountered.
    #[error("Invalid edge type: {0}")]
    InvalidEdgeType(String),

    /// JSON serialization/deserialization failed.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// A domain validation rule was violated.
    #[error("Validation error: {0}")]
    Validation(String),

    /// The node or edge has been soft-deleted.
    #[error("Node is tombstoned: {0}")]
    Tombstoned(String),

    /// Epistemic-layer nodes require provenance records.
    #[error("Provenance required for epistemic nodes")]
    ProvenanceRequired,

    /// Confidence value was outside the valid 0.0..=1.0 range.
    #[error("Confidence value must be between 0.0 and 1.0, got {0}")]
    InvalidConfidence(f64),

    /// Salience value was outside the valid 0.0..=1.0 range.
    #[error("Salience value must be between 0.0 and 1.0, got {0}")]
    InvalidSalience(f64),

    /// Props variant doesn't match the node/edge type.
    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },

    /// Embeddings not configured: call configure_embeddings(dimension) first.
    #[error("Embeddings not configured: call configure_embeddings(dimension) first")]
    EmbeddingNotConfigured,

    /// Embedding dimension mismatch.
    #[error("Embedding dimension mismatch: expected {expected}, got {got}")]
    EmbeddingDimensionMismatch { expected: usize, got: usize },

    /// HTTP error (for optional providers).
    #[cfg(feature = "openai")]
    #[error("HTTP error: {0}")]
    Http(String),
}

/// Convenience alias for `std::result::Result<T, mindgraph::Error>`.
pub type Result<T> = std::result::Result<T, Error>;
