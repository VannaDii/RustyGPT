use std::sync::Arc;

use axum::{
    Router, middleware,
    routing::{delete, get, put},
};

use crate::{app_state::AppState, handlers::admin_limits, middleware::auth::auth_middleware};

pub fn create_router_admin() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/admin/limits/profiles",
            get(admin_limits::list_profiles).post(admin_limits::create_profile),
        )
        .route(
            "/admin/limits/profiles/{id}",
            put(admin_limits::update_profile).delete(admin_limits::delete_profile),
        )
        .route(
            "/admin/limits/assignments",
            get(admin_limits::list_assignments).post(admin_limits::assign_route),
        )
        .route(
            "/admin/limits/assignments/{id}",
            delete(admin_limits::delete_assignment),
        )
        .route_layer(middleware::from_fn(auth_middleware))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_router_has_routes() {
        let router = create_router_admin();
        assert!(router.has_routes(), "admin router should not be empty");
    }
}
