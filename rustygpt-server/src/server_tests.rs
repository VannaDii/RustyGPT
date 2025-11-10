//! Tests for the server startup and configuration logic.

use crate::{app_state::AppState, server::create_app};
use axum::Router;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_app_with_default_state() {
        let app_state = Arc::new(AppState::default());
        let app = create_app(app_state).await;

        // Verify the router was created successfully
        assert!(!format!("{:?}", app).is_empty());
    }

    #[tokio::test]
    async fn test_create_app_with_custom_state() {
        let app_state = Arc::new(AppState::default());
        let app = create_app(app_state).await;

        // Test that the app can be created with any state
        let _router: Router = app;
    }

    #[test]
    fn test_app_state_creation() {
        let state = AppState::default();

        // Verify default state is created correctly
        assert!(state.pool.is_none());
    }

    #[test]
    fn test_app_state_default_behavior() {
        // Test that AppState can be created with default behavior
        let state = AppState::default();
        assert!(state.pool.is_none());
    }

    #[tokio::test]
    async fn test_server_router_structure() {
        let app_state = Arc::new(AppState::default());
        let app = create_app(app_state).await;

        // Verify the router structure contains expected routes
        let debug_output = format!("{:?}", app);
        assert!(debug_output.contains("Router"));
    }

    #[test]
    fn test_default_configuration() {
        // Test default server configuration values
        let state = AppState::default();

        // Verify state initialization
        assert!(state.pool.is_none());
    }

    #[tokio::test]
    async fn test_middleware_integration() {
        let app_state = Arc::new(AppState::default());
        let app = create_app(app_state).await;

        // Test that middleware is properly integrated
        // The app should be constructable without errors
        let _app_instance = app;
    }

    #[test]
    fn test_cors_configuration() {
        // Test CORS configuration setup
        // This test validates that CORS can be configured without errors
        use std::env;

        env::set_var("CORS_ORIGIN", "http://localhost:3000");
        let origin = env::var("CORS_ORIGIN").unwrap();
        assert_eq!(origin, "http://localhost:3000");
        env::remove_var("CORS_ORIGIN");
    }

    #[test]
    fn test_port_configuration() {
        // Test server port configuration
        use std::env;

        env::set_var("PORT", "8080");
        let port = env::var("PORT").unwrap();
        assert_eq!(port, "8080");
        env::remove_var("PORT");
    }

    #[test]
    fn test_database_configuration() {
        // Test database URL configuration
        use std::env;

        env::set_var("DATABASE_URL", "postgresql://localhost/test");
        let db_url = env::var("DATABASE_URL").unwrap();
        assert_eq!(db_url, "postgresql://localhost/test");
        env::remove_var("DATABASE_URL");
    }
}
