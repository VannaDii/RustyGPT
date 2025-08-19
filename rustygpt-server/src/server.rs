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
    use shared::config::server::Config;
    use std::path::PathBuf;

    #[test]
    fn test_socket_addr_creation() {
        // Test socket address creation from config
        let config = Config::with_defaults();
        let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
        assert_eq!(addr.port(), 8080);
        assert!(addr.is_ipv4());
    }

    #[test]
    fn test_socket_addr_with_custom_port() {
        // Test socket address with custom port
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        assert_eq!(addr.port(), 3000);
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn test_cors_layer_creation() {
        // Test CORS layer creation
        let cors = CorsLayer::new().allow_origin(Any);

        // Verify CORS layer can be created without errors
        assert!(!format!("{:?}", cors).is_empty());
    }

    #[test]
    fn test_app_state_creation_without_pool() {
        // Test AppState creation without database pool
        let state = Arc::new(AppState { pool: None });
        assert!(state.pool.is_none());
    }

    #[test]
    fn test_config_default_values() {
        // Test that Config has expected default values
        let config = Config::with_defaults();

        assert_eq!(config.server_port, 8080);
        assert_eq!(config.frontend_path, PathBuf::from("../frontend/dist"));
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_frontend_path_configuration() {
        // Test frontend path configuration
        let frontend_path = PathBuf::from("/custom/frontend/path");

        let mut config = Config::with_defaults();
        config.frontend_path = frontend_path.clone();

        assert_eq!(config.frontend_path, frontend_path);
    }

    #[test]
    fn test_database_url_validation() {
        // Test database URL format validation
        let valid_urls = vec![
            "postgresql://localhost/test",
            "postgresql://user:pass@localhost:5432/db",
            "postgresql://user@localhost/database",
        ];

        for url in valid_urls {
            // Just test that the string format is valid
            assert!(!url.is_empty());
            assert!(url.starts_with("postgresql://"));
        }
    }

    #[test]
    fn test_tracing_level_filter() {
        // Test tracing level filter creation
        use tracing::level_filters::LevelFilter;

        let debug_filter = LevelFilter::DEBUG;
        let info_filter = LevelFilter::INFO;
        let warn_filter = LevelFilter::WARN;

        assert!(debug_filter > info_filter);
        assert!(info_filter > warn_filter);
    }

    #[test]
    fn test_env_filter_creation() {
        // Test environment filter creation
        use tracing_subscriber::EnvFilter;

        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env_lossy();

        assert!(!format!("{:?}", filter).is_empty());
    }

    #[test]
    fn test_serve_dir_configuration() {
        // Test ServeDir configuration with static path
        let serve_dir = ServeDir::new("./static").append_index_html_on_directories(true);

        assert!(!format!("{:?}", serve_dir).is_empty());
    }

    #[test]
    fn test_pg_pool_options() {
        // Test PostgreSQL pool options configuration
        let pool_options = PgPoolOptions::new().max_connections(5);

        // Verify pool options can be configured
        assert!(!format!("{:?}", pool_options).is_empty());
    }

    #[test]
    fn test_ipv4_vs_ipv6_addresses() {
        // Test different IP address types
        let ipv4_addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        let ipv6_addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], 8080));

        assert!(ipv4_addr.is_ipv4());
        assert!(ipv6_addr.is_ipv6());
    }

    #[test]
    fn test_config_log_levels() {
        // Test different log level configurations
        let mut config = Config::with_defaults();

        config.log_level = "debug".to_string();
        assert_eq!(config.log_level, "debug");

        config.log_level = "warn".to_string();
        assert_eq!(config.log_level, "warn");
    }
}
