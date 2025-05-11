use std::sync::Arc;

use crate::app_state::AppState;
use crate::handlers::setup::setup_routes;
use axum::Router;
use tracing::info;

/// Function to register the setup routes
pub fn create_router_setup() -> Router<Arc<AppState>> {
    info!("Creating setup router");
    Router::new().merge(setup_routes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_router_setup() {
        let router = create_router_setup();

        // Assert that the router is created successfully
        assert!(router.has_routes(), "Router should not be empty");
    }
}
