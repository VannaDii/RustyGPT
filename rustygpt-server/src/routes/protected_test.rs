use crate::routes::protected::create_router_protected;

#[tokio::test]
async fn test_protected_router_exists() {
    tracing::info!("Testing protected router creation");
    // Create the protected router
    let _app = create_router_protected();
}
