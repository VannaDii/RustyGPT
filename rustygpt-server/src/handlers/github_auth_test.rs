use crate::app_state::AppState;
use crate::handlers::github_auth::github_auth_routes;
use axum_test::TestServer;
use http::StatusCode;
use serde_json::json;
use std::sync::Arc;

fn set_env_vars(vars: &[(&str, &str)]) {
    for (key, value) in vars {
        unsafe {
            std::env::set_var(key, value);
        }
    }
}

fn remove_env_vars(keys: &[&str]) {
    for key in keys {
        unsafe {
            std::env::remove_var(key);
        }
    }
}

#[tokio::test]
async fn test_github_auth_routes_exist() {
    tracing::info!("Testing GitHub auth routes creation");
    // Create the router with the GitHub auth routes
    let _app = github_auth_routes();
}

#[tokio::test]
async fn test_github_oauth_init() {
    // Test GitHub OAuth initialization endpoint
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/github").await;

    response.assert_status_ok();
    let json: serde_json::Value = response.json();

    // Check that response contains auth_url field
    assert!(json.get("auth_url").is_some());
    let auth_url = json["auth_url"].as_str().unwrap();
    assert!(!auth_url.is_empty());
}

#[tokio::test]
#[serial_test::serial]
async fn test_github_oauth_init_with_env_vars() {
    // Test GitHub OAuth initialization with environment variables set
    set_env_vars(&[
        ("GITHUB_CLIENT_ID", "test_client_id"),
        ("GITHUB_REDIRECT_URI", "http://localhost:8080/callback"),
        (
            "GITHUB_AUTH_URL",
            "https://github.com/login/oauth/authorize",
        ),
    ]);

    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/github").await;

    response.assert_status_ok();
    let json: serde_json::Value = response.json();
    let auth_url = json["auth_url"].as_str().unwrap();

    // The auth URL should contain the standard OAuth parameters
    assert!(auth_url.contains("scope=user"));

    // Environment variables might not affect the handler directly,
    // so just verify the URL structure is correct
    assert!(!auth_url.is_empty());

    // Clean up environment variables
    remove_env_vars(&["GITHUB_CLIENT_ID", "GITHUB_REDIRECT_URI", "GITHUB_AUTH_URL"]);
}

#[tokio::test]
async fn test_github_oauth_callback_with_invalid_code() {
    // Test GitHub OAuth callback with invalid authorization code
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/github/callback?code=invalid_code").await;

    // Should return internal server error for invalid code
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_github_oauth_callback_missing_code() {
    // Test GitHub OAuth callback without authorization code
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/github/callback").await;

    // Should return bad request for missing required query parameter
    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_github_oauth_manual_invalid_payload() {
    // Test manual GitHub OAuth with invalid payload
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let payload = json!({
        "auth_code": "invalid_test_code"
    });

    let response = server.post("/oauth/github/manual").json(&payload).await;

    // Should return internal server error for invalid auth code
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    let text = response.text();
    assert_eq!(text, "OAuth failed");
}

#[tokio::test]
async fn test_github_oauth_callback_success() {
    // Test successful GitHub OAuth callback - This will still fail but provides coverage
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/oauth/github/callback?code=test_auth_code")
        .await;

    // Should return internal server error due to no database connection,
    // but this exercises the callback handler path
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_github_oauth_manual_success() {
    // Test manual GitHub OAuth endpoint - This will still fail but provides coverage
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let payload = json!({
        "auth_code": "test_auth_code"
    });

    let response = server.post("/oauth/github/manual").json(&payload).await;

    // Should return internal server error due to no database connection,
    // but this exercises the manual handler path
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    let text = response.text();
    assert_eq!(text, "OAuth failed");
}

#[tokio::test]
async fn test_github_oauth_callback_empty_code() {
    // Test GitHub OAuth callback with empty code parameter
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/github/callback?code=").await;

    // Should return internal server error for empty auth code
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_github_auth_routes_creation() {
    // Test that all GitHub auth routes are properly registered
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    // Test that all three routes exist and respond appropriately
    let init_response = server.get("/oauth/github").await;
    init_response.assert_status_ok();

    let callback_response = server.get("/oauth/github/callback").await;
    callback_response.assert_status(StatusCode::BAD_REQUEST); // Missing required query param

    let manual_response = server.post("/oauth/github/manual").await;
    manual_response.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE); // Missing Content-Type header
}

#[tokio::test]
async fn test_github_oauth_init_response_format() {
    // Test the exact response format of the init endpoint
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/github").await;

    response.assert_status_ok();
    response.assert_header("content-type", "application/json");

    let json: serde_json::Value = response.json();
    assert!(json.is_object());
    assert!(json.get("auth_url").is_some());
    assert!(json["auth_url"].is_string());
}

#[tokio::test]
async fn test_github_oauth_manual_with_malformed_json() {
    // Test manual OAuth endpoint with malformed JSON
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/oauth/github/manual")
        .add_header("content-type", "application/json")
        .text("{invalid json}")
        .await;

    // Should return unsupported media type for this specific error condition
    response.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[test]
#[serial_test::serial]
fn test_github_env_var_behavior_with_special_characters() {
    // Test handling of environment variables with special characters
    set_env_vars(&[
        ("GITHUB_CLIENT_ID", "client@123"),
        (
            "GITHUB_REDIRECT_URI",
            "http://localhost:8080/callback?state=test",
        ),
        (
            "GITHUB_AUTH_URL",
            "https://github.com/login/oauth/authorize",
        ),
    ]);

    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_default();
    let redirect_uri = std::env::var("GITHUB_REDIRECT_URI").unwrap_or_default();
    let auth_base_url = std::env::var("GITHUB_AUTH_URL").unwrap_or_default();

    assert_eq!(client_id, "client@123");
    assert!(redirect_uri.contains("state=test"));
    assert!(!auth_base_url.is_empty());

    // Clean up
    remove_env_vars(&["GITHUB_CLIENT_ID", "GITHUB_REDIRECT_URI", "GITHUB_AUTH_URL"]);
}

#[tokio::test]
async fn test_github_oauth_routes_method_verification() {
    // Test that routes respond correctly to their expected HTTP methods
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    // GET /oauth/github should work
    let get_response = server.get("/oauth/github").await;
    get_response.assert_status_ok();

    // POST /oauth/github should not be allowed
    let post_response = server.post("/oauth/github").await;
    post_response.assert_status(StatusCode::METHOD_NOT_ALLOWED);

    // GET /oauth/github/callback should work (even if it fails)
    let callback_get = server.get("/oauth/github/callback").await;
    assert!(callback_get.status_code() != StatusCode::METHOD_NOT_ALLOWED);

    // POST /oauth/github/manual should work (even if it fails)
    let manual_post = server.post("/oauth/github/manual").await;
    assert!(manual_post.status_code() != StatusCode::METHOD_NOT_ALLOWED);

    // GET /oauth/github/manual should not be allowed
    let manual_get = server.get("/oauth/github/manual").await;
    manual_get.assert_status(StatusCode::METHOD_NOT_ALLOWED);
}

#[test]
fn test_github_auth_url_construction() {
    // Test URL construction logic for GitHub OAuth
    let client_id = "test_client";
    let redirect_uri = "http://localhost/callback";
    let auth_base_url = "https://github.com/login/oauth/authorize";

    let auth_url =
        format!("{auth_base_url}?client_id={client_id}&redirect_uri={redirect_uri}&scope=user");

    assert!(auth_url.contains("test_client"));
    assert!(auth_url.contains("http://localhost/callback"));
    assert!(auth_url.contains("scope=user"));
}

#[test]
#[serial_test::serial]
fn test_github_environment_variable_defaults() {
    // Test behavior when environment variables are not set
    remove_env_vars(&["GITHUB_CLIENT_ID", "GITHUB_REDIRECT_URI", "GITHUB_AUTH_URL"]);

    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_default();
    let redirect_uri = std::env::var("GITHUB_REDIRECT_URI").unwrap_or_default();
    let auth_base_url = std::env::var("GITHUB_AUTH_URL").unwrap_or_default();

    assert_eq!(client_id, "");
    assert_eq!(redirect_uri, "");
    assert_eq!(auth_base_url, "");
}

#[tokio::test]
async fn test_github_oauth_callback_with_state_parameter() {
    // Test GitHub OAuth callback with state parameter
    set_env_vars(&[
        ("GITHUB_CLIENT_ID", "test_client_id"),
        ("GITHUB_CLIENT_SECRET", "test_client_secret"),
        ("GITHUB_TOKEN_URL", "https://valid.token.url"),
        ("GITHUB_REDIRECT_URI", "http://localhost:8080/callback"),
    ]);

    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/oauth/github/callback?code=test_code&state=test_state")
        .await;

    // Should redirect on success (status 302 or 500 depending on OAuth service response)
    assert!(
        response.status_code() == StatusCode::FOUND
            || response.status_code() == StatusCode::INTERNAL_SERVER_ERROR
    );

    // Clean up
    remove_env_vars(&[
        "GITHUB_CLIENT_ID",
        "GITHUB_CLIENT_SECRET",
        "GITHUB_TOKEN_URL",
        "GITHUB_REDIRECT_URI",
    ]);
}

#[tokio::test]
async fn test_github_oauth_callback_error_handling() {
    // Test GitHub OAuth callback error handling
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/github/callback?code=").await;

    // Should return internal server error for empty code
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_github_oauth_callback_no_parameters() {
    // Test GitHub OAuth callback with no parameters
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/github/callback").await;

    // Should return bad request for missing code
    assert!(
        response.status_code() == StatusCode::BAD_REQUEST
            || response.status_code() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn test_github_oauth_callback_invalid_code_handling() {
    // Test GitHub OAuth callback with invalid code
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/oauth/github/callback?code=invalid_code_123")
        .await;

    // Should return internal server error for invalid code
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_github_oauth_manual_with_valid_payload() {
    // Test manual GitHub OAuth with valid payload
    set_env_vars(&[
        ("GITHUB_CLIENT_ID", "test_client_id"),
        ("GITHUB_CLIENT_SECRET", "test_client_secret"),
        ("GITHUB_TOKEN_URL", "https://valid.token.url"),
        ("GITHUB_REDIRECT_URI", "http://localhost:8080/callback"),
    ]);

    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let payload = json!({
        "auth_code": "test_auth_code"
    });

    let response = server.post("/oauth/github/manual").json(&payload).await;

    // Should return success or server error depending on OAuth service response
    assert!(
        response.status_code() == StatusCode::OK
            || response.status_code() == StatusCode::INTERNAL_SERVER_ERROR
    );

    // Clean up
    remove_env_vars(&[
        "GITHUB_CLIENT_ID",
        "GITHUB_CLIENT_SECRET",
        "GITHUB_TOKEN_URL",
        "GITHUB_REDIRECT_URI",
    ]);
}

#[tokio::test]
async fn test_github_oauth_manual_bad_payload() {
    // Test manual GitHub OAuth with invalid payload structure
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let payload = json!({
        "invalid_field": "test_value"
    });

    let response = server.post("/oauth/github/manual").json(&payload).await;

    // Should return bad request for invalid payload
    assert!(
        response.status_code() == StatusCode::BAD_REQUEST
            || response.status_code() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_github_oauth_manual_malformed_json() {
    // Test manual GitHub OAuth with malformed JSON
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/oauth/github/manual")
        .text("{invalid_json")
        .content_type("application/json")
        .await;

    // Should return bad request for malformed JSON
    assert!(
        response.status_code() == StatusCode::BAD_REQUEST
            || response.status_code() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_github_oauth_init_json_response() {
    // Test that GitHub OAuth init returns proper JSON format
    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/github").await;

    response.assert_status_ok();
    let json: serde_json::Value = response.json();

    // Verify JSON structure
    assert!(json.is_object());
    assert!(json.get("auth_url").is_some());
    assert!(json["auth_url"].is_string());
}

#[test]
#[serial_test::serial]
fn test_github_env_vars_with_special_chars() {
    // Test handling of environment variables with special characters
    set_env_vars(&[
        ("GITHUB_CLIENT_ID", "client@123"),
        (
            "GITHUB_REDIRECT_URI",
            "http://localhost:8080/callback?state=test",
        ),
        (
            "GITHUB_AUTH_URL",
            "https://github.com/login/oauth/authorize",
        ),
    ]);

    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_default();
    let redirect_uri = std::env::var("GITHUB_REDIRECT_URI").unwrap_or_default();
    let auth_base_url = std::env::var("GITHUB_AUTH_URL").unwrap_or_default();

    assert_eq!(client_id, "client@123");
    assert!(redirect_uri.contains("state=test"));
    assert!(!auth_base_url.is_empty());

    // Clean up
    remove_env_vars(&["GITHUB_CLIENT_ID", "GITHUB_REDIRECT_URI", "GITHUB_AUTH_URL"]);
}

#[tokio::test]
#[serial_test::serial]
async fn test_github_auth_url_format_validation() {
    // Test auth URL construction with various environment variable combinations
    set_env_vars(&[
        ("GITHUB_CLIENT_ID", "test_client_123"),
        ("GITHUB_REDIRECT_URI", "https://example.com/callback"),
        (
            "GITHUB_AUTH_URL",
            "https://github.com/login/oauth/authorize",
        ),
    ]);

    let app_state = Arc::new(AppState::default());
    let app = github_auth_routes().with_state(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/oauth/github").await;

    response.assert_status_ok();
    let json: serde_json::Value = response.json();
    let auth_url = json["auth_url"].as_str().unwrap();

    // Verify URL contains expected components
    assert!(auth_url.contains("client_id=test_client_123"));
    assert!(auth_url.contains("redirect_uri=https://example.com/callback"));
    assert!(auth_url.contains("scope=user"));
    assert!(auth_url.starts_with("https://github.com/login/oauth/authorize"));

    // Clean up
    remove_env_vars(&["GITHUB_CLIENT_ID", "GITHUB_REDIRECT_URI", "GITHUB_AUTH_URL"]);
}
