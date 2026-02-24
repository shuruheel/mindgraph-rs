use serde::{Deserialize, Serialize};

use crate::schema::node_props::NodeProps;
use crate::schema::{CustomNodeType, Layer, NodeType};
use crate::types::{Confidence, PrivacyLevel, Salience, Timestamp, Uid};

/// A node in the knowledge graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl GraphNode {
    /// Deserialize the custom props data back into a typed struct.
    /// Returns `None` if this node is not a `Custom` type or the type name doesn't match.
    pub fn custom_props<T: CustomNodeType>(&self) -> Option<T> {
        if let NodeProps::Custom { type_name, data, .. } = &self.props {
            if type_name == T::type_name() {
                return serde_json::from_value(data.clone()).ok();
            }
        }
        None
    }
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
    /// Optional pre-assigned UID. If `None`, a new UID is generated on insert.
    pub uid: Option<Uid>,
}

impl CreateNode {
    /// Create a new node builder with a label and typed props.
    /// The `node_type` and `layer` are inferred from the `NodeProps` variant.
    pub fn new(label: impl Into<String>, props: NodeProps) -> Self {
        CreateNode {
            label: label.into(),
            summary: String::new(),
            confidence: Confidence::default(),
            salience: Salience::default(),
            privacy_level: PrivacyLevel::default(),
            props,
            uid: None,
        }
    }

    /// Pre-assign a UID for this node. Useful for cross-referencing in [`MindGraph::validate_batch`](crate::MindGraph::validate_batch).
    pub fn with_uid(mut self, uid: Uid) -> Self {
        self.uid = Some(uid);
        self
    }

    /// Set the node summary text.
    pub fn summary(mut self, s: impl Into<String>) -> Self {
        self.summary = s.into();
        self
    }

    /// Set the epistemic confidence (0.0–1.0, default 1.0).
    pub fn confidence(mut self, c: Confidence) -> Self {
        self.confidence = c;
        self
    }

    /// Set the contextual salience (0.0–1.0, default 0.5).
    pub fn salience(mut self, s: Salience) -> Self {
        self.salience = s;
        self
    }

    /// Set the privacy level (default `Private`).
    pub fn privacy(mut self, p: PrivacyLevel) -> Self {
        self.privacy_level = p;
        self
    }
}
