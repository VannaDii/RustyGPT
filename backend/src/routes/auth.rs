use std::sync::Arc;

use crate::app_state::AppState;
use crate::handlers::apple_auth::apple_auth_routes;
use crate::handlers::github_auth::github_auth_routes;
use axum::Router;
use tracing::info;

/// Function to register the auth routes
pub fn create_router_auth() -> Router<Arc<AppState>> {
    info!("Creating auth router");
    Router::new()
        .merge(apple_auth_routes())
        .merge(github_auth_routes())
}
