use std::fmt;

use crate::schema::edge::GraphEdge;
use crate::schema::node::GraphNode;
use crate::schema::EdgeType;
use crate::types::Uid;

/// Events emitted by graph mutations.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphEvent {
    NodeAdded(Box<GraphNode>),
    NodeUpdated { uid: Uid, version: i64 },
    NodeTombstoned(Uid),
    EdgeAdded(Box<GraphEdge>),
    EdgeTombstoned {
        uid: Uid,
        from_uid: Uid,
        to_uid: Uid,
        edge_type: EdgeType,
    },
}

impl fmt::Display for GraphEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphEvent::NodeAdded(node) => write!(f, "NodeAdded({}, {})", node.uid, node.label),
            GraphEvent::NodeUpdated { uid, version } => write!(f, "NodeUpdated({}, v{})", uid, version),
            GraphEvent::NodeTombstoned(uid) => write!(f, "NodeTombstoned({})", uid),
            GraphEvent::EdgeAdded(edge) => write!(f, "EdgeAdded({}, {})", edge.uid, edge.edge_type),
            GraphEvent::EdgeTombstoned { uid, from_uid, to_uid, edge_type } => {
                write!(f, "EdgeTombstoned({}, {} -> {}, {})", uid, from_uid, to_uid, edge_type)
            }
        }
    }
}

/// Unique identifier for a subscription.
pub type SubscriptionId = u64;
