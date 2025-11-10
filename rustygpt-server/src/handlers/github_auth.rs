use crate::{
    app_state::AppState,
    handlers::oauth_testable::{
        github_oauth_callback_with_service, github_oauth_manual_with_service,
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

// Handler for initiating GitHub OAuth flow
#[utoipa::path(
    get,
    path = "/oauth/github",
    responses(
        (status = 200, description = "Authorization URL retrieved", body = OAuthInitResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn github_oauth_init() -> Json<OAuthInitResponse> {
    // In a real implementation, this would generate a proper OAuth URL with state
    let github_client_id = env::var("GITHUB_CLIENT_ID").unwrap_or_default();
    let redirect_uri = env::var("GITHUB_REDIRECT_URI").unwrap_or_default();
    let auth_base_url = env::var("GITHUB_AUTH_URL").unwrap_or_default();

    let auth_url = format!(
        "{auth_base_url}?client_id={github_client_id}&redirect_uri={redirect_uri}&scope=user"
    );

    Json(OAuthInitResponse { auth_url })
}

// Handler for GitHub OAuth callback
#[utoipa::path(
    get,
    path = "/oauth/github/callback",
    responses(
        (status = 302, description = "Redirect post authentication"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn github_oauth_callback(
    query: Query<OAuthCallback>,
    state: State<Arc<AppState>>,
) -> Response {
    github_oauth_callback_with_service(query, state, ProductionOAuthService).await
}

// Handler for manual GitHub OAuth (for testing with direct auth code)
#[utoipa::path(
    post,
    path = "/oauth/github/manual",
    responses(
        (status = 200, description = "Revealed user ID", body = String),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn github_oauth_manual(
    state: State<Arc<AppState>>,
    payload: Json<OAuthRequest>,
) -> Response {
    github_oauth_manual_with_service(state, payload, ProductionOAuthService).await
}

// Function to register the GitHub OAuth routes
pub fn github_auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/oauth/github", get(github_oauth_init))
        .route("/oauth/github/callback", get(github_oauth_callback))
        .route("/oauth/github/manual", post(github_oauth_manual))
}
