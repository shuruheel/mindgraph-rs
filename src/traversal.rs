//! Graph traversal types for multi-hop queries.

use serde::{Deserialize, Serialize};

use crate::schema::{EdgeType, NodeType};

/// Direction for graph traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Follow edges from source to target.
    Outgoing,
    /// Follow edges from target to source.
    Incoming,
    /// Follow edges in both directions.
    Both,
}

/// Options controlling a graph traversal.
#[derive(Debug, Clone)]
pub struct TraversalOptions {
    /// Direction to traverse.
    pub direction: Direction,
    /// If set, only follow edges of these types.
    pub edge_types: Option<Vec<EdgeType>>,
    /// Maximum traversal depth.
    pub max_depth: u32,
    /// If set, only traverse edges with weight >= this threshold.
    pub weight_threshold: Option<f64>,
}

impl Default for TraversalOptions {
    fn default() -> Self {
        TraversalOptions {
            direction: Direction::Outgoing,
            edge_types: None,
            max_depth: 10,
            weight_threshold: None,
        }
    }
}

/// A single step in a traversal result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathStep {
    /// UID of the reached node.
    pub node_uid: crate::types::Uid,
    /// Label of the reached node.
    pub label: String,
    /// Type of the reached node.
    pub node_type: NodeType,
    /// Type of the edge used to reach this node (None for the start node at depth 0).
    pub edge_type: Option<EdgeType>,
    /// Depth at which this node was reached.
    pub depth: u32,
    /// UID of the parent node in the traversal tree (None for the start node).
    pub parent_uid: Option<crate::types::Uid>,
}
