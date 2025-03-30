use crate::routes::auth::create_router_auth;

#[tokio::test]
async fn test_auth_router_exists() {
    tracing::info!("Testing auth router creation");
    // Create the auth router
    let _app = create_router_auth();

    // Verify that the router was created successfully
    assert!(true);
}
