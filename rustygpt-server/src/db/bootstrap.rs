use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::{cmp::Ordering, time::Instant};

use sqlx::PgPool;
use thiserror::Error;
use tracing::{debug, info, warn};

use shared::config::server::DatabaseConfig;

const STAGES: &[(&str, ScriptStage)] = &[
    ("schema", ScriptStage::Schema),
    ("procedures", ScriptStage::Procedures),
    ("indexes", ScriptStage::Indexes),
    ("seed", ScriptStage::Seed),
];

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
static READINESS_OVERRIDE: OnceLock<Mutex<Option<Result<(), sqlx::Error>>>> = OnceLock::new();
#[cfg(test)]
static LIVENESS_OVERRIDE: OnceLock<Mutex<Option<Result<(), sqlx::Error>>>> = OnceLock::new();

#[cfg(test)]
fn readiness_override_take() -> Option<Result<(), sqlx::Error>> {
    READINESS_OVERRIDE
        .get()
        .and_then(|lock| lock.lock().expect("override poisoned").take())
}

#[cfg(test)]
pub fn set_readiness_override(result: Option<Result<(), sqlx::Error>>) {
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
    Indexes,
    Seed,
}

impl ScriptStage {
    const fn label(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::Procedures => "procedures",
            Self::Indexes => "indexes",
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
    #[error("failed to read directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
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
}

/// Execute all bootstrap SQL scripts in the configured order.
pub async fn run(pool: &PgPool, config: &DatabaseConfig) -> Result<(), BootstrapError> {
    let root = &config.bootstrap_path;
    if !root.exists() {
        return Err(BootstrapError::MissingRoot(root.clone()));
    }

    info!(path = %root.display(), "running database bootstrap");

    for (folder, stage) in STAGES {
        apply_stage(pool, root, folder, *stage).await?;
    }

    Ok(())
}

async fn apply_stage(
    pool: &PgPool,
    root: &Path,
    folder: &str,
    stage: ScriptStage,
) -> Result<(), BootstrapError> {
    let stage_path = root.join(folder);
    if !stage_path.exists() {
        return Err(BootstrapError::MissingStage {
            stage: stage.label(),
            path: stage_path,
        });
    }

    let files = collect_sql_files(&stage_path)?;
    if files.is_empty() {
        debug!(stage = %stage, "no bootstrap scripts found for stage");
        return Ok(());
    }

    info!(stage = %stage, count = files.len(), "applying bootstrap scripts");
    record_stage_counter(stage, "started");

    for path in files {
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

fn record_stage_counter(stage: ScriptStage, status: &'static str) {
    metrics::counter!(
        "db_bootstrap_batches_total",
        "stage" => stage.label(),
        "status" => status
    )
    .increment(1);
}

fn record_script_metrics(stage: ScriptStage, status: &'static str, duration: f64) {
    metrics::counter!(
        "db_bootstrap_scripts_total",
        "stage" => stage.label(),
        "status" => status
    )
    .increment(1);
    metrics::histogram!(
        "db_bootstrap_script_duration_seconds",
        "stage" => stage.label()
    )
    .record(duration);
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

/// Readiness probe that expects the health stored procedure to exist.
pub async fn ensure_readiness(pool: &PgPool) -> Result<(), sqlx::Error> {
    #[cfg(test)]
    if let Some(result) = readiness_override_take() {
        return result;
    }

    match sqlx::query("CALL sp_healthz()").execute(pool).await {
        Ok(_) => {
            metrics::counter!("db_readiness_checks_total", "status" => "ok").increment(1);
            Ok(())
        }
        Err(err) => {
            metrics::counter!("db_readiness_checks_total", "status" => "error").increment(1);
            Err(err)
        }
    }
}

fn collect_sql_files(dir: &Path) -> Result<Vec<PathBuf>, BootstrapError> {
    let mut entries = Vec::new();
    let read_dir = fs::read_dir(dir).map_err(|source| BootstrapError::ReadDir {
        path: dir.to_path_buf(),
        source,
    })?;

    for entry in read_dir {
        let entry = entry.map_err(|source| BootstrapError::ReadDir {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path
            .extension()
            .and_then(OsStr::to_str)
            .map_or(false, |ext| ext.eq_ignore_ascii_case("sql"))
        {
            entries.push(path);
        }
    }

    entries.sort_by(|a, b| compare_paths(a, b));
    Ok(entries)
}

fn compare_paths(a: &Path, b: &Path) -> Ordering {
    match (a.file_name(), b.file_name()) {
        (Some(a_name), Some(b_name)) => a_name.cmp(b_name),
        _ => Ordering::Equal,
    }
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
    use sqlx::postgres::PgPoolOptions;
    use std::io;
    use tempfile::tempdir;

    #[test]
    fn collects_sql_files_in_order() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("002_second.sql"), "SELECT 1;").unwrap();
        std::fs::write(dir.path().join("001_first.sql"), "SELECT 1;").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "ignore me").unwrap();

        let files = collect_sql_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(
            files[0]
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("001")
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
    async fn ensure_readiness_override_errors() {
        let pool = test_pool();
        super::set_readiness_override(Some(Err(sqlx::Error::Io(io::Error::new(
            io::ErrorKind::Other,
            "simulated failure",
        )))));

        let result = super::ensure_readiness(&pool).await;
        assert!(result.is_err());

        super::set_readiness_override(None);
    }
}
