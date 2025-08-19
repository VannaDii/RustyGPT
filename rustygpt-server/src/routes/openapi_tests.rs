//! Tests for OpenAPI routes.

use crate::{app_state::AppState, routes::openapi::openapi_routes};
use axum_test::TestServer;
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
    assert!(text.contains("{"));
    assert!(text.contains("openapi"));
}
