use app_state::AppState;
use axum::{
    Router,
    middleware::{self},
    response::{IntoResponse, Redirect},
    serve,
};
use routes::openapi::openapi_routes;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::{env, sync::Arc};
use tokio::net::TcpListener;
use tower::service_fn;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;

use crate::{app_state, middleware::auth::auth_middleware, openapi, routes, tracer};

/// Starts the backend server and binds it to the specified port.
///
/// # Arguments
/// * `port` - The port number to bind the server to.
///
/// # Errors
/// Returns an error if the server fails to start.
///
/// # Examples
/// ```
/// commands::server::run(8080)?;
/// ```
#[tokio::main]
pub async fn run(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(fmt::layer()) // Log to stdout
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy()
        }))
        .init();

    info!("Starting server...");

    // Write OpenAPI spec to disk
    let openapi = openapi::ApiDoc::openapi();
    std::fs::write("../docs/rustygpt.yaml", openapi.to_yaml()?)?;
    info!("OpenAPI spec written to docs/rustygpt.yaml");

    // Set up database connection pool
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/rusty_gpt".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");

    // Create application state
    let state = Arc::new(AppState { pool });

    // Set up CORS - TODO: Configure this
    let cors = CorsLayer::new().allow_origin(Any);

    // Build API router
    let api_router = Router::new()
        .merge(routes::setup::create_router_setup())
        .merge(routes::auth::create_router_auth())
        .merge(
            routes::protected::create_router_protected()
                .route_layer(middleware::from_fn(auth_middleware)),
        );

    // Set up static file serving for the app
    let frontend_path = PathBuf::from("../frontend/dist");
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
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
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
