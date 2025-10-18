use std::sync::Arc;

use crate::{
    app_state::AppState,
    handlers::{
        apple_auth::apple_auth_routes,
        auth::{login, logout, me, refresh},
        github_auth::github_auth_routes,
    },
    middleware::auth::auth_middleware,
};
use axum::{
    Router, middleware,
    routing::{get, post},
};
use tracing::info;

/// Function to register the auth routes
pub fn create_router_auth() -> Router<Arc<AppState>> {
    info!("Creating auth router");
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh))
        .route(
            "/auth/me",
            get(me).route_layer(middleware::from_fn(auth_middleware)),
        )
        .merge(apple_auth_routes())
        .merge(github_auth_routes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_router_auth() {
        let router = create_router_auth();

        // Assert that the router is created successfully
        assert!(router.has_routes(), "Router should not be empty");
    }
}
