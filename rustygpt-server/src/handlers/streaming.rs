use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    convert::Infallible,
    fmt,
    sync::Arc,
    time::Duration,
};

use axum::{
    extract::{Extension, Path},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
};
use futures::{StreamExt, stream};
use serde_json::json;
use tokio::sync::{Mutex, broadcast};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    http::error::{ApiError, AppResult},
    middleware::request_context::RequestContext,
    services::{
        chat_service::ChatService,
        sse_persistence::{PersistedStreamEvent, SsePersistence, StreamEventRecord},
    },
};
use shared::{config::server::SsePersistenceConfig, models::ConversationStreamEvent};

type StampedEvent = (u64, ConversationStreamEvent);

#[derive(Clone)]
pub struct StreamHub {
    inner: Arc<Mutex<HashMap<Uuid, Arc<ConversationChannel>>>>,
    history_capacity: usize,
    persistence: Option<Arc<dyn SsePersistence>>,
    persistence_config: Option<SsePersistenceConfig>,
}

impl fmt::Debug for StreamHub {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamHub")
            .field("history_capacity", &self.history_capacity)
            .field("has_persistence", &self.persistence.is_some())
            .finish()
    }
}

struct ConversationChannel {
    sender: broadcast::Sender<StampedEvent>,
    state: Mutex<ConversationState>,
}

struct ConversationState {
    next_sequence: u64,
    history: VecDeque<StampedEvent>,
}

pub type SharedStreamHub = Arc<StreamHub>;

impl StreamHub {
    pub fn new(
        history_capacity: usize,
        persistence: Option<Arc<dyn SsePersistence>>,
        persistence_config: Option<SsePersistenceConfig>,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            history_capacity: history_capacity.max(64),
            persistence,
            persistence_config,
        }
    }

    async fn get_channel(&self, conversation_id: Uuid) -> Arc<ConversationChannel> {
        let mut guard = self.inner.lock().await;
        if let Some(channel) = guard.get(&conversation_id) {
            return channel.clone();
        }

        let (sender, _receiver) = broadcast::channel(256);
        let channel = Arc::new(ConversationChannel {
            sender,
            state: Mutex::new(ConversationState {
                next_sequence: 0,
                history: VecDeque::new(),
            }),
        });
        guard.insert(conversation_id, channel.clone());
        channel
    }

    pub async fn publish(&self, conversation_id: Uuid, event: ConversationStreamEvent) {
        let channel = self.get_channel(conversation_id).await;
        let mut state = channel.state.lock().await;
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.saturating_add(1);

        let stamped = (sequence, event.clone());
        state.history.push_back(stamped.clone());
        if state.history.len() > self.history_capacity {
            state.history.pop_front();
        }
        drop(state);

        let _ = channel.sender.send(stamped);

        persist_event(self, conversation_id, sequence, &event).await;
    }

    pub async fn subscribe(
        &self,
        conversation_id: Uuid,
        after_sequence: Option<u64>,
    ) -> (broadcast::Receiver<StampedEvent>, Vec<StampedEvent>) {
        let channel = self.get_channel(conversation_id).await;
        let in_memory = {
            let state = channel.state.lock().await;
            state
                .history
                .iter()
                .filter(|(seq, _)| after_sequence.map_or(true, |last| *seq > last))
                .cloned()
                .collect::<Vec<_>>()
        };

        let persisted = self.load_persisted(conversation_id, after_sequence).await;

        let mut ordered = BTreeMap::new();
        for stamped in persisted.into_iter().chain(in_memory.into_iter()) {
            ordered.insert(stamped.0, stamped);
        }

        (channel.sender.subscribe(), ordered.into_values().collect())
    }

    async fn load_persisted(
        &self,
        conversation_id: Uuid,
        after_sequence: Option<u64>,
    ) -> Vec<StampedEvent> {
        let Some(store) = &self.persistence else {
            return Vec::new();
        };
        let Some(config) = &self.persistence_config else {
            return Vec::new();
        };
        if config.max_events_per_user == 0 {
            return Vec::new();
        }

        let limit = config.max_events_per_user;
        let query_result = if let Some(last) = after_sequence {
            store
                .load_events_after(conversation_id, last as i64, limit)
                .await
        } else {
            store.load_recent_events(conversation_id, limit).await
        };

        match query_result {
            Ok(records) => records
                .into_iter()
                .filter_map(convert_persisted_record)
                .collect(),
            Err(err) => {
                warn!(error = %err, "failed to load persisted SSE history");
                Vec::new()
            }
        }
    }
}

async fn persist_event(
    hub: &StreamHub,
    conversation_id: Uuid,
    sequence: u64,
    event: &ConversationStreamEvent,
) {
    let Some(store) = &hub.persistence else {
        return;
    };

    match serde_json::to_value(event) {
        Ok(json_payload) => {
            let record = StreamEventRecord {
                sequence: sequence as i64,
                event_id: event_id(sequence, event).unwrap_or_else(|| sequence.to_string()),
                event_type: event_name(event).to_string(),
                payload: json_payload,
                root_message_id: event_root_id(event),
            };

            if let Err(err) = store.record_event(conversation_id, &record).await {
                warn!(error = %err, "failed to persist SSE event");
            }

            prune_history(hub, store, conversation_id).await;
        }
        Err(err) => {
            warn!(error = %err, "failed to encode SSE event for persistence");
        }
    }
}

async fn prune_history(hub: &StreamHub, store: &Arc<dyn SsePersistence>, conversation_id: Uuid) {
    let Some(cfg) = &hub.persistence_config else {
        return;
    };
    if cfg.max_events_per_user == 0 {
        return;
    }

    let retention = Duration::from_secs(u64::from(cfg.retention_hours).saturating_mul(3600));
    let hard_limit = (cfg.max_events_per_user > 0).then_some(cfg.max_events_per_user);

    if retention == Duration::ZERO && hard_limit.is_none() {
        return;
    }

    if let Err(err) = store
        .prune_events(conversation_id, retention, cfg.prune_batch_size, hard_limit)
        .await
    {
        warn!(error = %err, "failed to prune SSE history");
    }
}

fn convert_persisted_record(record: PersistedStreamEvent) -> Option<StampedEvent> {
    let PersistedStreamEvent {
        sequence,
        event_id: stored_event_id,
        event_type: stored_event_type,
        payload,
        root_message_id: stored_root_id,
        created_at,
        ..
    } = record;

    let _ = created_at;

    let sequence = u64::try_from(sequence).ok()?;
    let event = deserialize_persisted_event(payload)?;

    validate_persisted_event(
        sequence,
        &event,
        &stored_event_type,
        stored_root_id,
        &stored_event_id,
    );

    Some((sequence, event))
}

fn deserialize_persisted_event(payload: serde_json::Value) -> Option<ConversationStreamEvent> {
    serde_json::from_value(payload)
        .map_err(|err| {
            warn!(error = %err, "failed to deserialize persisted SSE event");
            err
        })
        .ok()
}

fn validate_persisted_event(
    sequence: u64,
    event: &ConversationStreamEvent,
    stored_event_type: &str,
    stored_root_id: Option<Uuid>,
    stored_event_id: &str,
) {
    warn_on_type_mismatch(stored_event_type, event);
    warn_on_root_mismatch(stored_root_id, event);
    warn_on_id_mismatch(sequence, stored_event_id, event);
}

fn warn_on_type_mismatch(stored_event_type: &str, event: &ConversationStreamEvent) {
    let computed_type = event_name(event);
    if stored_event_type != computed_type {
        warn!(stored = %stored_event_type, computed = %computed_type, "SSE event type mismatch in persistence");
    }
}

fn warn_on_root_mismatch(stored_root_id: Option<Uuid>, event: &ConversationStreamEvent) {
    if let Some(root_id) = stored_root_id {
        if Some(root_id) != event_root_id(event) {
            warn!(stored = %root_id, "SSE event root mismatch in persistence");
        }
    }
}

fn warn_on_id_mismatch(sequence: u64, stored_event_id: &str, event: &ConversationStreamEvent) {
    if let Some(computed_id) = event_id(sequence, event) {
        if stored_event_id != computed_id {
            warn!(stored = %stored_event_id, computed = %computed_id, "SSE event id mismatch in persistence");
        }
    }
}

#[instrument(skip(app_state, context, hub, headers))]
pub async fn conversation_stream(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Extension(hub): Extension<SharedStreamHub>,
    Path(conversation_id): Path<Uuid>,
    headers: HeaderMap,
) -> AppResult<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>> {
    let user_id = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);
    service.ensure_membership(user_id, conversation_id).await?;

    let last_sequence = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_last_sequence);

    let (receiver, replay) = hub.subscribe(conversation_id, last_sequence).await;

    let initial = stream::iter(replay.into_iter().filter_map(convert_event));
    let broadcast = BroadcastStream::new(receiver).filter_map(|result| async move {
        match result {
            Ok(stamped) => convert_event(stamped),
            Err(err) => {
                tracing::warn!(error = %err, "stream subscriber lagged");
                None
            }
        }
    });

    let combined = initial.chain(broadcast).map(|event| Ok(event));

    Ok(Sse::new(combined).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(20))
            .text(json!({"type": "ping"}).to_string()),
    ))
}

fn convert_event(stamped: StampedEvent) -> Option<Event> {
    let (sequence, payload) = stamped;
    let name = event_name(&payload);
    let data = serde_json::to_string(&payload).ok()?;
    let mut event = Event::default().event(name).data(data);
    if let Some(id) = event_id(sequence, &payload) {
        event = event.id(id);
    }
    Some(event)
}

const fn event_name(event: &ConversationStreamEvent) -> &'static str {
    match event {
        ConversationStreamEvent::ThreadNew { .. } => "thread.new",
        ConversationStreamEvent::ThreadActivity { .. } => "thread.activity",
        ConversationStreamEvent::MessageDelta { .. } => "message.delta",
        ConversationStreamEvent::MessageDone { .. } => "message.done",
        ConversationStreamEvent::PresenceUpdate { .. } => "presence.update",
        ConversationStreamEvent::TypingUpdate { .. } => "typing.update",
        ConversationStreamEvent::UnreadUpdate { .. } => "unread.update",
        ConversationStreamEvent::MembershipChanged { .. } => "membership.changed",
        ConversationStreamEvent::Error { .. } => "error",
    }
}

fn event_id(sequence: u64, event: &ConversationStreamEvent) -> Option<String> {
    match event {
        ConversationStreamEvent::MessageDelta { payload } => Some(format!(
            "{}:{}:{}",
            payload.root_id, payload.message_id, sequence
        )),
        ConversationStreamEvent::MessageDone { payload } => Some(format!(
            "{}:{}:{}",
            payload.root_id, payload.message_id, sequence
        )),
        _ => Some(sequence.to_string()),
    }
}

const fn event_root_id(event: &ConversationStreamEvent) -> Option<Uuid> {
    match event {
        ConversationStreamEvent::ThreadNew { payload } => Some(payload.root_id),
        ConversationStreamEvent::ThreadActivity { payload } => Some(payload.root_id),
        ConversationStreamEvent::MessageDelta { payload } => Some(payload.root_id),
        ConversationStreamEvent::MessageDone { payload } => Some(payload.root_id),
        ConversationStreamEvent::TypingUpdate { payload } => Some(payload.root_id),
        ConversationStreamEvent::UnreadUpdate { payload } => Some(payload.root_id),
        _ => None,
    }
}

fn parse_last_sequence(raw: &str) -> Option<u64> {
    raw.split(':').last()?.parse().ok()
}

fn require_user(context: &RequestContext) -> AppResult<Uuid> {
    context
        .user_id()
        .ok_or_else(|| ApiError::forbidden("authentication required"))
}

fn require_pool(state: &AppState) -> AppResult<sqlx::PgPool> {
    state.pool.clone().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
            "database pool not configured",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use async_trait::async_trait;
    use chrono::Utc;
    use shared::models::{
        ChatDelta, ChatDeltaChoice, ChatDeltaChunk, MessageDoneEvent, MessageRole,
        ThreadActivityEvent, ThreadNewEvent, ThreadSummary, Timestamp, UsageBreakdown,
    };
    use std::{collections::HashMap, sync::Mutex, time::Duration};

    #[derive(Clone, Default)]
    struct InMemoryPersistence {
        records: Arc<Mutex<HashMap<Uuid, Vec<PersistedStreamEvent>>>>,
    }

    #[async_trait]
    impl SsePersistence for InMemoryPersistence {
        async fn record_event(
            &self,
            conversation_id: Uuid,
            record: &StreamEventRecord,
        ) -> Result<()> {
            {
                let mut guard = self.records.lock().unwrap();
                let entry = guard.entry(conversation_id).or_default();
                let persisted = PersistedStreamEvent {
                    sequence: record.sequence,
                    event_id: record.event_id.clone(),
                    event_type: record.event_type.clone(),
                    payload: record.payload.clone(),
                    root_message_id: record.root_message_id,
                    created_at: chrono::Utc::now(),
                };
                if let Some(existing) = entry
                    .iter_mut()
                    .find(|existing| existing.sequence == persisted.sequence)
                {
                    *existing = persisted;
                } else {
                    entry.push(persisted);
                    entry.sort_by_key(|rec| rec.sequence);
                }
                drop(guard);
            }
            Ok(())
        }

        async fn load_recent_events(
            &self,
            conversation_id: Uuid,
            limit: usize,
        ) -> Result<Vec<PersistedStreamEvent>> {
            let events = {
                let guard = self.records.lock().unwrap();
                guard.get(&conversation_id).cloned().unwrap_or_default()
            };
            Ok(events.into_iter().take(limit).collect())
        }

        async fn load_events_after(
            &self,
            conversation_id: Uuid,
            last_sequence: i64,
            limit: usize,
        ) -> Result<Vec<PersistedStreamEvent>> {
            let events = {
                let guard = self.records.lock().unwrap();
                guard
                    .get(&conversation_id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|record| record.sequence > last_sequence)
                    .take(limit)
                    .collect()
            };
            Ok(events)
        }

        async fn prune_events(
            &self,
            conversation_id: Uuid,
            _retention: Duration,
            prune_batch: usize,
            hard_limit: Option<usize>,
        ) -> Result<()> {
            {
                let mut guard = self.records.lock().unwrap();
                if let (Some(records), Some(limit)) = (guard.get_mut(&conversation_id), hard_limit)
                {
                    if records.len() > limit {
                        let excess = records.len().saturating_sub(limit);
                        let remove = excess.min(prune_batch);
                        records.drain(0..remove);
                    }
                }
                drop(guard);
            }
            Ok(())
        }
    }

    fn persistence_config() -> SsePersistenceConfig {
        SsePersistenceConfig {
            enabled: true,
            max_events_per_user: 100,
            prune_batch_size: 50,
            retention_hours: 48,
        }
    }

    #[tokio::test]
    async fn replay_merges_persisted_and_memory_in_order() {
        let persistence = Arc::new(InMemoryPersistence::default());
        let config = Some(persistence_config());
        let hub = StreamHub::new(32, Some(persistence.clone()), config);
        let conversation = Uuid::new_v4();
        let root_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        hub.publish(
            conversation,
            sample_delta(message_id, root_id, conversation),
        )
        .await;
        hub.publish(conversation, sample_done(message_id, root_id, conversation))
            .await;

        let activity = ConversationStreamEvent::ThreadActivity {
            payload: ThreadActivityEvent {
                root_id,
                last_activity_at: Timestamp(Utc::now()),
            },
        };
        let record = StreamEventRecord {
            sequence: 2,
            event_id: "activity:2".into(),
            event_type: "thread.activity".into(),
            payload: serde_json::to_value(&activity).unwrap(),
            root_message_id: Some(root_id),
        };
        persistence
            .record_event(conversation, &record)
            .await
            .expect("persist activity");

        let (_, replay) = hub.subscribe(conversation, None).await;
        let sequences: Vec<u64> = replay.iter().map(|(seq, _)| *seq).collect();

        assert_eq!(sequences, vec![0, 1, 2]);
        if let Some((_, ConversationStreamEvent::ThreadActivity { .. })) = replay.last() {
            // expected activity from persistence
        } else {
            panic!("expected persisted thread activity event at the end");
        }
    }

    fn sample_delta(
        message_id: Uuid,
        root_id: Uuid,
        conversation_id: Uuid,
    ) -> ConversationStreamEvent {
        ConversationStreamEvent::MessageDelta {
            payload: ChatDeltaChunk {
                id: format!("{}:0", message_id),
                object: "chat.completion.chunk".to_string(),
                root_id,
                message_id,
                conversation_id,
                parent_id: None,
                depth: Some(1),
                choices: vec![ChatDeltaChoice {
                    index: 0,
                    delta: ChatDelta {
                        role: Some(MessageRole::Assistant),
                        content: Some("hello".to_string()),
                    },
                    finish_reason: None,
                }],
            },
        }
    }

    fn sample_done(
        message_id: Uuid,
        root_id: Uuid,
        conversation_id: Uuid,
    ) -> ConversationStreamEvent {
        ConversationStreamEvent::MessageDone {
            payload: MessageDoneEvent {
                message_id,
                root_id,
                conversation_id,
                finish_reason: Some("stop".to_string()),
                usage: Some(UsageBreakdown {
                    prompt_tokens: 5,
                    completion_tokens: 7,
                    total_tokens: 12,
                }),
            },
        }
    }

    #[tokio::test]
    async fn replay_includes_thread_events_from_persistence() {
        let persistence = Arc::new(InMemoryPersistence::default());
        let config = Some(persistence_config());
        let hub = StreamHub::new(128, Some(persistence.clone()), config.clone());
        let conversation = Uuid::new_v4();
        let root_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        hub.publish(
            conversation,
            ConversationStreamEvent::ThreadNew {
                payload: ThreadNewEvent {
                    conversation_id: conversation,
                    root_id,
                    summary: ThreadSummary {
                        root_id,
                        root_excerpt: "hello".into(),
                        root_author: None,
                        created_at: Timestamp(Utc::now()),
                        last_activity_at: Timestamp(Utc::now()),
                        message_count: 1,
                        participant_count: 1,
                    },
                },
            },
        )
        .await;

        hub.publish(
            conversation,
            ConversationStreamEvent::ThreadActivity {
                payload: ThreadActivityEvent {
                    root_id,
                    last_activity_at: Timestamp(Utc::now()),
                },
            },
        )
        .await;

        hub.publish(
            conversation,
            sample_delta(message_id, root_id, conversation),
        )
        .await;
        hub.publish(conversation, sample_done(message_id, root_id, conversation))
            .await;

        let restored = StreamHub::new(128, Some(persistence.clone()), config.clone());
        let (_, replay) = restored.subscribe(conversation, None).await;
        let event_names: Vec<&'static str> = replay
            .into_iter()
            .map(|(_, event)| match event {
                ConversationStreamEvent::ThreadNew { .. } => "thread.new",
                ConversationStreamEvent::ThreadActivity { .. } => "thread.activity",
                ConversationStreamEvent::MessageDelta { .. } => "message.delta",
                ConversationStreamEvent::MessageDone { .. } => "message.done",
                ConversationStreamEvent::PresenceUpdate { .. } => "presence.update",
                ConversationStreamEvent::TypingUpdate { .. } => "typing.update",
                ConversationStreamEvent::UnreadUpdate { .. } => "unread.update",
                ConversationStreamEvent::MembershipChanged { .. } => "membership.changed",
                ConversationStreamEvent::Error { .. } => "error",
            })
            .collect();

        assert_eq!(
            event_names,
            vec![
                "thread.new",
                "thread.activity",
                "message.delta",
                "message.done"
            ]
        );
    }

    #[tokio::test]
    async fn subscribe_after_sequence_filters_events() {
        let persistence = Arc::new(InMemoryPersistence::default());
        let config = Some(persistence_config());
        let hub = StreamHub::new(128, Some(persistence.clone()), config.clone());
        let conversation = Uuid::new_v4();
        let root_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        hub.publish(
            conversation,
            ConversationStreamEvent::ThreadNew {
                payload: ThreadNewEvent {
                    conversation_id: conversation,
                    root_id,
                    summary: ThreadSummary {
                        root_id,
                        root_excerpt: "greetings".into(),
                        root_author: None,
                        created_at: Timestamp(Utc::now()),
                        last_activity_at: Timestamp(Utc::now()),
                        message_count: 1,
                        participant_count: 1,
                    },
                },
            },
        )
        .await;
        hub.publish(
            conversation,
            sample_delta(message_id, root_id, conversation),
        )
        .await;
        hub.publish(conversation, sample_done(message_id, root_id, conversation))
            .await;

        let restored = StreamHub::new(128, Some(persistence.clone()), config.clone());
        let (_, replay) = restored.subscribe(conversation, Some(0)).await;
        let event_names: Vec<&'static str> = replay
            .into_iter()
            .map(|(_, event)| match event {
                ConversationStreamEvent::ThreadNew { .. } => "thread.new",
                ConversationStreamEvent::ThreadActivity { .. } => "thread.activity",
                ConversationStreamEvent::MessageDelta { .. } => "message.delta",
                ConversationStreamEvent::MessageDone { .. } => "message.done",
                ConversationStreamEvent::PresenceUpdate { .. } => "presence.update",
                ConversationStreamEvent::TypingUpdate { .. } => "typing.update",
                ConversationStreamEvent::UnreadUpdate { .. } => "unread.update",
                ConversationStreamEvent::MembershipChanged { .. } => "membership.changed",
                ConversationStreamEvent::Error { .. } => "error",
            })
            .collect();

        assert_eq!(event_names, vec!["message.delta", "message.done"]);
    }

    #[tokio::test]
    async fn prune_limits_persisted_events() {
        let persistence = Arc::new(InMemoryPersistence::default());
        let mut cfg = persistence_config();
        cfg.max_events_per_user = 3;
        cfg.prune_batch_size = 2;
        let config = Some(cfg);

        let hub = StreamHub::new(128, Some(persistence.clone()), config.clone());
        let conversation = Uuid::new_v4();
        let root_id = Uuid::new_v4();

        for _ in 0..6 {
            hub.publish(
                conversation,
                ConversationStreamEvent::ThreadActivity {
                    payload: ThreadActivityEvent {
                        root_id,
                        last_activity_at: Timestamp(Utc::now()),
                    },
                },
            )
            .await;
        }

        let stored = persistence
            .records
            .lock()
            .unwrap()
            .get(&conversation)
            .cloned()
            .unwrap_or_default();
        assert!(stored.len() <= 3);

        let restored = StreamHub::new(128, Some(persistence), config);
        let (_, replay) = restored.subscribe(conversation, None).await;
        assert!(replay.len() <= 3);
    }
}
