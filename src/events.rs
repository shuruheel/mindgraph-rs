use std::fmt;

use crate::schema::edge::GraphEdge;
use crate::schema::node::GraphNode;
use crate::schema::{EdgeType, Layer, NodeType};
use crate::types::Uid;

/// The kind of a graph event, without associated data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    NodeAdded,
    NodeUpdated,
    NodeTombstoned,
    EdgeAdded,
    EdgeTombstoned,
}

/// Events emitted by graph mutations.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphEvent {
    NodeAdded {
        node: Box<GraphNode>,
        changed_by: String,
    },
    NodeUpdated {
        uid: Uid,
        version: i64,
        node_type: NodeType,
        layer: Layer,
        changed_by: String,
    },
    NodeTombstoned {
        uid: Uid,
        node_type: NodeType,
        layer: Layer,
        changed_by: String,
    },
    EdgeAdded {
        edge: Box<GraphEdge>,
        changed_by: String,
    },
    EdgeTombstoned {
        uid: Uid,
        from_uid: Uid,
        to_uid: Uid,
        edge_type: EdgeType,
        changed_by: String,
    },
}

impl GraphEvent {
    /// Returns the kind of this event.
    pub fn kind(&self) -> EventKind {
        match self {
            GraphEvent::NodeAdded { .. } => EventKind::NodeAdded,
            GraphEvent::NodeUpdated { .. } => EventKind::NodeUpdated,
            GraphEvent::NodeTombstoned { .. } => EventKind::NodeTombstoned,
            GraphEvent::EdgeAdded { .. } => EventKind::EdgeAdded,
            GraphEvent::EdgeTombstoned { .. } => EventKind::EdgeTombstoned,
        }
    }

    /// Returns the agent identity that triggered this event.
    pub fn changed_by(&self) -> &str {
        match self {
            GraphEvent::NodeAdded { changed_by, .. }
            | GraphEvent::NodeUpdated { changed_by, .. }
            | GraphEvent::NodeTombstoned { changed_by, .. }
            | GraphEvent::EdgeAdded { changed_by, .. }
            | GraphEvent::EdgeTombstoned { changed_by, .. } => changed_by,
        }
    }
}

impl fmt::Display for GraphEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphEvent::NodeAdded { node, .. } => {
                write!(f, "NodeAdded({}, {})", node.uid, node.label)
            }
            GraphEvent::NodeUpdated { uid, version, .. } => {
                write!(f, "NodeUpdated({}, v{})", uid, version)
            }
            GraphEvent::NodeTombstoned { uid, .. } => write!(f, "NodeTombstoned({})", uid),
            GraphEvent::EdgeAdded { edge, .. } => {
                write!(f, "EdgeAdded({}, {})", edge.uid, edge.edge_type)
            }
            GraphEvent::EdgeTombstoned {
                uid,
                from_uid,
                to_uid,
                edge_type,
                ..
            } => {
                write!(
                    f,
                    "EdgeTombstoned({}, {} -> {}, {})",
                    uid, from_uid, to_uid, edge_type
                )
            }
        }
    }
}

/// Unique identifier for a subscription.
pub type SubscriptionId = u64;

/// Filter for selectively subscribing to graph events.
///
/// All fields are optional. An unset field means "match any". Multiple set fields
/// are AND'd together.
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// Only match events involving these node types.
    pub node_types: Option<Vec<NodeType>>,
    /// Only match events involving these edge types.
    pub edge_types: Option<Vec<EdgeType>>,
    /// Only match events involving nodes/edges in these layers.
    pub layers: Option<Vec<Layer>>,
    /// Only match these event kinds.
    pub event_kinds: Option<Vec<EventKind>>,
    /// Only match events from this agent (matches `changed_by` / `tombstone_by`).
    pub agent_id: Option<String>,
}

impl EventFilter {
    /// Create an empty filter that matches all events.
    pub fn new() -> Self {
        Self::default()
    }

    /// Only match events involving these node types.
    pub fn node_types(mut self, types: Vec<NodeType>) -> Self {
        self.node_types = Some(types);
        self
    }

    /// Only match events involving these edge types.
    pub fn edge_types(mut self, types: Vec<EdgeType>) -> Self {
        self.edge_types = Some(types);
        self
    }

    /// Only match events in these layers.
    pub fn layers(mut self, layers: Vec<Layer>) -> Self {
        self.layers = Some(layers);
        self
    }

    /// Only match these event kinds.
    pub fn event_kinds(mut self, kinds: Vec<EventKind>) -> Self {
        self.event_kinds = Some(kinds);
        self
    }

    /// Only match events from this agent.
    pub fn agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Check whether a graph event matches this filter.
    pub fn matches(&self, event: &GraphEvent) -> bool {
        // Check event kind
        if let Some(ref kinds) = self.event_kinds {
            if !kinds.contains(&event.kind()) {
                return false;
            }
        }

        // Check agent filter
        if let Some(ref agent) = self.agent_id {
            if event.changed_by() != agent {
                return false;
            }
        }

        // Check node/edge type and layer
        match event {
            GraphEvent::NodeAdded { node, .. } => {
                if let Some(ref types) = self.node_types {
                    if !types.contains(&node.node_type) {
                        return false;
                    }
                }
                if let Some(ref layers) = self.layers {
                    if !layers.contains(&node.layer) {
                        return false;
                    }
                }
            }
            GraphEvent::NodeUpdated {
                node_type, layer, ..
            } => {
                if let Some(ref types) = self.node_types {
                    if !types.contains(node_type) {
                        return false;
                    }
                }
                if let Some(ref layers) = self.layers {
                    if !layers.contains(layer) {
                        return false;
                    }
                }
            }
            GraphEvent::NodeTombstoned {
                node_type, layer, ..
            } => {
                if let Some(ref types) = self.node_types {
                    if !types.contains(node_type) {
                        return false;
                    }
                }
                if let Some(ref layers) = self.layers {
                    if !layers.contains(layer) {
                        return false;
                    }
                }
            }
            GraphEvent::EdgeAdded { edge, .. } => {
                if let Some(ref types) = self.edge_types {
                    if !types.contains(&edge.edge_type) {
                        return false;
                    }
                }
                if let Some(ref layers) = self.layers {
                    if !layers.contains(&edge.layer) {
                        return false;
                    }
                }
            }
            GraphEvent::EdgeTombstoned { edge_type, .. } => {
                if let Some(ref types) = self.edge_types {
                    if !types.contains(edge_type) {
                        return false;
                    }
                }
            }
        }

        true
    }
}
