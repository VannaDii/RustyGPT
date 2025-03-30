use std::sync::Arc;

use crate::AppState;
use crate::handlers::setup::setup_routes;
use axum::Router;
use tracing::info;

/// Function to register the setup routes
pub fn create_router_setup() -> Router<Arc<AppState>> {
    info!("Creating setup router");
    Router::new().merge(setup_routes())
}
