use app_state::AppState;
use axum::{
    Router,
    middleware::{self},
    response::{IntoResponse, Redirect},
    serve,
};
use routes::openapi::openapi_routes;
use shared::config::server::Config;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::service_fn;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{app_state, middleware::auth::auth_middleware, routes, tracer};

/// Starts the backend server and binds it to the specified port.
///
/// # Arguments
/// * `config` - The fully resolved configuration struct.
///
/// # Errors
/// Returns an error if the server fails to start.
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(fmt::layer()) // Log to stdout
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy()
        }))
        .init();

    info!("Starting server...");

    // Set up database connection pool
    let database_url = config.database_url;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");

    // Create application state
    let state = Arc::new(AppState { pool: Some(pool) });

    // Set up CORS - TODO: Configure this
    let cors = CorsLayer::new().allow_origin(Any);

    // Build API router
    let api_router = Router::new()
        .merge(routes::setup::create_router_setup())
        .merge(routes::auth::create_router_auth())
        .merge(
            routes::protected::create_router_protected()
                .route_layer(middleware::from_fn(auth_middleware)),
        )
        .merge(routes::copilot::create_router_copilot()); // Added copilot routes

    // Set up static file serving for the app
    let frontend_path = PathBuf::from(config.frontend_path);
    let fallback_service = service_fn(|_req| async {
        Ok::<_, std::convert::Infallible>(Redirect::to("/").into_response())
    });
    let static_files_service = ServeDir::new(frontend_path)
        .append_index_html_on_directories(true)
        .fallback(fallback_service);

    let app = Router::new()
        .layer(cors)
        .layer(tracer::create_trace_layer())
        .nest("/api", api_router)
        .merge(openapi_routes()) // OpenAPI routes defined in routes/openapi.rs
        .fallback_service(static_files_service)
        .with_state(Arc::clone(&state));

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {}", addr);

    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Shutting down...");
    };

    serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    Ok(())
}
