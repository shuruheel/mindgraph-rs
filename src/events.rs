use crate::schema::edge::GraphEdge;
use crate::schema::node::GraphNode;
use crate::types::Uid;

/// Events emitted by graph mutations.
#[derive(Debug, Clone)]
pub enum GraphEvent {
    NodeAdded(Box<GraphNode>),
    NodeUpdated { uid: Uid, version: i64 },
    NodeTombstoned(Uid),
    EdgeAdded(Box<GraphEdge>),
    EdgeTombstoned(Uid),
}

/// Unique identifier for a subscription.
pub type SubscriptionId = u64;
