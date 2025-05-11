use crate::handlers::apple_auth::apple_auth_routes;

#[tokio::test]
async fn test_apple_auth_routes_exist() {
    tracing::info!("Testing Apple auth routes creation");
    // Create the router with the Apple auth routes
    let _app = apple_auth_routes();
}
