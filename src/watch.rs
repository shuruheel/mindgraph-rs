//! Async filtered event stream for graph changes. Requires the `async` feature.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use tokio::sync::broadcast;

use crate::events::{EventFilter, GraphEvent};

/// An async stream of filtered graph events.
///
/// Created via [`MindGraph::watch`](crate::MindGraph::watch) or
/// [`AsyncMindGraph::watch`](crate::AsyncMindGraph::watch).
///
/// Implements [`futures_core::Stream`] for use with `StreamExt`, `select!`, etc.
pub struct WatchStream {
    rx: broadcast::Receiver<GraphEvent>,
    filter: EventFilter,
    lagged: AtomicU64,
}

impl WatchStream {
    pub(crate) fn new(rx: broadcast::Receiver<GraphEvent>, filter: EventFilter) -> Self {
        Self {
            rx,
            filter,
            lagged: AtomicU64::new(0),
        }
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
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    self.lagged.fetch_add(n, Ordering::Relaxed);
                    #[cfg(feature = "tracing")]
                    tracing::warn!(missed = n, "WatchStream lagged, {} events dropped", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return None;
                }
            }
        }
    }

    /// Returns the total number of events dropped due to broadcast lag.
    pub fn lagged_count(&self) -> u64 {
        self.lagged.load(Ordering::Relaxed)
    }
}

impl futures_core::Stream for WatchStream {
    type Item = GraphEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match this.rx.try_recv() {
                Ok(event) => {
                    if this.filter.matches(&event) {
                        return Poll::Ready(Some(event));
                    }
                    continue;
                }
                Err(broadcast::error::TryRecvError::Empty) => {
                    // No events buffered. Poll the recv future once to register waker.
                    let poll_result = {
                        let mut recv_fut = Box::pin(this.rx.recv());
                        recv_fut.as_mut().poll(cx)
                    };
                    // recv_fut is dropped here, releasing the mutable borrow on this.rx
                    match poll_result {
                        Poll::Ready(Ok(event)) => {
                            if this.filter.matches(&event) {
                                return Poll::Ready(Some(event));
                            }
                            continue;
                        }
                        Poll::Ready(Err(broadcast::error::RecvError::Lagged(n))) => {
                            this.lagged.fetch_add(n, Ordering::Relaxed);
                            #[cfg(feature = "tracing")]
                            tracing::warn!(missed = n, "WatchStream lagged, {} events dropped", n);
                            continue;
                        }
                        Poll::Ready(Err(broadcast::error::RecvError::Closed)) => {
                            return Poll::Ready(None);
                        }
                        Poll::Pending => {
                            return Poll::Pending;
                        }
                    }
                }
                Err(broadcast::error::TryRecvError::Closed) => {
                    return Poll::Ready(None);
                }
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    this.lagged.fetch_add(n, Ordering::Relaxed);
                    #[cfg(feature = "tracing")]
                    tracing::warn!(missed = n, "WatchStream lagged, {} events dropped", n);
                    continue;
                }
            }
        }
    }
}
