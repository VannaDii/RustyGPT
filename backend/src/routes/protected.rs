use std::sync::Arc;

use crate::{
    app_state::AppState,
    handlers::{conversation::conversation_routes, streaming::simple_sse_handler},
};
use axum::{Router, routing::get};
use tracing::info;

/// Function to register the protected routes
pub fn create_router_protected() -> Router<Arc<AppState>> {
    info!("Creating protected router");
    Router::new()
        .merge(conversation_routes())
        // Add the streaming route
        .route("/api/stream/{user_id}", get(simple_sse_handler))
}
