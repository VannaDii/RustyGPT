use std::sync::Arc;

use crate::{app_state::AppState, handlers::conversation::conversation_routes};
use axum::Router;
use tracing::info;

/// Function to register the protected routes
pub fn create_router_protected() -> Router<Arc<AppState>> {
    info!("Creating protected router");
    Router::new().merge(conversation_routes())
    // Note: SSE endpoint moved to unprotected routes for connection stability
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_router_protected() {
        let router = create_router_protected();

        // Assert that the router is created successfully
        assert!(router.has_routes(), "Router should not be empty");
    }
}
