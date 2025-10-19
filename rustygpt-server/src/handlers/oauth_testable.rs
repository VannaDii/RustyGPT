/// Testable handler functions that accept OAuth service as dependency
use crate::{app_state::AppState, services::oauth_service_trait::OAuthService};
use axum::{
    extract::{Json, Query, State},
    response::{IntoResponse, Redirect, Response},
};
use http::StatusCode;
use shared::models::oauth::{OAuthCallback, OAuthRequest};
use std::sync::Arc;

/// Testable GitHub OAuth callback handler
///
/// # Arguments
/// * `params` - OAuth callback parameters containing authorization code
/// * `state` - Application state containing database pool
/// * `oauth_service` - OAuth service implementation (can be mocked for testing)
///
/// # Returns
/// Redirect response to success page or error response
pub async fn github_oauth_callback_with_service<T: OAuthService>(
    Query(params): Query<OAuthCallback>,
    State(state): State<Arc<AppState>>,
    oauth_service: T,
) -> Response {
    oauth_service
        .handle_github_oauth(&state.pool, params.code)
        .await
        .map_or_else(
            |_| (StatusCode::INTERNAL_SERVER_ERROR, "OAuth failed").into_response(),
            |user_id| {
                Redirect::to(&format!("/auth-success.html?user_id={}", user_id)).into_response()
            },
        )
}

/// Testable GitHub OAuth manual handler
///
/// # Arguments
/// * `state` - Application state containing database pool
/// * `payload` - OAuth request containing authorization code
/// * `oauth_service` - OAuth service implementation (can be mocked for testing)
///
/// # Returns
/// Success message with user ID or error response
pub async fn github_oauth_manual_with_service<T: OAuthService>(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<OAuthRequest>,
    oauth_service: T,
) -> Response {
    oauth_service
        .handle_github_oauth(&state.pool, payload.auth_code)
        .await
        .map_or_else(
            |_| (StatusCode::INTERNAL_SERVER_ERROR, "OAuth failed").into_response(),
            |user_id| {
                (
                    StatusCode::OK,
                    format!("GitHub OAuth successful, user_id: {}", user_id),
                )
                    .into_response()
            },
        )
}

/// Testable Apple OAuth callback handler
///
/// # Arguments
/// * `params` - OAuth callback parameters containing authorization code
/// * `state` - Application state containing database pool
/// * `oauth_service` - OAuth service implementation (can be mocked for testing)
///
/// # Returns
/// Redirect response to success page or error response
pub async fn apple_oauth_callback_with_service<T: OAuthService>(
    Query(params): Query<OAuthCallback>,
    State(state): State<Arc<AppState>>,
    oauth_service: T,
) -> Response {
    oauth_service
        .handle_apple_oauth(&state.pool, params.code)
        .await
        .map_or_else(
            |_| (StatusCode::INTERNAL_SERVER_ERROR, "OAuth failed").into_response(),
            |user_id| {
                Redirect::to(&format!("/auth-success.html?user_id={}", user_id)).into_response()
            },
        )
}

/// Testable Apple OAuth manual handler
///
/// # Arguments
/// * `state` - Application state containing database pool
/// * `payload` - OAuth request containing authorization code
/// * `oauth_service` - OAuth service implementation (can be mocked for testing)
///
/// # Returns
/// Success message with user ID or error response
pub async fn apple_oauth_manual_with_service<T: OAuthService>(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<OAuthRequest>,
    oauth_service: T,
) -> Response {
    oauth_service
        .handle_apple_oauth(&state.pool, payload.auth_code)
        .await
        .map_or_else(
            |_| (StatusCode::INTERNAL_SERVER_ERROR, "OAuth failed").into_response(),
            |user_id| {
                (
                    StatusCode::OK,
                    format!("Apple OAuth successful, user_id: {}", user_id),
                )
                    .into_response()
            },
        )
}
