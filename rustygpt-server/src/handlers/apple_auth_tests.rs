use crate::{app_state::AppState, handlers::apple_auth::*};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use shared::models::oauth::{OAuthCallback, OAuthRequest};
use std::{env, sync::Arc};

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn set_test_env() {
        unsafe {
            env::set_var("APPLE_CLIENT_ID", "test.app.client");
            env::set_var("APPLE_REDIRECT_URI", "https://app.com/oauth/apple/callback");
            env::set_var("APPLE_AUTH_URL", "https://appleid.apple.com/auth/authorize");
        }
    }

    fn clean_test_env() {
        unsafe {
            env::remove_var("APPLE_CLIENT_ID");
            env::remove_var("APPLE_REDIRECT_URI");
            env::remove_var("APPLE_AUTH_URL");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_apple_oauth_init_with_env_vars() {
        set_test_env();

        let response = apple_oauth_init().await;
        let expected_url = "https://appleid.apple.com/auth/authorize?client_id=test.app.client&redirect_uri=https://app.com/oauth/apple/callback&response_type=code&scope=name%20email";

        assert_eq!(response.auth_url, expected_url);

        clean_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_apple_oauth_init_without_env_vars() {
        clean_test_env();

        let response = apple_oauth_init().await;
        let expected_url = "?client_id=&redirect_uri=&response_type=code&scope=name%20email";

        assert_eq!(response.auth_url, expected_url);
    }

    #[tokio::test]
    #[serial]
    async fn test_apple_oauth_init_partial_env_vars() {
        unsafe {
            env::set_var("APPLE_CLIENT_ID", "com.example.app");
            env::remove_var("APPLE_REDIRECT_URI");
            env::set_var("APPLE_AUTH_URL", "https://appleid.apple.com/auth/authorize");
        }

        let response = apple_oauth_init().await;
        let expected_url = "https://appleid.apple.com/auth/authorize?client_id=com.example.app&redirect_uri=&response_type=code&scope=name%20email";

        assert_eq!(response.auth_url, expected_url);

        unsafe {
            env::remove_var("APPLE_CLIENT_ID");
            env::remove_var("APPLE_AUTH_URL");
        }
    }

    #[tokio::test]
    async fn test_apple_oauth_callback_success() {
        let state = Arc::new(AppState::default());
        let callback = OAuthCallback {
            code: "test_apple_auth_code".to_string(),
            state: None,
        };

        let response = apple_oauth_callback(Query(callback), State(state)).await;

        // Since there's no database pool, we expect an error
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_apple_oauth_callback_empty_code() {
        let state = Arc::new(AppState::default());
        let callback = OAuthCallback {
            code: "".to_string(),
            state: None,
        };

        let response = apple_oauth_callback(Query(callback), State(state)).await;

        // Empty code should result in error
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_apple_oauth_manual_success() {
        let state = Arc::new(AppState::default());
        let request = OAuthRequest {
            auth_code: "test_apple_auth_code".to_string(),
        };

        let response = apple_oauth_manual(State(state), Json(request)).await;

        // Since there's no database pool, we expect an error
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_apple_auth_routes_exist() {
        let router = apple_auth_routes();

        // Test that the router was created successfully
        assert!(!format!("{:?}", router).is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_apple_oauth_scope_encoding() {
        unsafe {
            env::set_var("APPLE_CLIENT_ID", "test.client");
            env::set_var("APPLE_REDIRECT_URI", "https://test.com/callback");
            env::set_var("APPLE_AUTH_URL", "https://appleid.apple.com/auth/authorize");
        }

        let response = apple_oauth_init().await;

        // Verify that scope is properly URL encoded for Apple (name email -> name%20email)
        assert!(response.auth_url.contains("scope=name%20email"));
        assert!(!response.auth_url.contains("scope=name email")); // Should not contain unencoded space

        clean_test_env();
    }
}
