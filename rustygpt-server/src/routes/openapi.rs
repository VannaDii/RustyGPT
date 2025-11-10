use std::sync::Arc;

use crate::{app_state::AppState, openapi::ApiDoc};
use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

async fn openapi_yaml() -> impl IntoResponse {
    match ApiDoc::openapi().to_yaml() {
        Ok(yaml) => (StatusCode::OK, yaml),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("YAML error: {e}"),
        ),
    }
}

pub fn openapi_routes() -> Router<Arc<AppState>> {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi/rustygpt.json", ApiDoc::openapi()))
        .route("/openapi/rustygpt.yaml", get(openapi_yaml))
}
