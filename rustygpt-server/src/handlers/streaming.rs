use axum::{
    extract::Extension,
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::Stream;
use serde_json::{Value, json};
use shared::{config::server::Config, models::MessageChunk};
use std::{
    collections::{HashMap, VecDeque},
    convert::Infallible,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Mutex, mpsc};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use tracing::info;
use uuid::Uuid;

use crate::middleware::request_context::RequestContext;

pub type SharedState = Arc<SseCoordinator>;

#[derive(Clone, Debug)]
struct SseEvent {
    id: Option<String>,
    event: String,
    data: String,
}

struct UserStreamState {
    history: VecDeque<SseEvent>,
    next_sequence: u64,
    sender: Option<mpsc::Sender<SseEvent>>,
}

impl UserStreamState {
    fn new() -> Self {
        Self {
            history: VecDeque::new(),
            next_sequence: 0,
            sender: None,
        }
    }

    fn backlog(&self) -> Vec<SseEvent> {
        self.history.iter().cloned().collect()
    }

    fn backlog_after(&self, last_id: &str) -> Option<Vec<SseEvent>> {
        if last_id.is_empty() {
            return Some(self.backlog());
        }

        self.history
            .iter()
            .position(|event| event.id.as_deref() == Some(last_id))
            .map(|idx| self.history.iter().skip(idx + 1).cloned().collect())
    }

    fn next_persistent_event(
        &mut self,
        id_prefix: &str,
        event: &str,
        payload: Value,
        history_limit: usize,
    ) -> SseEvent {
        let id = format!("{}{}", id_prefix, self.next_sequence);
        self.next_sequence += 1;

        let event = SseEvent {
            id: Some(id),
            event: event.to_string(),
            data: payload.to_string(),
        };

        self.history.push_back(event.clone());
        if self.history.len() > history_limit {
            self.history.pop_front();
        }

        event
    }

    fn next_ephemeral_event(&mut self, event: &str, payload: Value) -> SseEvent {
        SseEvent {
            id: None,
            event: event.to_string(),
            data: payload.to_string(),
        }
    }
}

pub struct SseCoordinator {
    capacity: usize,
    history_limit: usize,
    id_prefix: String,
    inner: Mutex<HashMap<Uuid, UserStreamState>>,
}

impl SseCoordinator {
    pub fn new(capacity: usize, id_prefix: String) -> Self {
        let capacity = capacity.max(1);
        let history_limit = capacity.max(32);
        Self {
            capacity,
            history_limit,
            id_prefix,
            inner: Mutex::new(HashMap::new()),
        }
    }

    async fn subscribe(
        &self,
        user_id: Uuid,
        last_event_id: Option<&str>,
    ) -> Result<mpsc::Receiver<SseEvent>, SubscriptionError> {
        let (sender, receiver, stale, backlog) = {
            let mut guard = self.inner.lock().await;
            let entry = guard.entry(user_id).or_insert_with(UserStreamState::new);

            if let Some(existing) = entry.sender.as_ref() {
                if !existing.is_closed() {
                    return Err(SubscriptionError::AlreadyConnected);
                }
            }

            let (tx, rx) = mpsc::channel(self.capacity);
            entry.sender = Some(tx.clone());

            let mut stale = false;
            let backlog = if let Some(last_id) = last_event_id {
                match entry.backlog_after(last_id) {
                    Some(events) => events,
                    None => {
                        stale = true;
                        Vec::new()
                    }
                }
            } else {
                entry.backlog()
            };

            (tx, rx, stale, backlog)
        };

        // Notify client if their cursor is stale.
        if stale {
            let stale_event = {
                let mut guard = self.inner.lock().await;
                let entry = guard.get_mut(&user_id).unwrap();
                entry.next_persistent_event(
                    &self.id_prefix,
                    "error",
                    json!({
                        "message": "Event history no longer available; please reload.",
                        "reason": "stale_cursor"
                    }),
                    self.history_limit,
                )
            };

            let _ = sender.send(stale_event).await;
        }

        // Replay backlog to the client.
        for event in backlog {
            if sender.send(event).await.is_err() {
                break;
            }
        }

        // Emit connection acknowledgement (ephemeral).
        let connection_event = {
            let mut guard = self.inner.lock().await;
            let entry = guard.get_mut(&user_id).unwrap();
            entry.next_ephemeral_event(
                "message",
                json!({
                    "type": "connection",
                    "message": "Connected to SSE stream"
                }),
            )
        };
        let _ = sender.send(connection_event).await;

        Ok(receiver)
    }

    pub async fn publish(&self, user_id: Uuid, event: &str, payload: Value, persist: bool) {
        let (sender, message) = {
            let mut guard = self.inner.lock().await;
            let entry = guard.entry(user_id).or_insert_with(UserStreamState::new);

            let message = if persist {
                entry.next_persistent_event(&self.id_prefix, event, payload, self.history_limit)
            } else {
                entry.next_ephemeral_event(event, payload)
            };

            (entry.sender.clone(), message)
        };

        if let Some(sender) = sender {
            if let Err(err) = sender.try_send(message.clone()) {
                match err {
                    mpsc::error::TrySendError::Full(_msg) => {
                        // Drop low-priority events (tokens); ensure important events get delivered.
                        if message.event != "token" {
                            let _ = sender.send(message).await;
                        }
                    }
                    mpsc::error::TrySendError::Closed(_msg) => {
                        let mut guard = self.inner.lock().await;
                        if let Some(entry) = guard.get_mut(&user_id) {
                            entry.sender = None;
                        }
                    }
                }
            }
        }
    }

    pub fn spawn_heartbeat(self: &Arc<Self>, user_id: Uuid, heartbeat_seconds: u64) {
        let cadence = heartbeat_seconds.max(5);
        let coordinator = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(cadence));
            loop {
                interval.tick().await;
                if !coordinator.send_ping(user_id).await {
                    break;
                }
            }
        });
    }

    async fn send_ping(&self, user_id: Uuid) -> bool {
        let (sender, event) = {
            let mut guard = self.inner.lock().await;
            let entry = match guard.get_mut(&user_id) {
                Some(entry) => entry,
                None => return false,
            };

            match entry.sender.clone() {
                Some(sender) if !sender.is_closed() => {
                    let event = entry.next_ephemeral_event("ping", json!({}));
                    (sender, event)
                }
                _ => return false,
            }
        };

        match sender.try_send(event.clone()) {
            Ok(_) => true,
            Err(mpsc::error::TrySendError::Full(_)) => true, // drop ping silently
            Err(mpsc::error::TrySendError::Closed(_)) => {
                let mut guard = self.inner.lock().await;
                if let Some(entry) = guard.get_mut(&user_id) {
                    entry.sender = None;
                }
                false
            }
        }
    }
}

#[derive(Debug)]
enum SubscriptionError {
    AlreadyConnected,
}

/// Server-sent events endpoint with resumable support and backpressure control.
pub async fn sse_handler(
    Extension(config): Extension<Arc<Config>>,
    Extension(state): Extension<SharedState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, axum::http::StatusCode> {
    let user_id = context
        .user_id
        .ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

    info!("Establishing SSE stream for user {}", user_id);

    let last_event_id = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string());

    let receiver = state
        .subscribe(user_id, last_event_id.as_deref())
        .await
        .map_err(|err| match err {
            SubscriptionError::AlreadyConnected => axum::http::StatusCode::CONFLICT,
        })?;

    state.spawn_heartbeat(user_id, config.sse.heartbeat_seconds);

    let stream = ReceiverStream::new(receiver).map(|event| {
        let mut builder = Event::default().event(event.event);
        if let Some(id) = event.id {
            builder = builder.id(id);
        }
        Ok::<_, Infallible>(builder.data(event.data))
    });

    let keepalive = KeepAlive::new()
        .interval(Duration::from_secs(config.sse.heartbeat_seconds.max(5)))
        .text("keep-alive");

    Ok(Sse::new(stream).keep_alive(keepalive))
}

/// Stream partial responses to a connected user, respecting backpressure policy.
pub async fn stream_partial_response(
    state: SharedState,
    user_id: Uuid,
    conversation_id: Uuid,
    message_id: Uuid,
    chunks: Vec<String>,
) {
    if chunks.is_empty() {
        return;
    }

    let total = chunks.len();
    for (index, content) in chunks.into_iter().enumerate() {
        let is_final = index == total - 1;
        let chunk = MessageChunk {
            conversation_id,
            message_id,
            content_type: "text".to_string(),
            content: content.clone(),
            is_final,
        };

        let payload = serde_json::to_value(&chunk).unwrap_or_else(|_| {
            json!({
                "error": "serialization_failed",
                "conversation_id": conversation_id,
                "message_id": message_id
            })
        });

        let event_type = if is_final { "message" } else { "token" };
        state.publish(user_id, event_type, payload, true).await;

        if is_final {
            state
                .publish(
                    user_id,
                    "complete",
                    json!({
                        "conversation_id": conversation_id,
                        "message_id": message_id
                    }),
                    true,
                )
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::config::server::{Config, Profile};
    use tokio::time::timeout;

    fn test_config() -> Arc<Config> {
        let mut config = Config::default_for_profile(Profile::Dev);
        config.features.sse_v1 = true;
        config.features.auth_v1 = true;
        Arc::new(config)
    }

    #[tokio::test]
    async fn test_sse_handler_requires_user() {
        let config = test_config();
        let state = Arc::new(SseCoordinator::new(
            config.sse.channel_capacity,
            config.sse.id_prefix.clone(),
        ));

        let context = RequestContext {
            request_id: "req-1".into(),
            user_id: None,
        };

        let result = sse_handler(
            Extension(config),
            Extension(state),
            Extension(context),
            HeaderMap::new(),
        )
        .await;

        assert_eq!(result.unwrap_err(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_stream_partial_response_emits_events() {
        let config = test_config();
        let state = Arc::new(SseCoordinator::new(
            config.sse.channel_capacity,
            config.sse.id_prefix.clone(),
        ));

        let user_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        // Subscribe user to obtain receiver.
        let mut receiver = state
            .subscribe(user_id, None)
            .await
            .expect("subscription should succeed");

        // Drain connection acknowledgement.
        let _ = receiver.recv().await;

        stream_partial_response(
            state.clone(),
            user_id,
            conversation_id,
            message_id,
            vec!["Hello".into(), " world".into()],
        )
        .await;

        // Collect token and final message events.
        let token_event = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("token event")
            .expect("token payload");
        assert_eq!(token_event.event, "token");

        let message_event = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("message event")
            .expect("message payload");
        assert_eq!(message_event.event, "message");

        let complete_event = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("complete event")
            .expect("complete payload");
        assert_eq!(complete_event.event, "complete");
    }
}
