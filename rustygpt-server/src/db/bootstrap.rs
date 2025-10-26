use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use sqlx::PgPool;
use thiserror::Error;
use tracing::{debug, info, warn};

use shared::config::server::DatabaseConfig;

#[derive(Debug, Clone)]
struct BootstrapStage {
    label: &'static str,
    kind: ScriptStage,
    files: &'static [&'static str],
}

const BOOTSTRAP_STAGES: &[BootstrapStage] = &[
    BootstrapStage {
        label: "schema/010_auth.sql",
        kind: ScriptStage::Schema,
        files: &[
            "schema/001_create_schema.sql",
            "schema/002_create_sse_event_log.sql",
            "schema/010_auth.sql",
        ],
    },
    BootstrapStage {
        label: "procs/010_auth.sql",
        kind: ScriptStage::Procedures,
        files: &["procs/010_auth.sql"],
    },
    BootstrapStage {
        label: "schema/020_conversations_threads.sql",
        kind: ScriptStage::Schema,
        files: &[
            "schema/020_threads.sql",
            "schema/030_membership.sql",
            "schema/050_sse_persistence.sql",
        ],
    },
    BootstrapStage {
        label: "procs/020_threads.sql",
        kind: ScriptStage::Procedures,
        files: &[
            "procs/018_conversations.sql",
            "procs/019_access.sql",
            "procs/020_threads_roots.sql",
            "procs/021_threads_tree.sql",
            "procs/022_post_root.sql",
            "procs/023_reply.sql",
            "procs/024_append_chunk.sql",
            "procs/025_thread_summary.sql",
            "procs/026_update_message_content.sql",
            "procs/030_membership.sql",
            "procs/031_presence.sql",
            "procs/032_unread.sql",
            "procs/033_message_lifecycle.sql",
        ],
    },
    BootstrapStage {
        label: "schema/040_rate_limits.sql",
        kind: ScriptStage::Schema,
        files: &["schema/040_rate_limits.sql"],
    },
    BootstrapStage {
        label: "seed/002_rate_limits.sql",
        kind: ScriptStage::Seed,
        files: &[
            "seed/001_seed_feature_flags.sql",
            "seed/002_rate_limits.sql",
        ],
    },
    BootstrapStage {
        label: "procs/034_limits.sql",
        kind: ScriptStage::Procedures,
        files: &["procs/034_limits.sql"],
    },
];

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
static READINESS_OVERRIDE: OnceLock<Mutex<Option<Result<BootstrapStatus, sqlx::Error>>>> =
    OnceLock::new();
#[cfg(test)]
static LIVENESS_OVERRIDE: OnceLock<Mutex<Option<Result<(), sqlx::Error>>>> = OnceLock::new();

#[cfg(test)]
fn readiness_override_take() -> Option<Result<BootstrapStatus, sqlx::Error>> {
    READINESS_OVERRIDE
        .get()
        .and_then(|lock| lock.lock().expect("override poisoned").take())
}

#[cfg(test)]
pub fn set_readiness_override(result: Option<Result<BootstrapStatus, sqlx::Error>>) {
    let lock = READINESS_OVERRIDE.get_or_init(|| Mutex::new(None));
    *lock.lock().expect("override poisoned") = result;
}

#[cfg(test)]
fn liveness_override_take() -> Option<Result<(), sqlx::Error>> {
    LIVENESS_OVERRIDE
        .get()
        .and_then(|lock| lock.lock().expect("override poisoned").take())
}

#[cfg(test)]
pub fn set_liveness_override(result: Option<Result<(), sqlx::Error>>) {
    let lock = LIVENESS_OVERRIDE.get_or_init(|| Mutex::new(None));
    *lock.lock().expect("override poisoned") = result;
}

#[derive(Debug, Clone, Copy)]
enum ScriptStage {
    Schema,
    Procedures,
    Seed,
}

impl ScriptStage {
    const fn label(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::Procedures => "procedures",
            Self::Seed => "seed",
        }
    }
}

impl std::fmt::Display for ScriptStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("database bootstrap directory does not exist: {0}")]
    MissingRoot(PathBuf),
    #[error("database bootstrap stage '{stage}' missing at {path}")]
    MissingStage { stage: &'static str, path: PathBuf },
    #[error("failed to read file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("database error executing {path}: {source}")]
    Sql {
        path: PathBuf,
        #[source]
        source: sqlx::Error,
    },
    #[error("failed to update bootstrap ledger for stage {stage}: {source}")]
    Ledger {
        stage: &'static str,
        #[source]
        source: sqlx::Error,
    },
}

#[derive(Debug, Clone)]
pub struct BootstrapStatus {
    pub ready: bool,
    pub missing: Vec<&'static str>,
}

/// Execute all bootstrap SQL scripts in the configured order.
pub async fn run(pool: &PgPool, config: &DatabaseConfig) -> Result<(), BootstrapError> {
    let root = &config.bootstrap_path;
    if !root.exists() {
        return Err(BootstrapError::MissingRoot(root.clone()));
    }

    info!(path = %root.display(), "running database bootstrap");
    ensure_bootstrap_ledger(pool)
        .await
        .map_err(|source| BootstrapError::Sql {
            path: root.clone(),
            source,
        })?;

    let mut applied =
        fetch_applied_stage_names(pool)
            .await
            .map_err(|source| BootstrapError::Sql {
                path: root.clone(),
                source,
            })?;

    for stage in BOOTSTRAP_STAGES {
        if applied.contains(stage.label) {
            debug!(
                stage = stage.label,
                "bootstrap stage already applied; skipping"
            );
            continue;
        }

        apply_stage(pool, root, stage).await?;
        mark_stage_applied(pool, stage).await?;
        applied.insert(stage.label.to_string());
    }

    Ok(())
}

async fn ensure_bootstrap_ledger(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("CREATE SCHEMA IF NOT EXISTS rustygpt")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustygpt.bootstrap_applied (
            stage TEXT PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn fetch_applied_stage_names(pool: &PgPool) -> Result<HashSet<String>, sqlx::Error> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT stage FROM rustygpt.bootstrap_applied ORDER BY applied_at ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().collect())
}

async fn apply_stage(
    pool: &PgPool,
    root: &Path,
    stage: &BootstrapStage,
) -> Result<(), BootstrapError> {
    if stage.files.is_empty() {
        debug!(
            stage = stage.label,
            "no scripts registered for bootstrap stage"
        );
        return Ok(());
    }

    record_stage_counter(stage, "started");
    info!(
        stage = stage.label,
        count = stage.files.len(),
        "applying bootstrap scripts"
    );

    for file in stage.files {
        let path = root.join(file);
        if !path.exists() {
            return Err(BootstrapError::MissingStage {
                stage: stage.label,
                path,
            });
        }

        let timer = Instant::now();
        match apply_script(pool, &path).await {
            Ok(_) => record_script_metrics(stage, "ok", timer.elapsed().as_secs_f64()),
            Err(err) => {
                record_script_metrics(stage, "error", timer.elapsed().as_secs_f64());
                return Err(err);
            }
        }
    }

    record_stage_counter(stage, "completed");
    Ok(())
}

fn record_stage_counter(stage: &BootstrapStage, status: &'static str) {
    metrics::counter!(
        "db_bootstrap_batches_total",
        "stage" => stage.kind.label(),
        "name" => stage.label,
        "status" => status
    )
    .increment(1);
}

fn record_script_metrics(stage: &BootstrapStage, status: &'static str, duration: f64) {
    metrics::counter!(
        "db_bootstrap_scripts_total",
        "stage" => stage.kind.label(),
        "name" => stage.label,
        "status" => status
    )
    .increment(1);
    metrics::histogram!(
        "db_bootstrap_script_duration_seconds",
        "stage" => stage.kind.label(),
        "name" => stage.label
    )
    .record(duration);
}

async fn mark_stage_applied(pool: &PgPool, stage: &BootstrapStage) -> Result<(), BootstrapError> {
    sqlx::query(
        "INSERT INTO rustygpt.bootstrap_applied (stage, applied_at)
         VALUES ($1, now())
         ON CONFLICT (stage) DO NOTHING",
    )
    .bind(stage.label)
    .execute(pool)
    .await
    .map_err(|source| BootstrapError::Ledger {
        stage: stage.label,
        source,
    })?;

    Ok(())
}

/// Simple liveness check used during startup.
pub async fn ensure_liveness(pool: &PgPool) -> Result<(), sqlx::Error> {
    #[cfg(test)]
    if let Some(result) = liveness_override_take() {
        return result;
    }

    match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => {
            metrics::counter!("db_liveness_checks_total", "status" => "ok").increment(1);
            Ok(())
        }
        Err(err) => {
            metrics::counter!("db_liveness_checks_total", "status" => "error").increment(1);
            Err(err)
        }
    }
}

/// Readiness probe reporting bootstrap completion state.
pub async fn readiness_state(pool: &PgPool) -> Result<BootstrapStatus, sqlx::Error> {
    #[cfg(test)]
    if let Some(result) = readiness_override_take() {
        return result;
    }

    ensure_bootstrap_ledger(pool).await?;
    let applied_set = fetch_applied_stage_names(pool).await?;

    let missing = BOOTSTRAP_STAGES
        .iter()
        .filter(|stage| !applied_set.contains(stage.label))
        .map(|stage| stage.label)
        .collect::<Vec<_>>();

    if missing.is_empty() {
        metrics::counter!("db_readiness_checks_total", "status" => "ok").increment(1);
    } else {
        metrics::counter!("db_readiness_checks_total", "status" => "incomplete").increment(1);
    }

    Ok(BootstrapStatus {
        ready: missing.is_empty(),
        missing,
    })
}

async fn apply_script(pool: &PgPool, path: &Path) -> Result<(), BootstrapError> {
    let sql = fs::read_to_string(path).map_err(|source| BootstrapError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    let trimmed = sql.trim();
    if trimmed.is_empty() {
        warn!(path = %path.display(), "skipping empty bootstrap script");
        return Ok(());
    }

    let mut transaction = pool.begin().await.map_err(|source| BootstrapError::Sql {
        path: path.to_path_buf(),
        source,
    })?;

    info!(script = %path.display(), "executing bootstrap script");
    if let Err(source) = sqlx::query(trimmed).execute(&mut *transaction).await {
        return Err(BootstrapError::Sql {
            path: path.to_path_buf(),
            source,
        });
    }

    transaction
        .commit()
        .await
        .map_err(|source| BootstrapError::Sql {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use sqlx::postgres::PgPoolOptions;
    use std::io;

    #[test]
    fn bootstrap_orders_stages_in_strict_sequence() {
        let labels: Vec<&str> = BOOTSTRAP_STAGES.iter().map(|stage| stage.label).collect();
        assert_eq!(
            labels,
            vec![
                "schema/010_auth.sql",
                "procs/010_auth.sql",
                "schema/020_conversations_threads.sql",
                "procs/020_threads.sql",
                "schema/040_rate_limits.sql",
                "seed/002_rate_limits.sql",
                "procs/034_limits.sql"
            ]
        );
    }

    #[test]
    fn bootstrap_is_idempotent_when_all_stages_recorded() {
        let applied: HashSet<String> = BOOTSTRAP_STAGES
            .iter()
            .map(|stage| stage.label.to_string())
            .collect();

        let has_pending = BOOTSTRAP_STAGES
            .iter()
            .any(|stage| !applied.contains(stage.label));

        assert!(
            !has_pending,
            "no stages should remain when ledger already contains every entry"
        );
    }

    fn test_pool() -> PgPool {
        PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://postgres:postgres@localhost:5432/rustygpt_test")
            .expect("lazy pool creation should succeed")
    }

    #[tokio::test]
    async fn ensure_liveness_uses_override() {
        let pool = test_pool();
        super::set_liveness_override(Some(Ok(())));
        assert!(super::ensure_liveness(&pool).await.is_ok());
        super::set_liveness_override(None);
    }

    #[tokio::test]
    #[serial]
    async fn readiness_override_errors_propagate() {
        let pool = test_pool();
        super::set_readiness_override(Some(Err(sqlx::Error::Io(io::Error::new(
            io::ErrorKind::Other,
            "simulated failure",
        )))));

        let result = super::readiness_state(&pool).await;
        assert!(result.is_err());

        super::set_readiness_override(None);
    }

    #[tokio::test]
    #[serial]
    async fn readiness_override_ok_short_circuits() {
        let pool = test_pool();
        super::set_readiness_override(Some(Ok(BootstrapStatus {
            ready: true,
            missing: vec![],
        })));

        let result = super::readiness_state(&pool).await.unwrap();
        assert!(result.ready);

        super::set_readiness_override(None);
    }
}
