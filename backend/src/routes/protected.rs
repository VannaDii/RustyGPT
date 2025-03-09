use crate::handlers::conversation::conversation_routes;
use axum::Router;

/// Function to register the protected routes
pub fn create_router_protected() -> Router {
    Router::new().merge(conversation_routes()) // Merge multiple route groups here
}
