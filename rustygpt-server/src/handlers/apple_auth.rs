use crate::{app_state::AppState, services::oauth_service::handle_apple_oauth};
use axum::{
    Router,
    extract::{Json, Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use http::StatusCode;
use shared::models::{
    ErrorResponse,
    oauth::{OAuthCallback, OAuthInitResponse, OAuthRequest},
};
use std::{env, sync::Arc};

// Handler for initiating Apple OAuth flow
#[utoipa::path(
    get,
    path = "/oauth/apple",
    responses(
        (status = 200, description = "Authorization URL retrieved", body = OAuthInitResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
#[axum::debug_handler]
pub async fn apple_oauth_init() -> Json<OAuthInitResponse> {
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
#[utoipa::path(
    get,
    path = "/oauth/apple/callback",
    responses(
        (status = 302, description = "Authorization URL retrieved"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
#[axum::debug_handler]
pub async fn apple_oauth_callback(
    Query(params): Query<OAuthCallback>,
    State(state): State<Arc<AppState>>,
) -> Response {
    match handle_apple_oauth(&state.pool, params.code).await {
        Ok(user_id) => {
            // In a real app, you would set a cookie or return a JWT token
            // For now, redirect to a success page with the user ID
            Redirect::to(&format!("/auth-success.html?user_id={}", user_id)).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "OAuth failed").into_response(),
    }
}

// Handler for manual Apple OAuth (for testing with direct auth code)
#[utoipa::path(
    post,
    path = "/oauth/apple/manual",
    responses(
        (status = 200, description = "Authorization URL retrieved", body = String),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
#[axum::debug_handler]
pub async fn apple_oauth_manual(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<OAuthRequest>,
) -> Response {
    match handle_apple_oauth(&state.pool, payload.auth_code).await {
        Ok(user_id) => (
            StatusCode::OK,
            format!("Apple OAuth successful, user_id: {}", user_id),
        )
            .into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "OAuth failed").into_response(),
    }
}

// Function to register the Apple OAuth routes
pub fn apple_auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/oauth/apple", get(apple_oauth_init))
        .route("/oauth/apple/callback", get(apple_oauth_callback))
        .route("/oauth/apple/manual", post(apple_oauth_manual))
}
