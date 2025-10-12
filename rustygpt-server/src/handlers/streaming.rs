use axum::{
    extract::Extension,
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::Stream;
use serde_json::{Value, json};
use shared::{
    config::server::{Config, SseBackpressureConfig, SseDropStrategy, SsePersistenceConfig},
    models::MessageChunk,
};
use std::{
    collections::{HashMap, VecDeque},
    convert::Infallible,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Mutex, mpsc};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use tracing::{info, warn};
use uuid::Uuid;

use crate::middleware::request_context::RequestContext;
use crate::services::sse_persistence::{SsePersistence, StreamEventRecord};

pub type SharedState = Arc<SseCoordinator>;

#[derive(Clone, Debug)]
struct SseEvent {
    id: Option<String>,
    event: String,
    data: String,
    sequence: Option<u64>,
}

struct UserStreamState {
    history: VecDeque<SseEvent>,
    next_sequence: u64,
    sender: Option<mpsc::Sender<SseEvent>>,
    hydrated: bool,
}

impl UserStreamState {
    fn new() -> Self {
        Self {
            history: VecDeque::new(),
            next_sequence: 0,
            sender: None,
            hydrated: false,
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
        let sequence = self.next_sequence;
        let id = format!("{}{}", id_prefix, sequence);
        self.next_sequence = self.next_sequence.saturating_add(1);

        let event = SseEvent {
            id: Some(id),
            event: event.to_string(),
            data: payload.to_string(),
            sequence: Some(sequence),
        };

        self.history.push_back(event.clone());
        if self.history.len() > history_limit {
            self.history.pop_front();
            metrics::counter!("sse_history_trimmed_total").increment(1);
        }
        metrics::gauge!("sse_history_size").set(self.history.len() as f64);

        event
    }

    fn next_ephemeral_event(&mut self, event: &str, payload: Value) -> SseEvent {
        SseEvent {
            id: None,
            event: event.to_string(),
            data: payload.to_string(),
            sequence: None,
        }
    }

    fn append_persisted_events(&mut self, events: Vec<SseEvent>, history_limit: usize) {
        for event in events {
            if let Some(sequence) = event.sequence {
                if self
                    .history
                    .iter()
                    .any(|existing| existing.sequence == Some(sequence))
                {
                    continue;
                }
                self.history.push_back(event.clone());
                self.next_sequence = self.next_sequence.max(sequence.saturating_add(1));
            } else {
                self.history.push_back(event.clone());
            }

            if self.history.len() > history_limit {
                self.history.pop_front();
                metrics::counter!("sse_history_trimmed_total").increment(1);
            }
        }
        metrics::gauge!("sse_history_size").set(self.history.len() as f64);
    }
}

pub struct SseCoordinator {
    capacity: usize,
    history_limit: usize,
    id_prefix: String,
    backpressure: SseBackpressureConfig,
    persistence: Option<Arc<dyn SsePersistence>>,
    persistence_config: Option<SsePersistenceConfig>,
    inner: Mutex<HashMap<Uuid, UserStreamState>>,
}

impl SseCoordinator {
    pub fn new(
        capacity: usize,
        id_prefix: String,
        persistence: Option<Arc<dyn SsePersistence>>,
        persistence_config: Option<SsePersistenceConfig>,
        backpressure: SseBackpressureConfig,
    ) -> Self {
        let capacity = capacity.max(1);
        let history_limit = capacity.max(32);
        Self {
            capacity,
            history_limit,
            id_prefix,
            backpressure,
            persistence,
            persistence_config,
            inner: Mutex::new(HashMap::new()),
        }
    }

    fn update_active_gauge(&self, map: &HashMap<Uuid, UserStreamState>) {
        let active = map.values().filter(|state| state.sender.is_some()).count() as f64;
        metrics::gauge!("sse_active_connections").set(active);
    }

    fn record_queue_depth(&self, sender: &mpsc::Sender<SseEvent>, category: &'static str) {
        let used = self.capacity.saturating_sub(sender.capacity());
        metrics::gauge!("sse_queue_depth", "category" => category).set(used as f64);
        if self.capacity > 0 {
            let ratio = used as f64 / self.capacity as f64;
            metrics::histogram!(
                "sse_queue_occupancy_ratio",
                "category" => category
            )
            .record(ratio);
            if self.backpressure.warn_queue_ratio > 0.0
                && ratio >= self.backpressure.warn_queue_ratio
            {
                warn!(
                    queue_ratio = ratio,
                    category, "sse queue nearing configured capacity"
                );
            }
        }
    }

    fn mark_disconnected(
        &self,
        map: &mut HashMap<Uuid, UserStreamState>,
        user_id: &Uuid,
        status: &'static str,
    ) {
        if let Some(entry) = map.get_mut(user_id) {
            entry.sender = None;
        }
        self.update_active_gauge(map);
        metrics::counter!("sse_connections_total", "status" => status).increment(1);
    }

    fn should_drop_on_backpressure(&self, event_name: &str) -> bool {
        match self.backpressure.drop_strategy {
            SseDropStrategy::DropTokens => event_name == "token",
            SseDropStrategy::DropTokensAndSystem => matches!(event_name, "token" | "ping"),
        }
    }

    fn extract_sequence(&self, event_id: &str) -> Option<u64> {
        event_id
            .strip_prefix(&self.id_prefix)
            .and_then(|suffix| suffix.parse::<u64>().ok())
    }

    fn convert_records(&self, records: Vec<StreamEventRecord>) -> Vec<SseEvent> {
        let mut events = records
            .into_iter()
            .map(|record| SseEvent {
                id: Some(record.event_id),
                event: record.event_name,
                data: record.payload,
                sequence: Some(record.sequence as u64),
            })
            .collect::<Vec<_>>();
        events.sort_by_key(|event| event.sequence.unwrap_or(0));
        events
    }

    async fn hydrate_from_persistence(&self, user_id: Uuid) {
        let store = match &self.persistence {
            Some(store) => Arc::clone(store),
            None => return,
        };

        let needs_hydration = {
            let mut guard = self.inner.lock().await;
            let entry = guard.entry(user_id).or_insert_with(UserStreamState::new);
            if entry.hydrated {
                return;
            }
            entry.hydrated = true;
            entry.history.is_empty()
        };

        if !needs_hydration {
            return;
        }

        match store.load_recent_events(user_id, self.history_limit).await {
            Ok(records) if !records.is_empty() => {
                let events = self.convert_records(records);
                let mut guard = self.inner.lock().await;
                if let Some(entry) = guard.get_mut(&user_id) {
                    entry.append_persisted_events(events, self.history_limit);
                }
            }
            Ok(_) => {}
            Err(err) => {
                warn!(?err, "failed to hydrate SSE history from persistence");
                let mut guard = self.inner.lock().await;
                if let Some(entry) = guard.get_mut(&user_id) {
                    entry.hydrated = false;
                }
            }
        }
    }

    async fn backfill_from_persistence(
        &self,
        user_id: Uuid,
        last_event_id: &str,
    ) -> Option<Vec<SseEvent>> {
        let store = self.persistence.as_ref()?;
        let last_sequence = self.extract_sequence(last_event_id)? as i64;

        let records = store
            .load_events_after(user_id, last_sequence, self.history_limit)
            .await
            .map_err(|err| {
                warn!(?err, "failed to load persisted SSE backlog");
                err
            })
            .ok()?;

        if records.is_empty() {
            return None;
        }

        let events = self.convert_records(records);
        let mut guard = self.inner.lock().await;
        if let Some(entry) = guard.get_mut(&user_id) {
            entry.append_persisted_events(events.clone(), self.history_limit);
            return entry
                .backlog_after(last_event_id)
                .filter(|backlog| !backlog.is_empty());
        }

        Some(events)
    }

    async fn persist_event(&self, user_id: Uuid, event: &SseEvent) {
        let store = match &self.persistence {
            Some(store) => Arc::clone(store),
            None => return,
        };

        let sequence = match (event.sequence, &event.id) {
            (Some(seq), Some(_)) => seq as i64,
            _ => return,
        };

        let record = StreamEventRecord {
            sequence,
            event_id: event.id.clone().unwrap_or_default(),
            event_name: event.event.clone(),
            payload: event.data.clone(),
        };

        if let Err(err) = store.record_event(user_id, &record).await {
            warn!(?err, "failed to persist SSE event");
            return;
        }

        if let Some(cfg) = &self.persistence_config {
            if let Err(err) = store
                .prune_events(user_id, cfg.max_events_per_user, cfg.prune_batch_size)
                .await
            {
                warn!(?err, "failed to prune SSE event history");
            }
        }
    }

    async fn subscribe(
        &self,
        user_id: Uuid,
        last_event_id: Option<String>,
    ) -> Result<mpsc::Receiver<SseEvent>, SubscriptionError> {
        self.hydrate_from_persistence(user_id).await;

        let mut stale;
        let mut backlog;
        let last_event_id_clone = last_event_id.clone();

        let (sender, receiver) = {
            let mut guard = self.inner.lock().await;
            let entry = guard.entry(user_id).or_insert_with(UserStreamState::new);

            if let Some(existing) = entry.sender.as_ref() {
                if !existing.is_closed() {
                    metrics::counter!(
                        "sse_connections_total",
                        "status" => "duplicate"
                    )
                    .increment(1);
                    return Err(SubscriptionError::AlreadyConnected);
                }
            }

            let (tx, rx) = mpsc::channel(self.capacity);
            entry.sender = Some(tx.clone());

            let mut stale_local = false;
            let backlog_local = if let Some(ref last_id) = last_event_id_clone {
                match entry.backlog_after(last_id) {
                    Some(events) => events,
                    None => {
                        stale_local = true;
                        Vec::new()
                    }
                }
            } else {
                entry.backlog()
            };

            self.update_active_gauge(&guard);
            metrics::counter!("sse_connections_total", "status" => "accepted").increment(1);

            stale = stale_local;
            backlog = backlog_local;
            (tx, rx)
        };

        if stale {
            if let Some(ref last_id) = last_event_id_clone {
                if let Some(events) = self.backfill_from_persistence(user_id, last_id).await {
                    backlog = events;
                    stale = backlog.is_empty();
                }
            }
        }

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

            match sender.send(stale_event.clone()).await {
                Ok(_) => {
                    metrics::counter!(
                        "sse_events_sent_total",
                        "event" => "error",
                        "mode" => "system"
                    )
                    .increment(1);
                    self.record_queue_depth(&sender, "system");
                    self.persist_event(user_id, &stale_event).await;
                }
                Err(_) => {
                    metrics::counter!(
                        "sse_events_dropped_total",
                        "reason" => "stale_cursor",
                        "event" => "error"
                    )
                    .increment(1);
                    let mut guard = self.inner.lock().await;
                    self.mark_disconnected(&mut guard, &user_id, "dropped");
                }
            }

            return Ok(receiver);
        }

        for event in backlog {
            let replay_event = event.clone();
            let event_name = replay_event.event.clone();
            if sender.send(replay_event).await.is_err() {
                let mut guard = self.inner.lock().await;
                self.mark_disconnected(&mut guard, &user_id, "closed");
                break;
            }
            self.record_queue_depth(&sender, "replay");
            metrics::counter!(
                "sse_events_sent_total",
                "event" => event_name.clone(),
                "mode" => "replay"
            )
            .increment(1);
        }

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
        match sender.send(connection_event).await {
            Ok(_) => {
                metrics::counter!(
                    "sse_events_sent_total",
                    "event" => "connection",
                    "mode" => "system"
                )
                .increment(1);
                self.record_queue_depth(&sender, "system");
            }
            Err(_) => {
                metrics::counter!(
                    "sse_events_dropped_total",
                    "reason" => "connection_ack",
                    "event" => "connection"
                )
                .increment(1);
                let mut guard = self.inner.lock().await;
                self.mark_disconnected(&mut guard, &user_id, "dropped");
            }
        }

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

        if persist {
            self.persist_event(user_id, &message).await;
        }

        if let Some(sender) = sender {
            let event_name = message.event.clone();
            let queue_category = if persist { "persistent" } else { "ephemeral" };
            if let Err(err) = sender.try_send(message.clone()) {
                match err {
                    mpsc::error::TrySendError::Full(_msg) => {
                        if self.should_drop_on_backpressure(&event_name) {
                            metrics::counter!(
                                "sse_events_dropped_total",
                                "reason" => "full",
                                "event" => event_name.clone()
                            )
                            .increment(1);
                            self.record_queue_depth(&sender, queue_category);
                        } else {
                            if sender.send(message.clone()).await.is_err() {
                                let mut guard = self.inner.lock().await;
                                self.mark_disconnected(&mut guard, &user_id, "dropped");
                                metrics::counter!(
                                    "sse_events_dropped_total",
                                    "reason" => "closed",
                                    "event" => event_name.clone()
                                )
                                .increment(1);
                            } else {
                                metrics::counter!(
                                    "sse_events_sent_total",
                                    "event" => event_name.clone(),
                                    "mode" => "live"
                                )
                                .increment(1);
                                self.record_queue_depth(&sender, queue_category);
                            }
                        }
                    }
                    mpsc::error::TrySendError::Closed(_msg) => {
                        let mut guard = self.inner.lock().await;
                        self.mark_disconnected(&mut guard, &user_id, "closed");
                    }
                }
            } else {
                metrics::counter!(
                    "sse_events_sent_total",
                    "event" => event_name,
                    "mode" => if persist { "live" } else { "system" }
                )
                .increment(1);
                self.record_queue_depth(&sender, queue_category);
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
            Ok(_) => {
                metrics::counter!(
                    "sse_events_sent_total",
                    "event" => "ping",
                    "mode" => "system"
                )
                .increment(1);
                self.record_queue_depth(&sender, "heartbeat");
                true
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                metrics::counter!(
                    "sse_events_dropped_total",
                    "reason" => "full",
                    "event" => "ping"
                )
                .increment(1);
                self.record_queue_depth(&sender, "heartbeat");
                true
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                let mut guard = self.inner.lock().await;
                self.mark_disconnected(&mut guard, &user_id, "closed");
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
        .subscribe(user_id, last_event_id.clone())
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
    use crate::services::sse_persistence::{SsePersistence, StreamEventRecord};
    use async_trait::async_trait;
    use shared::config::server::{Config, Profile, SsePersistenceConfig};
    use tokio::{sync::Mutex, time::timeout};

    fn test_config() -> Arc<Config> {
        let _ = crate::server::metrics_handle();
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
            None,
            None,
            config.sse.backpressure.clone(),
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
            None,
            None,
            config.sse.backpressure.clone(),
        ));

        let user_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        // Subscribe user to obtain receiver.
        let mut receiver = state
            .subscribe(user_id, None::<String>)
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

    struct MockPersistence {
        events: Mutex<std::collections::HashMap<Uuid, Vec<StreamEventRecord>>>,
    }

    impl MockPersistence {
        fn new() -> Self {
            Self {
                events: Mutex::new(std::collections::HashMap::new()),
            }
        }

        fn with_events(user_id: Uuid, records: Vec<StreamEventRecord>) -> Self {
            let mut map = std::collections::HashMap::new();
            map.insert(user_id, records);
            Self {
                events: Mutex::new(map),
            }
        }

        async fn events_for(&self, user_id: Uuid) -> Vec<StreamEventRecord> {
            let mut events = self
                .events
                .lock()
                .await
                .get(&user_id)
                .cloned()
                .unwrap_or_default();
            events.sort_by_key(|record| record.sequence);
            events
        }
    }

    #[async_trait]
    impl SsePersistence for MockPersistence {
        async fn record_event(
            &self,
            user_id: Uuid,
            record: &StreamEventRecord,
        ) -> anyhow::Result<()> {
            let mut guard = self.events.lock().await;
            let slot = guard.entry(user_id).or_default();
            if let Some(existing) = slot.iter_mut().find(|r| r.sequence == record.sequence) {
                *existing = record.clone();
            } else {
                slot.push(record.clone());
            }
            Ok(())
        }

        async fn load_recent_events(
            &self,
            user_id: Uuid,
            limit: usize,
        ) -> anyhow::Result<Vec<StreamEventRecord>> {
            let mut events = self.events_for(user_id).await;
            if events.len() > limit {
                events = events.split_off(events.len().saturating_sub(limit));
            }
            Ok(events)
        }

        async fn load_events_after(
            &self,
            user_id: Uuid,
            last_sequence: i64,
            limit: usize,
        ) -> anyhow::Result<Vec<StreamEventRecord>> {
            let events = self.events_for(user_id).await;
            let filtered = events
                .into_iter()
                .filter(|record| record.sequence > last_sequence)
                .take(limit)
                .collect();
            Ok(filtered)
        }

        async fn prune_events(
            &self,
            user_id: Uuid,
            max_events: usize,
            _prune_batch: usize,
        ) -> anyhow::Result<()> {
            if max_events == 0 {
                return Ok(());
            }

            let mut guard = self.events.lock().await;
            if let Some(events) = guard.get_mut(&user_id) {
                if events.len() > max_events {
                    events.sort_by_key(|record| record.sequence);
                    let keep_from = events.len().saturating_sub(max_events);
                    events.drain(0..keep_from);
                }
            }
            Ok(())
        }
    }

    fn persistence_config() -> SsePersistenceConfig {
        SsePersistenceConfig {
            enabled: true,
            max_events_per_user: 50,
            prune_batch_size: 10,
        }
    }

    #[tokio::test]
    async fn persistence_hydrates_backlog_on_subscribe() {
        let config = test_config();
        let user_id = Uuid::new_v4();
        let persistence: Arc<dyn SsePersistence> = Arc::new(MockPersistence::with_events(
            user_id,
            vec![StreamEventRecord {
                sequence: 0,
                event_id: "evt_0".into(),
                event_name: "message".into(),
                payload: "{\"body\":\"hello\"}".into(),
            }],
        ));

        let coordinator = Arc::new(SseCoordinator::new(
            config.sse.channel_capacity,
            config.sse.id_prefix.clone(),
            Some(persistence),
            Some(persistence_config()),
            config.sse.backpressure.clone(),
        ));

        let mut receiver = coordinator
            .subscribe(user_id, None::<String>)
            .await
            .expect("subscription succeeds");

        let replayed = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("backlog event emitted")
            .expect("backlog payload");
        assert_eq!(replayed.event, "message");

        let ack = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("connection ack emitted")
            .expect("ack payload");
        assert_eq!(ack.event, "connection");
    }

    #[tokio::test]
    async fn persistence_backfills_after_cursor() {
        let config = test_config();
        let user_id = Uuid::new_v4();
        let persistence: Arc<dyn SsePersistence> = Arc::new(MockPersistence::with_events(
            user_id,
            vec![
                StreamEventRecord {
                    sequence: 0,
                    event_id: "evt_0".into(),
                    event_name: "message".into(),
                    payload: "{\"body\":\"hello\"}".into(),
                },
                StreamEventRecord {
                    sequence: 1,
                    event_id: "evt_1".into(),
                    event_name: "message".into(),
                    payload: "{\"body\":\"world\"}".into(),
                },
            ],
        ));

        let coordinator = Arc::new(SseCoordinator::new(
            config.sse.channel_capacity,
            config.sse.id_prefix.clone(),
            Some(persistence),
            Some(persistence_config()),
            config.sse.backpressure.clone(),
        ));

        let mut receiver = coordinator
            .subscribe(user_id, Some("evt_0".to_string()))
            .await
            .expect("subscription succeeds");

        let replayed = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("replayed event")
            .expect("replayed payload");
        assert_eq!(replayed.id.as_deref(), Some("evt_1"));
    }

    #[tokio::test]
    async fn persistence_records_and_prunes_events() {
        let config = test_config();
        let user_id = Uuid::new_v4();
        let mock = Arc::new(MockPersistence::new());
        let coordinator = Arc::new(SseCoordinator::new(
            config.sse.channel_capacity,
            config.sse.id_prefix.clone(),
            Some(mock.clone()),
            Some(SsePersistenceConfig {
                enabled: true,
                max_events_per_user: 2,
                prune_batch_size: 1,
            }),
            config.sse.backpressure.clone(),
        ));

        let mut receiver = coordinator
            .subscribe(user_id, None::<String>)
            .await
            .expect("subscription succeeds");
        let _ = receiver.recv().await; // drain ack

        for index in 0..3 {
            let payload = json!({"seq": index});
            coordinator.publish(user_id, "message", payload, true).await;
        }

        let stored = mock.events_for(user_id).await;
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].sequence, 1);
        assert_eq!(stored[1].sequence, 2);
    }
}
