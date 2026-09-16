//! SSE client wrapper over `GET /api/v1/stream`, mapping server events to app
//! events (#27/#51/#57). See docs/api-contract.md for the wire format.

use futures_util::StreamExt;
use reqwest_eventsource::{Event as SseEvent, EventSource};

use super::client::ApiClient;
use super::types::RealtimeEvent;

pub struct RealtimeStream {
    source: EventSource,
}

impl RealtimeStream {
    pub fn connect(
        client: &ApiClient,
    ) -> Result<Self, reqwest_eventsource::CannotCloneRequestError> {
        Ok(Self {
            source: EventSource::new(client.stream_request())?,
        })
    }

    /// The next application-relevant event. Transparently skips the
    /// connection-open notice, unparseable frames, and transient transport
    /// errors — `EventSource` retries those on its own with exponential
    /// backoff (see docs/architecture.md), so this just logs and loops again.
    ///
    /// Returns `None` only when the stream is permanently done. That happens
    /// for exactly one reason: `EventSource` doesn't retry a bad *initial*
    /// response at all (e.g. a 401 because the bearer token died, or a 429
    /// from the server's connection cap) — it yields that one error, which
    /// this logs, and then closes; the *next* call here sees the closed
    /// state and returns `None` immediately. Callers shouldn't try to
    /// reconnect themselves after that — a dead token needs a fresh login,
    /// not a retry.
    pub async fn next(&mut self) -> Option<RealtimeEvent> {
        loop {
            match self.source.next().await? {
                Ok(SseEvent::Open) => {
                    tracing::info!("SSE stream connected");
                    continue;
                }
                Ok(SseEvent::Message(message)) => {
                    match serde_json::from_str::<RealtimeEvent>(&message.data) {
                        Ok(event) => return Some(event),
                        Err(error) => {
                            tracing::warn!(%error, data = %message.data, "unparseable SSE event");
                            continue;
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "SSE connection error");
                    continue;
                }
            }
        }
    }
}
