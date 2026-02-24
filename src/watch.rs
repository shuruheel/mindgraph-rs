//! Async filtered event stream for graph changes. Requires the `async` feature.

use tokio::sync::broadcast;

use crate::events::{EventFilter, GraphEvent};

/// An async stream of filtered graph events.
///
/// Created via [`MindGraph::watch`](crate::MindGraph::watch) or
/// [`AsyncMindGraph::watch`](crate::AsyncMindGraph::watch).
pub struct WatchStream {
    rx: broadcast::Receiver<GraphEvent>,
    filter: EventFilter,
}

impl WatchStream {
    pub(crate) fn new(rx: broadcast::Receiver<GraphEvent>, filter: EventFilter) -> Self {
        Self { rx, filter }
    }

    /// Receive the next matching event. Returns `None` if the sender is dropped.
    pub async fn recv(&mut self) -> Option<GraphEvent> {
        loop {
            match self.rx.recv().await {
                Ok(event) => {
                    if self.filter.matches(&event) {
                        return Some(event);
                    }
                    // Doesn't match filter; keep looping
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Some events were dropped due to buffer overflow; continue
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return None;
                }
            }
        }
    }
}
