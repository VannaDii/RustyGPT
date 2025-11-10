#![cfg(not(target_arch = "wasm32"))]

use crate::{
    app_state::AppState,
    handlers::{
        github_auth::*,
        oauth_testable::{github_oauth_callback_with_service, github_oauth_manual_with_service},
    },
    services::oauth_service_trait::test_implementations::{
        MockOAuthServiceFailure, MockOAuthServiceSuccess,
    },
};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use shared::models::oauth::{OAuthCallback, OAuthRequest};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn set_env_var(key: &str, value: &str) {
        unsafe {
            std::env::set_var(key, value);
        }
    }

    fn remove_env_var(key: &str) {
        unsafe {
            std::env::remove_var(key);
        }
    }

    fn set_test_env() {
        set_env_var("GITHUB_CLIENT_ID", "test_client_id");
        set_env_var("GITHUB_REDIRECT_URI", "http://localhost:8080/callback");
        set_env_var(
            "GITHUB_AUTH_URL",
            "https://github.com/login/oauth/authorize",
        );
    }

    fn clean_test_env() {
        remove_env_var("GITHUB_CLIENT_ID");
        remove_env_var("GITHUB_REDIRECT_URI");
        remove_env_var("GITHUB_AUTH_URL");
    }

    #[tokio::test]
    #[serial]
    async fn test_github_oauth_init_with_env_vars() {
        set_test_env();

        let response = github_oauth_init().await;
        let expected_url = "https://github.com/login/oauth/authorize?client_id=test_client_id&redirect_uri=http://localhost:8080/callback&scope=user";

        assert_eq!(response.auth_url, expected_url);

        clean_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_github_oauth_init_without_env_vars() {
        clean_test_env();

        let response = github_oauth_init().await;
        let expected_url = "?client_id=&redirect_uri=&scope=user";

        assert_eq!(response.auth_url, expected_url);
    }

    #[tokio::test]
    async fn test_github_oauth_callback_success() {
        let state = Arc::new(AppState::default());
        let callback = OAuthCallback {
            code: "test_auth_code".to_string(),
            state: None,
        };

        let response = github_oauth_callback(Query(callback), State(state)).await;

        // Since there's no database pool, we expect an error
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_github_oauth_callback_empty_code() {
        let state = Arc::new(AppState::default());
        let callback = OAuthCallback {
            code: String::new(),
            state: None,
        };

        let response = github_oauth_callback(Query(callback), State(state)).await;

        // Empty code should result in error
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_github_oauth_manual_success() {
        let state = Arc::new(AppState::default());
        let request = OAuthRequest {
            auth_code: "test_auth_code".to_string(),
        };

        let response = github_oauth_manual(State(state), Json(request)).await;

        // Since there's no database pool, we expect an error
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_github_auth_routes_exist() {
        let router = github_auth_routes();

        // Test that the router was created successfully
        assert!(!format!("{router:?}").is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_github_oauth_init_response_format() {
        set_env_var("GITHUB_CLIENT_ID", "client123");
        set_env_var("GITHUB_REDIRECT_URI", "http://example.com/callback");
        set_env_var("GITHUB_AUTH_URL", "https://auth.example.com");

        let response = github_oauth_init().await;

        // Verify URL format is correct
        assert!(response.auth_url.contains("client_id=client123"));
        assert!(
            response
                .auth_url
                .contains("redirect_uri=http://example.com/callback")
        );
        assert!(response.auth_url.contains("scope=user"));
        assert!(response.auth_url.starts_with("https://auth.example.com"));

        clean_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_env_var_edge_cases() {
        // Test with special characters in env vars
        set_env_var("GITHUB_CLIENT_ID", "client-123_abc");
        set_env_var(
            "GITHUB_REDIRECT_URI",
            "https://app.com/oauth/callback?param=value",
        );
        set_env_var(
            "GITHUB_AUTH_URL",
            "https://github.com/login/oauth/authorize",
        );

        let response = github_oauth_init().await;

        assert!(response.auth_url.contains("client-123_abc"));
        assert!(
            response
                .auth_url
                .contains("https://app.com/oauth/callback?param=value")
        );

        clean_test_env();
    }

    #[tokio::test]
    async fn test_github_oauth_callback_success_path() {
        let state = Arc::new(AppState::default());
        let callback = OAuthCallback {
            code: "test_auth_code".to_string(),
            state: None,
        };

        let response = github_oauth_callback_with_service(
            Query(callback),
            State(state),
            MockOAuthServiceSuccess,
        )
        .await;

        // Should get a redirect response for success
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
    }

    #[tokio::test]
    async fn test_github_oauth_callback_failure_path() {
        let state = Arc::new(AppState::default());
        let callback = OAuthCallback {
            code: "invalid_code".to_string(),
            state: None,
        };

        let response = github_oauth_callback_with_service(
            Query(callback),
            State(state),
            MockOAuthServiceFailure,
        )
        .await;

        // Should get an error response for failure
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_github_oauth_manual_success_path() {
        let state = Arc::new(AppState::default());
        let request = OAuthRequest {
            auth_code: "test_auth_code".to_string(),
        };

        let response =
            github_oauth_manual_with_service(State(state), Json(request), MockOAuthServiceSuccess)
                .await;

        // Should get a success response with user ID
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_github_oauth_manual_failure_path() {
        let state = Arc::new(AppState::default());
        let request = OAuthRequest {
            auth_code: "invalid_code".to_string(),
        };

        let response =
            github_oauth_manual_with_service(State(state), Json(request), MockOAuthServiceFailure)
                .await;

        // Should get an error response for failure
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
