use serde::{Deserialize, Serialize};

use crate::schema::edge_props::EdgeProps;
use crate::schema::{EdgeType, Layer};
use crate::types::{Confidence, Timestamp, Uid};

/// An edge in the knowledge graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub uid: Uid,
    pub from_uid: Uid,
    pub to_uid: Uid,
    pub edge_type: EdgeType,
    pub layer: Layer,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub version: i64,
    pub confidence: Confidence,
    pub weight: f64,
    pub tombstone_at: Option<Timestamp>,
    pub props: EdgeProps,
}

/// Builder for creating new edges.
#[derive(Debug, Clone)]
pub struct CreateEdge {
    pub from_uid: Uid,
    pub to_uid: Uid,
    pub confidence: Confidence,
    pub weight: f64,
    pub props: EdgeProps,
}

impl CreateEdge {
    /// Create a new edge builder between two nodes with typed props.
    /// The `edge_type` is inferred from the `EdgeProps` variant.
    pub fn new(from: Uid, to: Uid, props: EdgeProps) -> Self {
        CreateEdge {
            from_uid: from,
            to_uid: to,
            confidence: Confidence::default(),
            weight: 0.5,
            props,
        }
    }

    /// Set the epistemic confidence (0.0–1.0, default 1.0).
    pub fn confidence(mut self, c: Confidence) -> Self {
        self.confidence = c;
        self
    }

    /// Set the edge weight (default 0.5).
    pub fn weight(mut self, w: f64) -> Self {
        self.weight = w;
        self
    }
}
