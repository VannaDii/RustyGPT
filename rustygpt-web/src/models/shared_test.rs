use chrono::Utc;
use uuid::Uuid;
use wasm_bindgen_test::*;

use crate::models::{Conversation, Message, Timestamp, User};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn models_can_be_imported() {
    // This test verifies that the models can be imported and used

    // Create a user
    let user = User {
        id: Uuid::new_v4(),
        username: "test_user".to_string(),
        email: "test@example.com".to_string(),
    };

    // Create a message
    let message = Message {
        id: Uuid::new_v4(),
        sender_id: user.id,
        conversation_id: Uuid::new_v4(),
        content: "Test message".to_string(),
        timestamp: Timestamp(Utc::now()),
    };

    // Create a conversation
    let conversation = Conversation {
        id: Uuid::new_v4(),
        title: "Test Conversation".to_string(),
        participant_ids: vec![user.id],
        messages: vec![message.clone()],
        last_updated: Timestamp(Utc::now()),
    };

    // Verify that the models were created correctly
    assert_eq!(user.username, "test_user");
    assert_eq!(message.content, "Test message");
    assert_eq!(conversation.title, "Test Conversation");
    assert_eq!(conversation.messages.len(), 1);
    assert_eq!(conversation.messages[0].content, "Test message");
}
