use app_state::AppState;
use axum::{
    Router,
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    serve,
};
use http::Request;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::{env, sync::Arc};
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing::{info, instrument, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod app_state;
mod handlers;
mod routes;
mod services;
mod tracer;

#[cfg(test)]
mod main_test;

// Middleware to check if a user is authenticated
#[instrument(skip(next))]
pub async fn auth_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // In a real application, you would check for a valid JWT token or session
    // For now, we'll just pass through all requests
    info!(
        "Auth middleware processing request to: {}",
        req.uri().path()
    );
    Ok(next.run(req).await)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(fmt::layer()) // Log to stdout
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy()
        }))
        .init();

    info!("Starting server with verbose logging...");

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
    let static_files_service = ServeDir::new(frontend_path).append_index_html_on_directories(true);

    // Register tracing and streaming routes BEFORE the fallback route.
    let app = Router::new()
        .layer(cors)
        .layer(tracer::create_trace_layer())
        .nest("/api", api_router) // API routes
        .fallback_service(static_files_service)
        .with_state(state);

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
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
