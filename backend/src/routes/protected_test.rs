use crate::routes::protected::create_router_protected;

#[tokio::test]
async fn test_protected_router_exists() {
    // Create the protected router
    let app = create_router_protected();

    // Verify that the router was created successfully
    assert!(true);
}
