use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, Copy)]
enum ScriptStage {
    Schema,
    Procedures,
    Indexes,
    Seed,
}

impl ScriptStage {
    fn label(self) -> &'static str {
        match self {
            ScriptStage::Schema => "schema",
            ScriptStage::Procedures => "procedures",
            ScriptStage::Indexes => "indexes",
            ScriptStage::Seed => "seed",
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
            continue;
        }

        info!(stage = %stage, count = files.len(), "applying bootstrap scripts");
        for path in files {
            apply_script(pool, &path).await?;
        }
    }

    Ok(())
}

/// Simple liveness check used during startup.
pub async fn ensure_liveness(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ())
}

/// Readiness probe that expects the health stored procedure to exist.
pub async fn ensure_readiness(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("CALL sp_healthz()")
        .execute(pool)
        .await
        .map(|_| ())
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
}
