use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    convert::Infallible,
    fmt,
    sync::Arc,
    time::Duration,
};

use axum::{
    extract::{Extension, Path, Query},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
};
use chrono::{DateTime, Utc};
use futures::{StreamExt, stream};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{Mutex, broadcast};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    handlers::threads::build_delta_event,
    http::error::{ApiError, AppResult},
    middleware::request_context::RequestContext,
    services::{
        chat_service::ChatService,
        sse_persistence::{PersistedStreamEvent, SsePersistence, StreamEventRecord},
    },
};
use shared::{
    config::server::SsePersistenceConfig,
    models::{ConversationStreamEvent, ReplyMessageResponse},
};

#[derive(Clone, Copy, Debug, Default)]
struct EventMetadata {
    chunk_index: Option<i32>,
}

impl EventMetadata {
    const fn with_chunk_index(idx: i32) -> Self {
        Self {
            chunk_index: Some(idx),
        }
    }
}

#[derive(Clone, Debug)]
struct EventEnvelope {
    sequence: u64,
    timestamp_ms: i64,
    metadata: EventMetadata,
    event: ConversationStreamEvent,
}

impl EventEnvelope {
    const fn root_id(&self) -> Option<Uuid> {
        event_root_id(&self.event)
    }

    const fn message_id(&self) -> Option<Uuid> {
        match &self.event {
            ConversationStreamEvent::MessageDelta { payload } => Some(payload.message_id),
            ConversationStreamEvent::MessageDone { payload } => Some(payload.message_id),
            _ => None,
        }
    }

    const fn chunk_index(&self) -> Option<i32> {
        self.metadata.chunk_index
    }

    fn event_id(&self) -> String {
        format_event_id(
            self.root_id(),
            self.message_id(),
            self.chunk_index(),
            self.timestamp_ms,
        )
    }

    fn as_sse_event(&self) -> Option<Event> {
        let name = event_name(&self.event);
        let data = serde_json::to_string(&self.event).ok()?;
        Some(Event::default().event(name).data(data).id(self.event_id()))
    }
}

#[derive(Clone, Copy, Debug)]
struct ReplayCursor {
    _root_id: Option<Uuid>,
    message_id: Option<Uuid>,
    chunk_index: Option<i32>,
    timestamp_ms: i64,
}

#[derive(Clone, Copy, Debug)]
struct ParsedEventId {
    root_id: Option<Uuid>,
    message_id: Option<Uuid>,
    chunk_index: Option<i32>,
    timestamp_ms: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StreamQuery {
    since: Option<i64>,
}
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
    sender: broadcast::Sender<EventEnvelope>,
    state: Mutex<ConversationState>,
}

struct ConversationState {
    next_sequence: u64,
    last_timestamp_ms: i64,
    history: VecDeque<EventEnvelope>,
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

    pub fn replay_limit(&self) -> usize {
        self.persistence_config
            .as_ref()
            .map(|cfg| cfg.max_events_per_user)
            .filter(|limit| *limit > 0)
            .unwrap_or(500)
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
                last_timestamp_ms: 0,
                history: VecDeque::new(),
            }),
        });
        guard.insert(conversation_id, channel.clone());
        channel
    }

    pub async fn publish(&self, conversation_id: Uuid, event: ConversationStreamEvent) {
        self.publish_with_metadata(conversation_id, event, EventMetadata::default())
            .await;
    }

    async fn publish_with_metadata(
        &self,
        conversation_id: Uuid,
        event: ConversationStreamEvent,
        metadata: EventMetadata,
    ) {
        let channel = self.get_channel(conversation_id).await;
        let mut state = channel.state.lock().await;
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.saturating_add(1);

        let now_ms = Utc::now().timestamp_millis();
        let timestamp_ms = if state.last_timestamp_ms >= now_ms {
            state.last_timestamp_ms.saturating_add(1)
        } else {
            now_ms
        };
        state.last_timestamp_ms = timestamp_ms;

        let envelope = EventEnvelope {
            sequence,
            timestamp_ms,
            metadata,
            event: event.clone(),
        };

        state.history.push_back(envelope.clone());
        if state.history.len() > self.history_capacity {
            state.history.pop_front();
        }
        drop(state);

        let _ = channel.sender.send(envelope.clone());

        persist_event(self, conversation_id, &envelope).await;
    }

    pub async fn publish_chunk_event(
        &self,
        conversation_id: Uuid,
        event: ConversationStreamEvent,
        chunk_index: i32,
    ) {
        self.publish_with_metadata(
            conversation_id,
            event,
            EventMetadata::with_chunk_index(chunk_index),
        )
        .await;
    }

    async fn subscribe(
        &self,
        conversation_id: Uuid,
        replay_cursor: Option<ReplayCursor>,
    ) -> (broadcast::Receiver<EventEnvelope>, Vec<EventEnvelope>) {
        let channel = self.get_channel(conversation_id).await;
        let in_memory = {
            let state = channel.state.lock().await;
            state.history.iter().cloned().collect::<Vec<_>>()
        };

        let persisted = self.load_persisted(conversation_id, replay_cursor).await;

        let mut ordered: BTreeMap<u64, EventEnvelope> = BTreeMap::new();
        for envelope in persisted
            .into_iter()
            .chain(in_memory.into_iter())
            .filter(|env| should_include_event(env, replay_cursor.as_ref()))
        {
            ordered.insert(envelope.sequence, envelope);
        }

        (channel.sender.subscribe(), ordered.into_values().collect())
    }

    async fn load_persisted(
        &self,
        conversation_id: Uuid,
        replay_cursor: Option<ReplayCursor>,
    ) -> Vec<EventEnvelope> {
        let Some(store) = &self.persistence else {
            return Vec::new();
        };
        let Some(config) = &self.persistence_config else {
            return Vec::new();
        };
        if config.max_events_per_user == 0 {
            return Vec::new();
        }

        let since = replay_cursor
            .and_then(|cursor| DateTime::<Utc>::from_timestamp_millis(cursor.timestamp_ms));

        let query_result = store
            .replay_events(conversation_id, since, config.max_events_per_user)
            .await;

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

async fn persist_event(hub: &StreamHub, conversation_id: Uuid, envelope: &EventEnvelope) {
    let Some(store) = &hub.persistence else {
        return;
    };

    match serde_json::to_value(&envelope.event) {
        Ok(json_payload) => {
            let record = StreamEventRecord {
                sequence: envelope.sequence as i64,
                event_id: envelope.event_id(),
                event_type: event_name(&envelope.event).to_string(),
                payload: json_payload,
                root_message_id: envelope.root_id(),
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

fn convert_persisted_record(record: PersistedStreamEvent) -> Option<EventEnvelope> {
    let PersistedStreamEvent {
        sequence,
        event_id: stored_event_id,
        event_type: stored_event_type,
        payload,
        root_message_id: stored_root_id,
        created_at,
        ..
    } = record;

    let sequence = u64::try_from(sequence).ok()?;
    let event = deserialize_persisted_event(payload)?;

    let parsed = parse_event_id(&stored_event_id).or_else(|| {
        Some(ParsedEventId {
            root_id: stored_root_id,
            message_id: match &event {
                ConversationStreamEvent::MessageDelta { payload } => Some(payload.message_id),
                ConversationStreamEvent::MessageDone { payload } => Some(payload.message_id),
                _ => None,
            },
            chunk_index: None,
            timestamp_ms: created_at.timestamp_millis(),
        })
    })?;

    validate_persisted_event(&event, &stored_event_type, stored_root_id, &parsed);

    Some(EventEnvelope {
        sequence,
        timestamp_ms: parsed.timestamp_ms,
        metadata: EventMetadata {
            chunk_index: parsed.chunk_index,
        },
        event,
    })
}

async fn chunk_replay_events(
    service: &ChatService,
    actor: Uuid,
    cursor: Option<&ReplayCursor>,
    limit: usize,
    existing: &[EventEnvelope],
) -> Result<Vec<EventEnvelope>, ApiError> {
    let Some(cursor) = cursor else {
        return Ok(Vec::new());
    };

    let Some(message_id) = cursor.message_id else {
        return Ok(Vec::new());
    };

    let start_idx = cursor.chunk_index.unwrap_or(-1).saturating_add(1);
    let start_idx = start_idx.max(0);

    let message = service.get_message(actor, message_id).await?;
    let reply = ReplyMessageResponse {
        message_id: message.id,
        root_id: message.root_id,
        conversation_id: message.conversation_id,
        parent_id: message.parent_id,
        depth: message.depth,
    };

    let chunk_limit = limit.min(i32::MAX as usize);
    let chunk_limit = chunk_limit.max(1);
    let chunk_limit_i32 = i32::try_from(chunk_limit).unwrap_or(i32::MAX);

    let chunks = service
        .list_chunks(actor, message_id, Some(start_idx), Some(chunk_limit_i32))
        .await?;

    if chunks.is_empty() {
        return Ok(Vec::new());
    }

    let mut seen = existing_chunk_keys(existing);
    let mut envelopes = Vec::with_capacity(chunks.len());

    for chunk in chunks {
        let key = (chunk.message_id, chunk.idx);
        if !seen.insert(key) {
            continue;
        }
        if chunk.content.is_empty() {
            continue;
        }

        let event = build_delta_event(&reply, &chunk.content, chunk.idx);
        let timestamp_ms = chunk.created_at.0.timestamp_millis();
        envelopes.push(chunk_event_envelope(event, chunk.idx, timestamp_ms));
    }

    Ok(envelopes)
}

fn merge_replay_events(
    mut sse_events: Vec<EventEnvelope>,
    mut chunk_events: Vec<EventEnvelope>,
) -> Vec<EventEnvelope> {
    if chunk_events.is_empty() {
        return sse_events;
    }

    sse_events.append(&mut chunk_events);
    sse_events.sort_by(|a, b| {
        a.timestamp_ms
            .cmp(&b.timestamp_ms)
            .then(a.sequence.cmp(&b.sequence))
    });
    sse_events
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
    event: &ConversationStreamEvent,
    stored_event_type: &str,
    stored_root_id: Option<Uuid>,
    parsed_id: &ParsedEventId,
) {
    warn_on_type_mismatch(stored_event_type, event);
    warn_on_root_mismatch(stored_root_id, event);
    if let Some(root_from_id) = parsed_id.root_id {
        if Some(root_from_id) != event_root_id(event) {
            warn!(stored = %root_from_id, "SSE event id root mismatch in persistence");
        }
    }
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

#[instrument(skip(app_state, context, hub, headers))]
pub async fn conversation_stream(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Extension(hub): Extension<SharedStreamHub>,
    Path(conversation_id): Path<Uuid>,
    Query(params): Query<StreamQuery>,
    headers: HeaderMap,
) -> AppResult<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>> {
    let user_id = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);
    service.ensure_membership(user_id, conversation_id).await?;

    let header_cursor = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_last_event_id);

    let since_override = params.since.map(|value| value.max(0));
    let replay_cursor = build_replay_cursor(header_cursor, since_override);

    let (receiver, mut replay) = hub.subscribe(conversation_id, replay_cursor).await;
    let chunk_limit = hub.replay_limit();
    let chunk_events = chunk_replay_events(
        &service,
        user_id,
        replay_cursor.as_ref(),
        chunk_limit,
        &replay,
    )
    .await?;
    replay = merge_replay_events(replay, chunk_events);

    let initial =
        stream::iter(replay).filter_map(|envelope| async move { convert_event(envelope) });
    let broadcast = BroadcastStream::new(receiver).filter_map(|result| async move {
        match result {
            Ok(envelope) => convert_event(envelope),
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

fn convert_event(envelope: EventEnvelope) -> Option<Event> {
    envelope.as_sse_event()
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

fn format_event_id(
    root_id: Option<Uuid>,
    message_id: Option<Uuid>,
    chunk_index: Option<i32>,
    timestamp_ms: i64,
) -> String {
    let root = root_id.unwrap_or_else(Uuid::nil);
    let message = message_id.unwrap_or_else(Uuid::nil);
    let idx = chunk_index.unwrap_or(-1);
    format!("{root}:{message}:{idx}:{timestamp_ms}")
}

fn parse_event_id(raw: &str) -> Option<ParsedEventId> {
    let mut parts = raw.splitn(4, ':');
    let root_raw = parts.next()?;
    let message_raw = parts.next()?;
    let idx_raw = parts.next()?;
    let timestamp_raw = parts.next()?;

    let root_id = Uuid::parse_str(root_raw)
        .ok()
        .and_then(|value| if value.is_nil() { None } else { Some(value) });
    let message_id = Uuid::parse_str(message_raw)
        .ok()
        .and_then(|value| if value.is_nil() { None } else { Some(value) });

    let idx = idx_raw.parse::<i32>().ok()?;
    let chunk_index = if idx >= 0 { Some(idx) } else { None };
    let timestamp_ms = timestamp_raw.parse::<i64>().ok()?;

    Some(ParsedEventId {
        root_id,
        message_id,
        chunk_index,
        timestamp_ms,
    })
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

fn should_include_event(event: &EventEnvelope, cursor: Option<&ReplayCursor>) -> bool {
    let Some(cursor) = cursor else {
        return true;
    };

    if event.timestamp_ms < cursor.timestamp_ms {
        return false;
    }

    if event.timestamp_ms == cursor.timestamp_ms {
        match (cursor.message_id, event.message_id()) {
            (Some(cursor_msg), Some(event_msg)) if cursor_msg == event_msg => {
                match (cursor.chunk_index, event.chunk_index()) {
                    (Some(cursor_idx), Some(event_idx)) => {
                        if event_idx <= cursor_idx {
                            return false;
                        }
                    }
                    (Some(_), None) => return false,
                    _ => {}
                }
            }
            (None, None) => return false,
            _ => {}
        }
    }

    true
}

fn parse_last_event_id(raw: &str) -> Option<ReplayCursor> {
    let parsed = parse_event_id(raw)?;
    Some(ReplayCursor {
        _root_id: parsed.root_id,
        message_id: parsed.message_id,
        chunk_index: parsed.chunk_index,
        timestamp_ms: parsed.timestamp_ms,
    })
}

const fn build_replay_cursor(
    header_cursor: Option<ReplayCursor>,
    since_override: Option<i64>,
) -> Option<ReplayCursor> {
    match (header_cursor, since_override) {
        (None, None) => None,
        (Some(mut cursor), Some(since)) => {
            cursor.timestamp_ms = since;
            Some(cursor)
        }
        (Some(cursor), None) => Some(cursor),
        (None, Some(since)) => Some(ReplayCursor {
            _root_id: None,
            message_id: None,
            chunk_index: None,
            timestamp_ms: since,
        }),
    }
}

fn chunk_event_envelope(
    event: ConversationStreamEvent,
    chunk_index: i32,
    timestamp_ms: i64,
) -> EventEnvelope {
    EventEnvelope {
        sequence: chunk_index.max(0) as u64,
        timestamp_ms,
        metadata: EventMetadata::with_chunk_index(chunk_index),
        event,
    }
}

fn existing_chunk_keys(events: &[EventEnvelope]) -> HashSet<(Uuid, i32)> {
    let mut keys = HashSet::new();
    for envelope in events {
        if let (Some(message_id), Some(idx)) = (envelope.message_id(), envelope.chunk_index()) {
            keys.insert((message_id, idx));
        }
    }
    keys
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
    use chrono::{DateTime, Utc};
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

        async fn replay_events(
            &self,
            conversation_id: Uuid,
            since: Option<DateTime<Utc>>,
            limit: usize,
        ) -> Result<Vec<PersistedStreamEvent>> {
            let events = {
                let guard = self.records.lock().unwrap();
                guard.get(&conversation_id).cloned().unwrap_or_default()
            };
            let filtered = events
                .into_iter()
                .filter(|record| since.map_or(true, |threshold| record.created_at > threshold))
                .take(limit)
                .collect();
            Ok(filtered)
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

        hub.publish_chunk_event(
            conversation,
            sample_delta(message_id, root_id, conversation),
            0,
        )
        .await;
        hub.publish_chunk_event(
            conversation,
            sample_done(message_id, root_id, conversation),
            1,
        )
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
        let sequences: Vec<u64> = replay.iter().map(|env| env.sequence).collect();

        assert_eq!(sequences, vec![0, 1, 2]);
        if let Some(ConversationStreamEvent::ThreadActivity { .. }) =
            replay.last().map(|env| &env.event)
        {
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

        hub.publish_chunk_event(
            conversation,
            sample_delta(message_id, root_id, conversation),
            0,
        )
        .await;
        hub.publish_chunk_event(
            conversation,
            sample_done(message_id, root_id, conversation),
            1,
        )
        .await;

        let restored = StreamHub::new(128, Some(persistence.clone()), config.clone());
        let (_, replay) = restored.subscribe(conversation, None).await;
        let event_names: Vec<&'static str> = replay
            .into_iter()
            .map(|env| match env.event {
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
        hub.publish_chunk_event(
            conversation,
            sample_delta(message_id, root_id, conversation),
            0,
        )
        .await;
        hub.publish_chunk_event(
            conversation,
            sample_done(message_id, root_id, conversation),
            1,
        )
        .await;

        let restored = StreamHub::new(128, Some(persistence.clone()), config.clone());
        let (_, initial) = restored.subscribe(conversation, None).await;
        let first = initial.first().expect("expected at least one event");
        let cursor = ReplayCursor {
            _root_id: first.root_id(),
            message_id: first.message_id(),
            chunk_index: first.chunk_index(),
            timestamp_ms: first.timestamp_ms,
        };
        let (_, replay) = restored.subscribe(conversation, Some(cursor)).await;
        let event_names: Vec<&'static str> = replay
            .into_iter()
            .map(|env| match env.event {
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

    #[test]
    fn merge_replay_events_sorts_by_timestamp() {
        let activity = ConversationStreamEvent::ThreadActivity {
            payload: ThreadActivityEvent {
                root_id: Uuid::new_v4(),
                last_activity_at: Timestamp(Utc::now()),
            },
        };
        let sse_events = vec![EventEnvelope {
            sequence: 10,
            timestamp_ms: 200,
            metadata: EventMetadata { chunk_index: None },
            event: activity,
        }];

        let root_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let merged = super::merge_replay_events(
            sse_events,
            vec![chunk_event_envelope(
                sample_delta(message_id, root_id, conversation_id),
                0,
                150,
            )],
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].timestamp_ms, 150);
        assert_eq!(merged[1].timestamp_ms, 200);

        let keys = existing_chunk_keys(&merged);
        assert!(keys.contains(&(message_id, 0)));
    }
}
