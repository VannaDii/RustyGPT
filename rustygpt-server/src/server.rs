use crate::handlers::streaming::{SharedStreamHub, StreamHub};
use app_state::AppState;
use axum::{Extension, Router, middleware, response::IntoResponse, routing::get, serve};
use routes::openapi::openapi_routes;
use shared::config::server::{Config, DatabaseConfig, LogFormat, SsePersistenceConfig};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    net::SocketAddr,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::{
    net::TcpListener,
    time::{self, MissedTickBehavior},
};
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    services::ServeDir,
};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;

use crate::{
    app_state,
    auth::session::SessionService,
    db::bootstrap,
    middleware::{
        auth::auth_middleware,
        csrf::{self, CsrfState},
        rate_limit::{self, RateLimitState},
        request_context::{self, RequestIdState},
        security::{self, SecurityHeadersState},
    },
    routes,
    services::{
        assistant_service::AssistantService,
        sse_persistence::{SsePersistence, SsePersistenceStore},
    },
    tracer,
};
use axum::http::{HeaderValue, StatusCode, header};
use chrono::{Duration as ChronoDuration, Utc};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
const SSE_RETENTION_SCAN_LIMIT: i64 = 128;
const SSE_RETENTION_INTERVAL_SECS: u64 = 300;
const RATE_LIMIT_REFRESH_INTERVAL_SECS: u64 = 60;

pub fn metrics_handle() -> PrometheusHandle {
    PROMETHEUS_HANDLE
        .get_or_init(|| {
            PrometheusBuilder::new()
                .install_recorder()
                .expect("failed to install Prometheus recorder")
        })
        .clone()
}

async fn metrics_endpoint() -> impl IntoResponse {
    let handle = metrics_handle();
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; version=0.0.4"),
        )],
        handle.render(),
    )
}

async fn prune_sse_once(
    pool: &sqlx::PgPool,
    retention_seconds: i32,
    prune_batch: i32,
    hard_limit: Option<i32>,
) -> Result<(), sqlx::Error> {
    if retention_seconds <= 0 {
        return Ok(());
    }

    let cutoff = Utc::now() - ChronoDuration::seconds(retention_seconds as i64);
    let conversations = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT DISTINCT conversation_id
         FROM rustygpt.sse_event_log
         WHERE created_at < $1
         LIMIT $2",
    )
    .bind(cutoff)
    .bind(SSE_RETENTION_SCAN_LIMIT)
    .fetch_all(pool)
    .await?;

    for conversation_id in conversations {
        sqlx::query("CALL rustygpt.sp_prune_sse_events($1, $2, $3, $4)")
            .bind(conversation_id)
            .bind(retention_seconds)
            .bind(prune_batch)
            .bind(hard_limit)
            .execute(pool)
            .await?;
    }

    Ok(())
}

fn spawn_sse_retention_task(
    pool: sqlx::PgPool,
    config: SsePersistenceConfig,
) -> tokio::task::JoinHandle<()> {
    let retention_seconds =
        (u64::from(config.retention_hours).saturating_mul(3600)).min(i32::MAX as u64) as i32;
    let prune_batch = config.prune_batch_size.max(1).min(i32::MAX as usize) as i32;
    let hard_limit = if config.max_events_per_user > 0 {
        Some(config.max_events_per_user.min(i32::MAX as usize) as i32)
    } else {
        None
    };

    tokio::spawn(async move {
        let mut ticker = time::interval(Duration::from_secs(SSE_RETENTION_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;
            if let Err(err) =
                prune_sse_once(&pool, retention_seconds, prune_batch, hard_limit).await
            {
                warn!(error = %err, "SSE retention sweep failed");
            }
        }
    })
}

/// Initializes the tracing subscriber for logging using the provided configuration.
pub fn initialize_tracing(config: &Config) -> String {
    if matches!(config.logging.format, LogFormat::Json) {
        let env_filter = build_env_filter(config);
        let _ = fmt::fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .with_level(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .json()
            .with_ansi(false)
            .try_init();
    } else {
        let env_filter = build_env_filter(config);
        let _ = fmt::fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .with_level(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_ansi(true)
            .try_init();
    }

    config.logging.level.clone()
}

fn build_env_filter(config: &Config) -> EnvFilter {
    let default_level = config
        .logging
        .level
        .parse::<LevelFilter>()
        .unwrap_or(LevelFilter::INFO);

    EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::builder()
            .with_default_directive(default_level.into())
            .from_env_lossy()
    })
}

/// Creates a database connection pool from the given database URL.
///
/// # Arguments
/// * `db` - Database configuration settings.
///
/// # Returns
/// Returns a configured [`sqlx::PgPool`] or an error if connection fails.
///
/// # Errors
/// Returns an error if the database connection pool cannot be created.
pub async fn create_database_pool(db: &DatabaseConfig) -> Result<sqlx::PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(db.max_connections)
        .connect(&db.url)
        .await?;
    metrics::gauge!("db_pool_max_connections").set(db.max_connections as f64);
    metrics::gauge!("db_statement_timeout_ms").set(db.statement_timeout_ms as f64);
    Ok(pool)
}

/// Creates the application state with the given database pool.
///
/// # Arguments
/// * `pool` - Optional database connection pool.
///
/// # Returns
/// Returns an [`Arc<AppState>`] for sharing across the application.
pub fn create_app_state(
    pool: Option<sqlx::PgPool>,
    assistant: Option<Arc<AssistantService>>,
    sse_store: Option<Arc<dyn SsePersistence>>,
    sessions: Option<Arc<SessionService>>,
    rate_limits: Option<Arc<RateLimitState>>,
) -> Arc<AppState> {
    Arc::new(AppState {
        pool,
        assistant,
        sse_store,
        sessions,
        rate_limits,
    })
}

/// Creates the CORS layer for the application.
///
/// # Returns
/// Returns a configured [`CorsLayer`] allowing any origin.
pub fn create_cors_layer(config: &Config) -> CorsLayer {
    use http::Method;

    let methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ];

    let mut cors = CorsLayer::new()
        .allow_methods(AllowMethods::list(methods))
        .allow_headers(AllowHeaders::any())
        .allow_credentials(config.server.cors.allow_credentials)
        .max_age(Duration::from_secs(config.server.cors.max_age_seconds));

    if config.server.cors.allowed_origins.is_empty() {
        cors = cors.allow_origin(AllowOrigin::any());
    } else {
        let origins = config
            .server
            .cors
            .allowed_origins
            .iter()
            .filter_map(|origin| http::HeaderValue::from_str(origin).ok())
            .collect::<Vec<_>>();
        cors = cors.allow_origin(AllowOrigin::list(origins));
    }

    cors
}

/// Creates the API router with all route modules.
///
/// # Returns
/// Returns a configured [`Router`] with all API routes.
pub fn create_api_router(config: Arc<Config>) -> Router<Arc<AppState>> {
    let mut router = Router::new().merge(routes::setup::create_router_setup());

    if config.features.auth_v1 {
        router = router.merge(routes::auth::create_router_auth());
    }

    if config.features.auth_v1 {
        router = router.merge(
            routes::protected::create_router_protected()
                .route_layer(middleware::from_fn(auth_middleware)),
        );
        if config.rate_limits.admin_api_enabled {
            router = router.merge(routes::admin::create_router_admin());
        }
    }

    router = router.merge(routes::copilot::create_router_copilot());

    if config.features.sse_v1 && config.features.auth_v1 {
        router = router.route(
            "/stream/conversations/:conversation_id",
            axum::routing::get(crate::handlers::streaming::conversation_stream)
                .route_layer(middleware::from_fn(auth_middleware)),
        );
    }

    router
}

/// Creates the static file service for serving frontend assets.
///
/// # Arguments
/// * `static_dir` - Path to the frontend build directory.
///
/// # Returns
/// Returns a configured static file service with SPA fallback.
pub fn create_static_service<S>(
    static_dir: std::path::PathBuf,
    spa_index: std::path::PathBuf,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    use axum::routing::get_service;
    use tower_http::services::ServeFile;

    let index_path = spa_index;

    Router::new().fallback_service(
        ServeDir::new(static_dir)
            .append_index_html_on_directories(true)
            .fallback(get_service(ServeFile::new(index_path))),
    )
}

/// Creates the main application router with all middleware and routes.
///
/// # Arguments
/// * `state` - Application state to share across handlers.
/// * `cors` - CORS layer for the application.
/// * `config` - Shared application configuration.
///
/// # Returns
/// Returns the fully configured application [`Router`].
pub fn create_app_router(state: Arc<AppState>, config: Arc<Config>) -> Router {
    let persistence_config = if config.sse.persistence.enabled {
        Some(config.sse.persistence.clone())
    } else {
        None
    };
    let api_router = {
        let stream_hub: SharedStreamHub = Arc::new(StreamHub::new(
            config.sse.channel_capacity,
            state.sse_store.clone(),
            persistence_config,
        ));
        create_api_router(config.clone()).layer(Extension(stream_hub))
    };
    let static_files_service =
        create_static_service(config.web.static_dir.clone(), config.web.spa_index.clone());

    let cors = create_cors_layer(&config);
    let request_id_state = RequestIdState::from_config(&config);
    let rate_limit_state_arc = state
        .rate_limits
        .clone()
        .unwrap_or_else(|| Arc::new(RateLimitState::new(&config, None)));
    let rate_limit_layer_state = rate_limit_state_arc.as_ref().clone();
    let csrf_state = CsrfState::from_config(&config);
    let security_state = SecurityHeadersState::from_config(&config);

    Router::new()
        .layer(Extension(config))
        .layer(cors)
        .layer(axum::middleware::from_fn_with_state(
            security_state,
            security::apply_security_headers,
        ))
        .layer(axum::middleware::from_fn_with_state(
            csrf_state,
            csrf::enforce_csrf,
        ))
        .layer(axum::middleware::from_fn_with_state(
            rate_limit_layer_state,
            rate_limit::enforce_rate_limits,
        ))
        .layer(tracer::create_trace_layer())
        .layer(axum::middleware::from_fn_with_state(
            request_id_state,
            request_context::assign_request_id,
        ))
        .nest("/api", api_router)
        .merge(routes::health::create_health_router())
        .route("/metrics", get(metrics_endpoint))
        .merge(routes::well_known::create_router_well_known())
        .merge(openapi_routes())
        .merge(static_files_service)
        .with_state(state)
}

/// Creates the graceful shutdown signal handler.
///
/// # Returns
/// Returns a future that resolves when a shutdown signal is received.
pub async fn create_shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    info!("Shutting down...");
}

/// Starts the backend server and binds it to the specified port.
///
/// # Arguments
/// * `config` - The fully resolved configuration struct.
///
/// # Errors
/// Returns an error if the server fails to start.
type AnyError = Box<dyn std::error::Error>;

pub async fn run(config: Config) -> Result<(), AnyError> {
    initialize_tracing(&config);
    info!("Starting server...");

    let _ = metrics_handle();
    let config = Arc::new(config);

    let pool = setup_database(&config).await?;

    let assistant = Arc::new(AssistantService::new(config.clone()));
    let sse_store = build_sse_store(&config, &pool);
    let session_service = build_session_service(&config, &pool);
    let rate_limit_state = initialize_rate_limiting(&config, &pool).await;

    let state = create_app_state(
        Some(pool.clone()),
        Some(assistant),
        sse_store.clone(),
        session_service.clone(),
        Some(rate_limit_state.clone()),
    );

    // Create the application router
    let app = create_app_router(state, config.clone());

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {}", addr);

    let shutdown_signal = create_shutdown_signal();

    serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    Ok(())
}

async fn setup_database(config: &Arc<Config>) -> Result<PgPool, AnyError> {
    let pool = create_database_pool(&config.db)
        .await
        .map_err(|err| -> AnyError { Box::new(err) })?;

    bootstrap::ensure_liveness(&pool)
        .await
        .map_err(|err| -> AnyError { Box::new(err) })?;

    bootstrap::run(&pool, &config.db)
        .await
        .map_err(|err| -> AnyError { Box::new(err) })?;

    bootstrap::ensure_readiness(&pool)
        .await
        .map_err(|err| -> AnyError { Box::new(err) })?;

    Ok(pool)
}

fn build_sse_store(config: &Arc<Config>, pool: &PgPool) -> Option<Arc<dyn SsePersistence>> {
    if config.sse.persistence.enabled {
        let persistence_config = config.sse.persistence.clone();
        let _ = spawn_sse_retention_task(pool.clone(), persistence_config.clone());
        Some(Arc::new(SsePersistenceStore::new(
            pool.clone(),
            persistence_config,
        )))
    } else {
        None
    }
}

fn build_session_service(config: &Arc<Config>, pool: &PgPool) -> Option<Arc<SessionService>> {
    config
        .features
        .auth_v1
        .then(|| Arc::new(SessionService::new(pool.clone(), config.clone())))
}

async fn initialize_rate_limiting(config: &Arc<Config>, pool: &PgPool) -> Arc<RateLimitState> {
    let rate_limit_state = Arc::new(RateLimitState::new(config.as_ref(), Some(pool.clone())));
    if let Err(err) = rate_limit_state.reload_from_db().await {
        warn!(error = %err, "failed to load rate limit configuration");
    }
    let _ = rate_limit_state
        .clone()
        .spawn_auto_refresh(Duration::from_secs(RATE_LIMIT_REFRESH_INTERVAL_SECS));
    rate_limit_state
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use shared::config::server::{Config, LogFormat, Profile};
    use std::{
        io::{self, Write},
        sync::{Arc, Mutex},
    };
    use tracing::{Subscriber, info};
    use tracing_subscriber::fmt::{self, MakeWriter};

    #[derive(Clone)]
    struct BufferMakeWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl BufferMakeWriter {
        fn new(buffer: Arc<Mutex<Vec<u8>>>) -> Self {
            Self { buffer }
        }
    }

    struct BufferWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl<'a> MakeWriter<'a> for BufferMakeWriter {
        type Writer = BufferWriter;

        fn make_writer(&'a self) -> Self::Writer {
            BufferWriter {
                buffer: Arc::clone(&self.buffer),
            }
        }
    }

    impl Write for BufferWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn initialize_tracing_returns_configured_level() {
        let config = Config::default_for_profile(Profile::Dev);
        assert_eq!(initialize_tracing(&config), config.logging.level);
    }

    #[test]
    fn json_log_format_produces_json_output() {
        let mut config = Config::default_for_profile(Profile::Dev);
        config.logging.format = LogFormat::Json;

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let make_writer = BufferMakeWriter::new(buffer.clone());

        let subscriber = subscriber_with_writer(&config, make_writer);
        let dispatch = tracing::dispatcher::Dispatch::new(subscriber);
        let default_guard = tracing::dispatcher::set_default(&dispatch);
        info!("log entry");
        drop(default_guard);

        let contents = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        let line = contents
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or_else(|| panic!("no log output captured: {contents}"));
        let value: Value = serde_json::from_str(line).unwrap();
        assert_eq!(value["fields"]["message"], "log entry");
    }

    #[test]
    fn text_log_format_emits_plain_events() {
        let mut config = Config::default_for_profile(Profile::Dev);
        config.logging.format = LogFormat::Text;

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let make_writer = BufferMakeWriter::new(buffer.clone());

        let subscriber = subscriber_with_writer(&config, make_writer);
        let dispatch = tracing::dispatcher::Dispatch::new(subscriber);
        let default_guard = tracing::dispatcher::set_default(&dispatch);
        info!("log entry");
        drop(default_guard);

        let contents = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        let line = contents
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or_else(|| panic!("no log output captured: {contents}"));
        assert!(
            serde_json::from_str::<Value>(line).is_err(),
            "expected plain text log line"
        );
        assert!(line.contains("log entry"));
    }

    fn subscriber_with_writer<W>(config: &Config, writer: W) -> Box<dyn Subscriber + Send + Sync>
    where
        W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
    {
        let builder = fmt::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_target(false)
            .with_level(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_writer(writer);

        if matches!(config.logging.format, LogFormat::Json) {
            Box::new(builder.json().with_ansi(false).finish())
        } else {
            Box::new(builder.with_ansi(true).finish())
        }
    }

    #[tokio::test]
    async fn metrics_endpoint_returns_prometheus_payload() {
        use axum::{
            body::{Body, to_bytes},
            http::Request,
        };
        use tower::ServiceExt;

        let _ = super::metrics_handle();
        let config = Arc::new(Config::default_for_profile(Profile::Test));
        let app_state = Arc::new(AppState::default());
        let app = super::create_app_router(app_state, config);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(status, StatusCode::OK);
        let content_type = headers.get(header::CONTENT_TYPE).unwrap();
        assert_eq!(content_type, "text/plain; version=0.0.4");

        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(
            body.contains("# TYPE"),
            "expected prometheus exposition format body"
        );
    }
}
