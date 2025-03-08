use crate::handlers::conversation::conversation_routes;
use axum::Router;

pub fn create_router() -> Router {
    Router::new().merge(conversation_routes()) // Merge multiple route groups here
}
