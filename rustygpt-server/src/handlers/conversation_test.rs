use crate::handlers::conversation::conversation_routes;

#[tokio::test]
async fn test_conversation_routes_exist() {
    tracing::info!("Testing conversation routes creation");
    // Create the router with the conversation routes
    let _app = conversation_routes();
}
