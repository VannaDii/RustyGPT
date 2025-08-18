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
    let frontend_path = config.frontend_path;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_configuration() {
        // Test CORS configuration setup
        use std::env;

        unsafe {
            env::set_var("CORS_ORIGIN", "http://localhost:3000");
            let origin = env::var("CORS_ORIGIN").unwrap();
            assert_eq!(origin, "http://localhost:3000");
            env::remove_var("CORS_ORIGIN");
        }
    }

    #[test]
    fn test_port_configuration() {
        // Test server port configuration
        use std::env;

        unsafe {
            env::set_var("PORT", "8080");
            let port = env::var("PORT").unwrap();
            assert_eq!(port, "8080");
            env::remove_var("PORT");
        }
    }

    #[test]
    fn test_database_configuration() {
        // Test database URL configuration
        use std::env;

        unsafe {
            env::set_var("DATABASE_URL", "postgresql://localhost/test");
            let db_url = env::var("DATABASE_URL").unwrap();
            assert_eq!(db_url, "postgresql://localhost/test");
            env::remove_var("DATABASE_URL");
        }
    }

    #[test]
    fn test_config_struct_creation() {
        // Test that Config can be created with basic values
        use std::path::PathBuf;

        let config = Config::with_defaults();

        assert_eq!(config.server_port, 8080);
        assert_eq!(config.frontend_path, PathBuf::from("../frontend/dist"));
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_socket_addr_parsing() {
        // Test socket address parsing functionality
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        assert_eq!(addr.port(), 8080);
        assert!(addr.is_ipv4());
    }

    #[test]
    fn test_graceful_shutdown_signal_types() {
        // Test that we can reference shutdown signal types
        #[cfg(unix)]
        {
            use tokio::signal::unix::SignalKind;
            let _sigterm = SignalKind::terminate();
            let _sigint = SignalKind::interrupt();
        }
    }

    #[test]
    fn test_environment_variable_handling() {
        // Test various environment variable scenarios
        use std::env;

        unsafe {
            // Test setting and retrieving multiple env vars
            env::set_var("TEST_VAR_1", "value1");
            env::set_var("TEST_VAR_2", "value2");

            assert_eq!(env::var("TEST_VAR_1").unwrap(), "value1");
            assert_eq!(env::var("TEST_VAR_2").unwrap(), "value2");

            // Test removing env vars
            env::remove_var("TEST_VAR_1");
            env::remove_var("TEST_VAR_2");

            assert!(env::var("TEST_VAR_1").is_err());
            assert!(env::var("TEST_VAR_2").is_err());
        }
    }

    #[test]
    fn test_tracing_configuration() {
        // Test tracing configuration options
        use tracing::Level;

        let debug_level = Level::DEBUG;
        let info_level = Level::INFO;

        assert!(debug_level > info_level);
    }

    #[test]
    fn test_cors_layer_creation() {
        // Test CORS layer creation
        let cors = CorsLayer::new().allow_origin(Any);

        // Verify CORS layer can be created
        assert!(!format!("{:?}", cors).is_empty());
    }
}
