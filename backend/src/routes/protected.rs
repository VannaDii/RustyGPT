use crate::handlers::{conversation::conversation_routes, streaming::simple_sse_handler};
use axum::{Router, routing::get};

/// Function to register the protected routes
pub fn create_router_protected() -> Router {
    // Create the main router
    let router = Router::new().merge(conversation_routes());

    // Add the streaming route
    let router = router.route("/api/stream/{user_id}", get(simple_sse_handler));

    router
}
