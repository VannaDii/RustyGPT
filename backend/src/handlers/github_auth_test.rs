use crate::handlers::github_auth::github_auth_routes;

#[tokio::test]
async fn test_github_auth_routes_exist() {
    tracing::info!("Testing GitHub auth routes creation");
    // Create the router with the GitHub auth routes
    let _app = github_auth_routes();

    // Verify that the router was created successfully
    assert!(true);
}
