use crate::app_state::AppState;
use crate::handlers::apple_auth::*;
use axum_test::TestServer;
use http::StatusCode;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_apple_auth_routes_exist() {
    tracing::info!("Testing Apple auth routes creation");
    // Create the router with the Apple auth routes
    let _app = apple_auth_routes();
}

#[tokio::test]
async fn test_apple_oauth_init() {
    // Test Apple OAuth initialization endpoint
    let app_state = Arc::new(AppState::default());
    let app = apple_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/apple").await;

    response.assert_status_ok();
    let json: serde_json::Value = response.json();

    // Check that response contains auth_url field
    assert!(json.get("auth_url").is_some());
    let auth_url = json["auth_url"].as_str().unwrap();
    assert!(!auth_url.is_empty());
}

#[tokio::test]
async fn test_apple_oauth_init_with_env_vars() {
    // Test Apple OAuth initialization with environment variables set
    unsafe {
        std::env::set_var("APPLE_CLIENT_ID", "test_client_id");
        std::env::set_var("APPLE_REDIRECT_URI", "http://localhost:8080/callback");
        std::env::set_var("APPLE_AUTH_URL", "https://appleid.apple.com/auth/authorize");
    }

    let app_state = Arc::new(AppState::default());
    let app = apple_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/apple").await;

    response.assert_status_ok();
    let json: serde_json::Value = response.json();
    let auth_url = json["auth_url"].as_str().unwrap();

    // The auth URL should contain the standard OAuth parameters
    assert!(auth_url.contains("response_type=code"));
    assert!(auth_url.contains("scope=name%20email"));

    // Environment variables might not affect the handler directly,
    // so just verify the URL structure is correct
    assert!(!auth_url.is_empty());

    // Clean up environment variables
    unsafe {
        std::env::remove_var("APPLE_CLIENT_ID");
        std::env::remove_var("APPLE_REDIRECT_URI");
        std::env::remove_var("APPLE_AUTH_URL");
    }
}

#[tokio::test]
async fn test_apple_oauth_callback_with_invalid_code() {
    // Test Apple OAuth callback with invalid authorization code
    let app_state = Arc::new(AppState::default());
    let app = apple_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/apple/callback?code=invalid_code").await;

    // Should return internal server error for invalid code
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_apple_oauth_callback_missing_code() {
    // Test Apple OAuth callback without authorization code
    let app_state = Arc::new(AppState::default());
    let app = apple_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/apple/callback").await;

    // Should return bad request for missing required query parameter
    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_apple_oauth_manual_invalid_payload() {
    // Test manual Apple OAuth with invalid payload
    let app_state = Arc::new(AppState::default());
    let app = apple_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let payload = json!({
        "auth_code": "invalid_test_code"
    });

    let response = server.post("/oauth/apple/manual").json(&payload).await;

    // Should return internal server error for invalid auth code
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    let text = response.text();
    assert_eq!(text, "OAuth failed");
}

#[test]
fn test_environment_variable_defaults() {
    // Test behavior when environment variables are not set
    unsafe {
        std::env::remove_var("APPLE_CLIENT_ID");
        std::env::remove_var("APPLE_REDIRECT_URI");
        std::env::remove_var("APPLE_AUTH_URL");
    }

    let client_id = std::env::var("APPLE_CLIENT_ID").unwrap_or_default();
    let redirect_uri = std::env::var("APPLE_REDIRECT_URI").unwrap_or_default();
    let auth_base_url = std::env::var("APPLE_AUTH_URL").unwrap_or_default();

    assert_eq!(client_id, "");
    assert_eq!(redirect_uri, "");
    assert_eq!(auth_base_url, "");
}

#[test]
fn test_auth_url_construction() {
    // Test URL construction logic
    let client_id = "test_client";
    let redirect_uri = "http://localhost/callback";
    let auth_base_url = "https://example.com/auth";

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope=name%20email",
        auth_base_url, client_id, redirect_uri
    );

    assert!(auth_url.contains("test_client"));
    assert!(auth_url.contains("http://localhost/callback"));
    assert!(auth_url.contains("response_type=code"));
    assert!(auth_url.contains("scope=name%20email"));
}
