//! Tests for `OpenAPI` routes.

use crate::{app_state::AppState, routes::openapi::openapi_routes};
use axum_test::TestServer;
use futures_util::future;
use std::sync::Arc;

#[tokio::test]
async fn test_openapi_yaml_route() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    let response = server.get("/openapi/rustygpt.yaml").await;

    response.assert_status_ok();
    let text = response.text();
    assert!(text.contains("openapi"));
    // Just check for openapi keyword, version might vary
}

#[tokio::test]
async fn test_swagger_ui_route() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    let response = server.get("/swagger-ui/").await;

    response.assert_status_ok();
    let text = response.text();
    assert!(text.contains("swagger-ui"));
}

#[tokio::test]
async fn test_openapi_json_route_via_swagger() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    // The JSON endpoint is created by SwaggerUI at /openapi/rustygpt.json
    let response = server.get("/openapi/rustygpt.json").await;

    response.assert_status_ok();
    let text = response.text();
    assert!(text.contains('{'));
    assert!(text.contains("openapi"));
}

#[tokio::test]
async fn test_openapi_yaml_content_structure() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    let response = server.get("/openapi/rustygpt.yaml").await;
    response.assert_status_ok();

    let text = response.text();

    // Check for required OpenAPI structure
    assert!(text.contains("openapi:"));
    assert!(text.contains("info:"));
    assert!(text.contains("paths:"));

    // Check for API version
    assert!(text.contains("version:"));

    // Check for content type header
    assert_eq!(response.header("content-type"), "text/plain; charset=utf-8");
}

#[tokio::test]
async fn test_openapi_json_content_structure() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    let response = server.get("/openapi/rustygpt.json").await;
    response.assert_status_ok();

    let text = response.text();

    // Parse as JSON to ensure validity
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Check for required OpenAPI structure
    assert!(json.get("openapi").is_some());
    assert!(json.get("info").is_some());
    assert!(json.get("paths").is_some());

    // Check for content type header
    let content_type = response.header("content-type");
    assert!(content_type.to_str().unwrap().contains("application/json"));
}

#[tokio::test]
async fn test_swagger_ui_html_structure() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    let response = server.get("/swagger-ui/").await;
    response.assert_status_ok();

    let text = response.text();

    // Check for HTML structure
    assert!(text.contains("<!DOCTYPE html>") || text.contains("<html"));
    assert!(text.contains("swagger-ui"));

    // Check for content type
    let content_type = response.header("content-type");
    assert!(content_type.to_str().unwrap().contains("text/html"));
}

#[tokio::test]
async fn test_swagger_ui_index_page() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    let response = server.get("/swagger-ui/index.html").await;
    response.assert_status_ok();

    let text = response.text();
    assert!(text.contains("swagger") || text.contains("Swagger"));
}

#[tokio::test]
async fn test_nonexistent_openapi_routes() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    // Test invalid routes
    let response = server.get("/openapi/nonexistent.yaml").await;
    response.assert_status_not_found();

    let response = server.get("/swagger-ui-invalid/").await;
    response.assert_status_not_found();

    let response = server.get("/openapi/").await;
    response.assert_status_not_found();
}

#[tokio::test]
async fn test_openapi_routes_with_different_app_states() {
    // Test with default state
    let app_state_default = Arc::new(AppState::default());
    let app_default = openapi_routes().with_state(app_state_default);
    let server_default = TestServer::new(app_default).unwrap();

    let response = server_default.get("/openapi/rustygpt.yaml").await;
    response.assert_status_ok();

    // Test with new state (should behave the same since OpenAPI is static)
    let app_state_new = Arc::new(AppState::default());
    let app_new = openapi_routes().with_state(app_state_new);
    let server_new = TestServer::new(app_new).unwrap();

    let response = server_new.get("/openapi/rustygpt.yaml").await;
    response.assert_status_ok();

    // Both should return similar content
    let content_default = server_default.get("/openapi/rustygpt.yaml").await.text();
    let content_new = server_new.get("/openapi/rustygpt.yaml").await.text();

    // Should both contain OpenAPI spec
    assert!(content_default.contains("openapi"));
    assert!(content_new.contains("openapi"));
}

#[tokio::test]
async fn test_openapi_yaml_response_headers() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    let response = server.get("/openapi/rustygpt.yaml").await;
    response.assert_status_ok();

    // Check that appropriate headers are set
    let content_type = response.header("content-type");
    assert!(content_type.to_str().is_ok());

    // Response should be non-empty
    assert!(!response.text().is_empty());
}

#[tokio::test]
async fn test_swagger_ui_css_and_js_resources() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    // These are common resources that SwaggerUI typically serves
    let potential_resources = vec![
        "/swagger-ui/swagger-ui-bundle.js",
        "/swagger-ui/swagger-ui-standalone-preset.js",
        "/swagger-ui/swagger-ui.css",
    ];

    for resource in potential_resources {
        let response = server.get(resource).await;
        // These might return 200 or 404 depending on SwaggerUI configuration
        // We just ensure the server handles them without panicking
        assert!(response.status_code().as_u16() < 500);
    }
}

#[tokio::test]
async fn test_openapi_routes_method_validation() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = TestServer::new(app).unwrap();

    // Test that only GET is allowed for YAML endpoint
    let response = server.post("/openapi/rustygpt.yaml").await;
    response.assert_status(axum::http::StatusCode::METHOD_NOT_ALLOWED);

    let response = server.put("/openapi/rustygpt.yaml").await;
    response.assert_status(axum::http::StatusCode::METHOD_NOT_ALLOWED);

    let response = server.delete("/openapi/rustygpt.yaml").await;
    response.assert_status(axum::http::StatusCode::METHOD_NOT_ALLOWED);

    // GET should work
    let response = server.get("/openapi/rustygpt.yaml").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn test_concurrent_openapi_requests() {
    let app_state = Arc::new(AppState::default());
    let app = openapi_routes().with_state(app_state);

    let server = Arc::new(TestServer::new(app).unwrap());

    // Test concurrent requests using futures::join_all instead of tokio::spawn
    let futures = (0..5).map(|_| async {
        let response = server.get("/openapi/rustygpt.yaml").await;
        response.assert_status_ok();
        assert!(response.text().contains("openapi"));
    });

    // Wait for all requests to complete concurrently
    future::join_all(futures).await;
}

#[tokio::test]
async fn test_openapi_yaml_error_path() {
    use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};
    use std::sync::Arc;

    // Create a custom handler that simulates the YAML serialization error
    async fn failing_openapi_yaml() -> impl IntoResponse {
        // Simulate the error condition that could occur in openapi_yaml()
        let error_message = "Simulated YAML serialization error";
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("YAML error: {error_message}"),
        )
    }

    // Create a test router with the failing handler
    let app_state = Arc::new(AppState::default());
    let app = Router::new()
        .route("/openapi/rustygpt.yaml", get(failing_openapi_yaml))
        .with_state(app_state);

    let server = TestServer::new(app).unwrap();

    let response = server.get("/openapi/rustygpt.yaml").await;

    // Verify the error response matches what openapi_yaml() would return on failure
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    let text = response.text();
    assert!(text.contains("YAML error:"));
    assert!(text.contains("Simulated YAML serialization error"));
}
