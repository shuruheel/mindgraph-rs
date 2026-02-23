use serde::{Deserialize, Serialize};

use crate::schema::node_props::NodeProps;
use crate::schema::{Layer, NodeType};
use crate::types::{Confidence, PrivacyLevel, Salience, Timestamp, Uid};

/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub uid: Uid,
    pub node_type: NodeType,
    pub layer: Layer,
    pub label: String,
    pub summary: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub version: i64,
    pub confidence: Confidence,
    pub salience: Salience,
    pub privacy_level: PrivacyLevel,
    pub embedding_ref: Option<String>,
    pub tombstone_at: Option<Timestamp>,
    pub tombstone_reason: Option<String>,
    pub tombstone_by: Option<String>,
    pub props: NodeProps,
}

/// Builder for creating new nodes. `node_type` and `layer` are inferred from `props`.
#[derive(Debug, Clone)]
pub struct CreateNode {
    pub label: String,
    pub summary: String,
    pub confidence: Confidence,
    pub salience: Salience,
    pub privacy_level: PrivacyLevel,
    pub props: NodeProps,
}

impl CreateNode {
    pub fn new(label: impl Into<String>, props: NodeProps) -> Self {
        CreateNode {
            label: label.into(),
            summary: String::new(),
            confidence: Confidence::default(),
            salience: Salience::default(),
            privacy_level: PrivacyLevel::default(),
            props,
        }
    }

    pub fn summary(mut self, s: impl Into<String>) -> Self {
        self.summary = s.into();
        self
    }

    pub fn confidence(mut self, c: Confidence) -> Self {
        self.confidence = c;
        self
    }

    pub fn salience(mut self, s: Salience) -> Self {
        self.salience = s;
        self
    }

    pub fn privacy(mut self, p: PrivacyLevel) -> Self {
        self.privacy_level = p;
        self
    }
}
