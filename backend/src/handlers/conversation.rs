use axum::{Json, Router, routing::get};
use chrono::Utc;
use shared::models::{Conversation, Message, Timestamp};
use uuid::Uuid;

pub async fn get_conversations() -> Json<Vec<Conversation>> {
    let mock_conversations = vec![Conversation {
        id: Uuid::new_v4(),
        title: "Sample Chat".into(),
        participant_ids: vec![Uuid::new_v4()],
        messages: vec![Message {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            content: "Hello, world!".into(),
            timestamp: Timestamp(Utc::now()),
        }],
        last_updated: Timestamp(Utc::now()),
    }];
    Json(mock_conversations)
}

// Function to register the conversation routes
pub fn conversation_routes() -> Router {
    Router::new().route("/api/conversations", get(get_conversations))
}
