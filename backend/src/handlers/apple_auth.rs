use crate::services::oauth_service::handle_apple_oauth;
use axum::{
    Router,
    extract::{Extension, Json, Query},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use http::StatusCode;
use shared::models::oauth::{OAuthCallback, OAuthInitResponse, OAuthRequest};
use sqlx::PgPool;
use std::env;

// Handler for initiating Apple OAuth flow
#[axum::debug_handler]
async fn apple_oauth_init() -> Json<OAuthInitResponse> {
    // In a real implementation, this would generate a proper OAuth URL with state
    let apple_client_id = env::var("APPLE_CLIENT_ID").unwrap_or_default();
    let redirect_uri = env::var("APPLE_REDIRECT_URI").unwrap_or_default();
    let auth_base_url = env::var("APPLE_AUTH_URL").unwrap_or_default();

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope=name%20email",
        auth_base_url, apple_client_id, redirect_uri
    );

    Json(OAuthInitResponse { auth_url })
}

// Handler for Apple OAuth callback
#[axum::debug_handler]
async fn apple_oauth_callback(
    Query(params): Query<OAuthCallback>,
    Extension(pool): Extension<PgPool>,
) -> Response {
    match handle_apple_oauth(&pool, params.code).await {
        Ok(user_id) => {
            // In a real app, you would set a cookie or return a JWT token
            // For now, redirect to a success page with the user ID
            Redirect::to(&format!("/auth-success.html?user_id={}", user_id)).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "OAuth failed").into_response(),
    }
}

// Handler for manual Apple OAuth (for testing with direct auth code)
#[axum::debug_handler]
async fn apple_oauth_manual(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<OAuthRequest>,
) -> Response {
    match handle_apple_oauth(&pool, payload.auth_code).await {
        Ok(user_id) => (
            StatusCode::OK,
            format!("Apple OAuth successful, user_id: {}", user_id),
        )
            .into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "OAuth failed").into_response(),
    }
}

// Function to register the Apple OAuth routes
pub fn apple_auth_routes() -> Router {
    Router::new()
        .route("/api/oauth/apple", get(apple_oauth_init))
        .route("/api/oauth/apple/callback", get(apple_oauth_callback))
        .route("/api/oauth/apple/manual", post(apple_oauth_manual))
}
