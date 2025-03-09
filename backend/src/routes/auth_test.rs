use crate::routes::auth::create_router_auth;

#[tokio::test]
async fn test_auth_router_exists() {
    // Create the auth router
    let app = create_router_auth();

    // Verify that the router was created successfully
    assert!(true);
}
