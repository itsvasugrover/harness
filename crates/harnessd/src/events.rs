//! Live event bus: broadcast hub plus a bounded replay buffer.
//! Phones and browsers hold `GET /api/v1/events?cursor=N`; the server
//! replays buffered events newer than N, then streams live ones with
//! keep-alives. A lagged receiver gets a resync nudge and refetches
//! the board — cursors resume, they never guarantee delivery.
use axum::{
    extract::{Query, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio_stream::StreamExt as _;

/// Buffered events per hub before eviction (the resume window).
pub const REPLAY_WINDOW: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusEvent {
    pub seq: u64,
    pub kind: String,
    pub summary: String,
    pub time: String,
}

#[derive(Debug)]
struct Inner {
    seq: AtomicU64,
    tx: tokio::sync::broadcast::Sender<BusEvent>,
    buffer: tokio::sync::RwLock<VecDeque<BusEvent>>,
}

#[derive(Debug, Clone)]
pub struct Hub {
    inner: Arc<Inner>,
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

impl Hub {
    /// Infallible constructor: channel plus an empty replay buffer.
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(256);
        Self {
            inner: Arc::new(Inner {
                seq: AtomicU64::new(0),
                tx,
                buffer: tokio::sync::RwLock::new(VecDeque::new()),
            }),
        }
    }

    /// Publish one event: sequence it, buffer it (evicting past the
    /// window), and broadcast to live receivers. Lagged receivers drop
    /// the message and resync through the board poll.
    pub async fn publish(&self, kind: &str, summary: &str) -> BusEvent {
        let event = BusEvent {
            seq: self.inner.seq.fetch_add(1, Ordering::SeqCst) + 1,
            kind: kind.into(),
            summary: summary.into(),
            time: chrono::Utc::now().to_rfc3339(),
        };
        {
            let mut buffer = self.inner.buffer.write().await;
            buffer.push_back(event.clone());
            while buffer.len() > REPLAY_WINDOW {
                buffer.pop_front();
            }
        }
        let _ = self.inner.tx.send(event.clone());
        event
    }

    /// Buffered events newer than `cursor`, oldest first.
    pub async fn replay_since(&self, cursor: u64) -> Vec<BusEvent> {
        self.inner
            .buffer
            .read()
            .await
            .iter()
            .filter(|e| e.seq > cursor)
            .cloned()
            .collect()
    }

    /// Live receiver from now on (combine with `replay_since`).
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<BusEvent> {
        self.inner.tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sequences_increase_and_replay_filters() {
        let hub = Hub::new();
        let first = hub.publish("board", "tick").await;
        let second = hub.publish("intent", "applied a1").await;
        assert!(second.seq > first.seq);
        assert_eq!(hub.replay_since(0).await.len(), 2);
        assert_eq!(hub.replay_since(first.seq).await.len(), 1);
        assert!(hub.replay_since(second.seq).await.is_empty());
    }

    #[tokio::test]
    async fn live_subscribers_see_publishes() {
        let hub = Hub::new();
        let mut rx = hub.subscribe();
        hub.publish("hello", "daemon online").await;
        let got = rx.recv().await.unwrap();
        assert_eq!(got.kind, "hello");
    }

    #[tokio::test]
    async fn buffer_evicts_past_the_window() {
        let hub = Hub::new();
        for i in 0..REPLAY_WINDOW + 10 {
            hub.publish("tick", &format!("{i}")).await;
        }
        let all = hub.replay_since(0).await;
        assert_eq!(all.len(), REPLAY_WINDOW);
        assert_eq!(all.first().unwrap().seq, 11);
    }
}

/// `GET /api/v1/events`: server-sent events with cursor replay.
/// `?cursor=N` replays buffered events newer than N, then streams live
/// ones with keep-alives. A lagged receiver gets a resync nudge and
/// refetches the board — cursors resume, never guarantee.
pub(crate) async fn route(
    State(state): State<super::api::AppState>,
    Query(q): Query<EventsQuery>,
) -> axum::response::Response {
    let cursor = q.cursor.unwrap_or(0);
    let buffered = state.hub.replay_since(cursor).await;
    let rx = state.hub.subscribe();
    let past = tokio_stream::iter(
        buffered
            .into_iter()
            .map(|e| Ok::<Event, Infallible>(to_sse(&e))),
    );
    let live = tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(e) => Some(Ok(to_sse(&e))),
        Err(_) => Some(Ok(Event::default()
            .event("resync")
            .data("lagged — refetch board"))),
    });
    Sse::new(past.chain(live))
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}

/// One bus event as one SSE frame (id = sequence for cursors).
fn to_sse(e: &crate::events::BusEvent) -> Event {
    Event::default()
        .event(e.kind.clone())
        .data(serde_json::to_string(e).unwrap_or_default())
        .id(e.seq.to_string())
}

/// `GET /api/v1/events` filters: cursor into the replay buffer.
#[derive(Debug, Deserialize)]
pub(crate) struct EventsQuery {
    cursor: Option<u64>,
}
