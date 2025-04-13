//! Routes for Copilot-compatible endpoints.

use crate::{
    app_state::AppState,
    handlers::copilot::{get_models, post_chat_completions},
};
use axum::{Router, routing::get, routing::post};
use std::sync::Arc;

/// Configures the Copilot API routes.
///
/// # Returns
/// A [`Router`](axum::Router) with the Copilot API routes.
pub fn create_router_copilot() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/models", get(get_models))
        .route("/v1/chat/completions", post(post_chat_completions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_create_router_copilot() {
        let app_state = Arc::new(AppState::default());
        let router = create_router_copilot().with_state(app_state);

        // Test `/v1/models` route
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test `/v1/chat/completions` route with valid payload
        let valid_payload = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                { "role": "user", "content": "Hello!" }
            ],
            "temperature": 0.7
        });

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("Content-Type", "application/json")
                    .body(Body::from(valid_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        println!("{:?}", response);
        assert_eq!(response.status(), StatusCode::OK);
    }
}
