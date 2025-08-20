use crate::{
    app_state::AppState,
    handlers::oauth_testable::{
        apple_oauth_callback_with_service, apple_oauth_manual_with_service,
    },
    services::oauth_service_trait::ProductionOAuthService,
};
use axum::{
    Router,
    extract::{Json, Query, State},
    response::Response,
    routing::{get, post},
};
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
    query: Query<OAuthCallback>,
    state: State<Arc<AppState>>,
) -> Response {
    apple_oauth_callback_with_service(query, state, ProductionOAuthService).await
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
    state: State<Arc<AppState>>,
    payload: Json<OAuthRequest>,
) -> Response {
    apple_oauth_manual_with_service(state, payload, ProductionOAuthService).await
}

// Function to register the Apple OAuth routes
pub fn apple_auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/oauth/apple", get(apple_oauth_init))
        .route("/oauth/apple/callback", get(apple_oauth_callback))
        .route("/oauth/apple/manual", post(apple_oauth_manual))
}
