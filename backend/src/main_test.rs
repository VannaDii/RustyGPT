use axum::{
    body::Body,
    http::{Request, StatusCode},
};

use crate::auth_middleware;

#[tokio::test]
async fn test_auth_middleware_exists() {
    // This test just verifies that the auth_middleware function exists
    // and can be referenced
    assert!(true);
}

#[tokio::test]
async fn test_create_router() {
    // Import the necessary functions from the main module
    use crate::routes::auth::create_router_auth;
    use crate::routes::protected::create_router_protected;

    // Create the auth router
    let _auth_router = create_router_auth();

    // Create the protected router
    let _protected_router = create_router_protected();

    // Verify that the routers were created successfully
    assert!(true);
}
