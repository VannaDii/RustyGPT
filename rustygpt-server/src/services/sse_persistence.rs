use anyhow::Result;
use async_trait::async_trait;
use shared::config::server::SsePersistenceConfig;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::trace;
use uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct StreamEventRecord {
    pub sequence: i64,
    pub event_id: String,
    pub event_name: String,
    pub payload: String,
}

#[async_trait]
pub trait SsePersistence: Send + Sync {
    async fn record_event(&self, user_id: Uuid, record: &StreamEventRecord) -> Result<()>;
    async fn load_recent_events(
        &self,
        user_id: Uuid,
        limit: usize,
    ) -> Result<Vec<StreamEventRecord>>;
    async fn load_events_after(
        &self,
        user_id: Uuid,
        last_sequence: i64,
        limit: usize,
    ) -> Result<Vec<StreamEventRecord>>;
    async fn prune_events(
        &self,
        user_id: Uuid,
        max_events: usize,
        prune_batch: usize,
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
    async fn record_event(&self, user_id: Uuid, record: &StreamEventRecord) -> Result<()> {
        sqlx::query("CALL rustygpt.sp_record_sse_event($1, $2, $3, $4, $5)")
            .bind(user_id)
            .bind(record.sequence)
            .bind(&record.event_id)
            .bind(&record.event_name)
            .bind(&record.payload)
            .execute(&self.pool)
            .await?;

        trace!(user_id = %user_id, sequence = record.sequence, "persisted SSE event");
        Ok(())
    }

    async fn load_recent_events(
        &self,
        user_id: Uuid,
        limit: usize,
    ) -> Result<Vec<StreamEventRecord>> {
        let rows = sqlx::query_as::<_, StreamEventRecord>(
            "SELECT sequence, event_id, event_name, payload \
             FROM rustygpt.fn_load_recent_sse_events($1, $2)",
        )
        .bind(user_id)
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn load_events_after(
        &self,
        user_id: Uuid,
        last_sequence: i64,
        limit: usize,
    ) -> Result<Vec<StreamEventRecord>> {
        let rows = sqlx::query_as::<_, StreamEventRecord>(
            "SELECT sequence, event_id, event_name, payload \
             FROM rustygpt.fn_load_sse_events_after($1, $2, $3)",
        )
        .bind(user_id)
        .bind(last_sequence)
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn prune_events(
        &self,
        user_id: Uuid,
        max_events: usize,
        prune_batch: usize,
    ) -> Result<()> {
        if max_events == 0 {
            return Ok(());
        }

        sqlx::query("CALL rustygpt.sp_prune_sse_events($1, $2, $3)")
            .bind(user_id)
            .bind(max_events as i32)
            .bind(prune_batch as i32)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
