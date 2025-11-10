use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use shared::config::server::SsePersistenceConfig;
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use tracing::trace;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct StreamEventRecord {
    pub sequence: i64,
    pub event_id: String,
    pub event_type: String,
    pub payload: Value,
    pub root_message_id: Option<Uuid>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PersistedStreamEvent {
    pub sequence: i64,
    pub event_id: String,
    pub event_type: String,
    pub payload: Value,
    pub root_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait SsePersistence: Send + Sync {
    async fn record_event(&self, conversation_id: Uuid, record: &StreamEventRecord) -> Result<()>;
    async fn replay_events(
        &self,
        conversation_id: Uuid,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<PersistedStreamEvent>>;
    async fn prune_events(
        &self,
        conversation_id: Uuid,
        retention: Duration,
        prune_batch: usize,
        hard_limit: Option<usize>,
    ) -> Result<()>;
}

#[derive(Clone)]
pub struct SsePersistenceStore {
    pool: PgPool,
    _config: Arc<SsePersistenceConfig>,
}

impl SsePersistenceStore {
    pub fn new(pool: PgPool, config: SsePersistenceConfig) -> Self {
        Self {
            pool,
            _config: Arc::new(config),
        }
    }
}

#[async_trait]
impl SsePersistence for SsePersistenceStore {
    async fn record_event(&self, conversation_id: Uuid, record: &StreamEventRecord) -> Result<()> {
        sqlx::query("CALL rustygpt.sp_record_sse_event($1, $2, $3, $4, $5, $6)")
            .bind(conversation_id)
            .bind(record.sequence)
            .bind(&record.event_id)
            .bind(&record.event_type)
            .bind(&record.payload)
            .bind(record.root_message_id)
            .execute(&self.pool)
            .await?;

        trace!(conversation_id = %conversation_id, sequence = record.sequence, "persisted SSE event");
        Ok(())
    }

    async fn replay_events(
        &self,
        conversation_id: Uuid,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<PersistedStreamEvent>> {
        self.replay_events_internal(conversation_id, since, limit)
            .await
    }

    async fn prune_events(
        &self,
        conversation_id: Uuid,
        retention: Duration,
        prune_batch: usize,
        hard_limit: Option<usize>,
    ) -> Result<()> {
        if retention.is_zero() && hard_limit.is_none() {
            return Ok(());
        }

        let retention_seconds = retention
            .as_secs()
            .min(i32::MAX as u64)
            .try_into()
            .unwrap_or(i32::MAX);
        let batch = i32::try_from(prune_batch.max(1).min(i32::MAX as usize)).unwrap_or(i32::MAX);
        let hard_limit = hard_limit
            .filter(|value| *value > 0)
            .map(|value| i32::try_from(value.min(i32::MAX as usize)).unwrap_or(i32::MAX));

        sqlx::query("CALL rustygpt.sp_prune_sse_events($1, $2, $3, $4)")
            .bind(conversation_id)
            .bind(retention_seconds)
            .bind(batch)
            .bind(hard_limit)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

impl SsePersistenceStore {
    async fn replay_events_internal(
        &self,
        conversation_id: Uuid,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<PersistedStreamEvent>> {
        let rows = sqlx::query_as::<_, PersistedStreamEvent>(
            "SELECT sequence, event_id, event_type, payload, root_message_id, created_at \
             FROM rustygpt.sp_sse_replay($1, $2, $3)",
        )
        .bind(conversation_id)
        .bind(since)
        .bind(i32::try_from(limit.min(i32::MAX as usize)).unwrap_or(i32::MAX))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
