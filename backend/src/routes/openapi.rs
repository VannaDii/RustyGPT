use std::sync::Arc;

use crate::{app_state::AppState, openapi::ApiDoc};
use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use utoipa::OpenApi;

async fn openapi_yaml() -> impl IntoResponse {
    match ApiDoc::openapi().to_yaml() {
        Ok(yaml) => (StatusCode::OK, yaml),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("YAML error: {}", e),
        ),
    }
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

pub fn openapi_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/openapi/rustygpt.json", get(openapi_json))
        .route("/openapi/rustygpt.yaml", get(openapi_yaml))
}
