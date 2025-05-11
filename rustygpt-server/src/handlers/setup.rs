use std::sync::Arc;

use crate::{
    app_state::AppState,
    services::setup::{init_setup, is_setup},
};
use axum::{
    Router,
    extract::{Json, State},
    response::{IntoResponse, Response},
    routing::get,
};
use http::StatusCode;
use shared::models::{ErrorResponse, SetupRequest, SetupResponse};
use tracing::{info, instrument};

// Handler for checking if the system is setup
#[utoipa::path(
    get,
    path = "/setup",
    responses(
        (status = 200, description = "Setup status retrieved successfully", body = SetupResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Setup"
)]
#[instrument(skip(state))]
pub async fn get_setup(State(state): State<Arc<AppState>>) -> Response {
    info!("Received setup check request");
    match is_setup(&state.pool).await {
        Ok(is_setup) => {
            info!("Setup check completed, is_setup: {}", is_setup);
            (StatusCode::OK, Json(SetupResponse { is_setup })).into_response()
        }
        Err(err) => {
            info!("Setup check failed: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Failed to check setup".to_string(),
                    details: Some(err.to_string()),
                }),
            )
                .into_response()
        }
    }
}

// Handler for configuring the system the first time
#[utoipa::path(
    post,
    path = "/setup",
    request_body = SetupRequest,
    responses(
        (status = 200, description = "Setup completed successfully"),
        (status = 400, description = "Setup rejected by database", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Setup"
)]
#[instrument(skip(state))]
pub async fn post_setup(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetupRequest>,
) -> Response {
    info!("Received setup request: {:?}", payload);
    match init_setup(&state.pool, &payload).await {
        Ok(true) => {
            info!("Setup completed successfully");
            (StatusCode::OK).into_response()
        }
        Ok(false) => {
            info!("Setup rejected by database");
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Database rejected setup.".to_string(),
                    details: Some("`init_setup` returned false".to_string()),
                }),
            )
                .into_response()
        }
        Err(err) => {
            info!("Setup failed: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Failed to setup".to_string(),
                    details: Some(err.to_string()),
                }),
            )
                .into_response()
        }
    }
}

// Function to register the setup routes
#[instrument]
pub fn setup_routes() -> Router<Arc<AppState>> {
    info!("Registering setup routes");
    Router::new().route("/setup", get(get_setup).post(post_setup))
}
