use crate::handlers::streaming::{SharedState, SseCoordinator};
use app_state::AppState;
use axum::{Extension, Router, middleware, response::IntoResponse, routing::get, serve};
use routes::openapi::openapi_routes;
use shared::config::server::{Config, DatabaseConfig, LogFormat};
use sqlx::postgres::PgPoolOptions;
use std::{
    net::SocketAddr,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::net::TcpListener;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    services::ServeDir,
};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;

use crate::{
    app_state,
    db::bootstrap,
    middleware::{
        auth::auth_middleware,
        csrf::{self, CsrfState},
        rate_limit::{self, RateLimitState},
        request_context::{self, RequestIdState},
        security::{self, SecurityHeadersState},
    },
    routes,
    services::sse_persistence::{self, SsePersistenceStore},
    tracer,
};
use axum::http::{HeaderValue, StatusCode, header};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

pub(crate) fn metrics_handle() -> PrometheusHandle {
    PROMETHEUS_HANDLE
        .get_or_init(|| {
            PrometheusBuilder::new()
                .install_recorder()
                .expect("failed to install Prometheus recorder")
        })
        .clone()
}

async fn metrics_endpoint(Extension(handle): Extension<PrometheusHandle>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; version=0.0.4"),
        )],
        handle.render(),
    )
}

/// Initializes the tracing subscriber for logging using the provided configuration.
pub fn initialize_tracing(config: &Config) -> String {
    let env_filter = build_env_filter(config);

    let fmt_builder = fmt::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false);

    if matches!(config.logging.format, LogFormat::Json) {
        fmt_builder.json().with_ansi(false).init();
    } else {
        fmt_builder.with_ansi(true).init();
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
pub fn create_app_state(pool: Option<sqlx::PgPool>) -> Arc<AppState> {
    Arc::new(AppState { pool })
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
    }

    router = router.merge(routes::copilot::create_router_copilot());

    if config.features.sse_v1 && config.features.auth_v1 {
        router = router.route(
            "/stream",
            axum::routing::get(crate::handlers::streaming::sse_handler)
                .route_layer(middleware::from_fn(auth_middleware)),
        );
    }

    router.route(
        "/messages",
        axum::routing::post(crate::handlers::conversation::send_message_to_default_conversation),
    )
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
pub fn create_app_router(
    state: Arc<AppState>,
    config: Arc<Config>,
    metrics_handle: PrometheusHandle,
) -> Router {
    let persistence_cfg = if config.sse.persistence.enabled {
        Some(config.sse.persistence.clone())
    } else {
        None
    };

    let persistence_store: Option<Arc<dyn sse_persistence::SsePersistence>> =
        if config.sse.persistence.enabled {
            state.pool.as_ref().map(|pool| {
                Arc::new(SsePersistenceStore::new(
                    pool.clone(),
                    config.sse.persistence.clone(),
                )) as Arc<dyn sse_persistence::SsePersistence>
            })
        } else {
            None
        };

    let shared_state: SharedState = Arc::new(SseCoordinator::new(
        config.sse.channel_capacity,
        config.sse.id_prefix.clone(),
        persistence_store,
        persistence_cfg,
        config.sse.backpressure.clone(),
    ));

    let api_router = create_api_router(config.clone()).layer(Extension(shared_state));
    let static_files_service =
        create_static_service(config.web.static_dir.clone(), config.web.spa_index.clone());

    let cors = create_cors_layer(&config);
    let request_id_state = RequestIdState::from_config(&config);
    let rate_limit_state = RateLimitState::from_config(&config);
    let csrf_state = CsrfState::from_config(&config);
    let security_state = SecurityHeadersState::from_config(&config);

    Router::new()
        .layer(Extension(config.clone()))
        .layer(Extension(metrics_handle))
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
            rate_limit_state,
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
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing(&config);
    info!("Starting server...");

    let metrics_handle = metrics_handle();
    let config = Arc::new(config);

    // Set up database connection pool
    let pool = create_database_pool(&config.db)
        .await
        .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })?;

    // Run database bootstrap and health checks
    bootstrap::ensure_liveness(&pool)
        .await
        .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })?;

    bootstrap::run(&pool, &config.db)
        .await
        .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })?;

    bootstrap::ensure_readiness(&pool)
        .await
        .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })?;

    // Create application state
    let state = create_app_state(Some(pool));

    // Create the application router
    let app = create_app_router(state, config.clone(), metrics_handle.clone());

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

        tracing::dispatcher::with_default(&dispatch, || {
            info!(event = "json_test", "log entry");
        });

        let contents = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        let line = contents
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap();
        let value: Value = serde_json::from_str(line).unwrap();
        assert_eq!(value["fields"]["message"], "log entry");
        assert_eq!(value["fields"]["event"], "json_test");
    }

    #[test]
    fn text_log_format_emits_plain_events() {
        let mut config = Config::default_for_profile(Profile::Dev);
        config.logging.format = LogFormat::Text;

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let make_writer = BufferMakeWriter::new(buffer.clone());

        let subscriber = subscriber_with_writer(&config, make_writer);
        let dispatch = tracing::dispatcher::Dispatch::new(subscriber);

        tracing::dispatcher::with_default(&dispatch, || {
            info!(event = "text_test", "log entry");
        });

        let contents = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        let line = contents
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap();
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
        let env_filter = super::build_env_filter(config);
        let builder = fmt::fmt()
            .with_env_filter(env_filter)
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
            http::{Request, StatusCode, header},
        };
        use tower::ServiceExt;

        let _ = super::metrics_handle();
        let config = Arc::new(Config::default_for_profile(Profile::Test));
        let app_state = Arc::new(AppState::default());
        let metrics_handle = super::metrics_handle();

        let app = super::create_app_router(app_state, config, metrics_handle.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
        assert_eq!(content_type, "text/plain; version=0.0.4");

        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(
            body.contains("# HELP"),
            "expected prometheus exposition format body"
        );
    }
}
