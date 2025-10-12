use crate::handlers::streaming::{SharedState, SseCoordinator};
use app_state::AppState;
use axum::{Extension, Router, middleware, serve};
use routes::openapi::openapi_routes;
use shared::config::server::{Config, DatabaseConfig};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    services::ServeDir,
};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

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
    routes, tracer,
};

/// Initializes the tracing subscriber for logging.
///
/// # Returns
/// Returns the configured directive as a string for use in testing.
pub fn initialize_tracing() -> String {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env_lossy()
    });

    tracing_subscriber::registry()
        .with(fmt::layer()) // Log to stdout
        .with(env_filter)
        .init();

    "DEBUG".to_string()
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
    PgPoolOptions::new()
        .max_connections(db.max_connections)
        .connect(&db.url)
        .await
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
pub fn create_app_router(state: Arc<AppState>, config: Arc<Config>) -> Router {
    let shared_state: SharedState = Arc::new(SseCoordinator::new(
        config.sse.channel_capacity,
        config.sse.id_prefix.clone(),
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
    initialize_tracing();
    info!("Starting server...");

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
