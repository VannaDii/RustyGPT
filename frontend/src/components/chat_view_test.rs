use chrono::Utc;
use uuid::Uuid;
use wasm_bindgen_test::*;
use yew::prelude::*;

use crate::components::chat_view::ChatView;
use shared::models::{Conversation, Message, Timestamp};

wasm_bindgen_test_configure!(run_in_browser);

fn create_test_conversation() -> Conversation {
    let conversation_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    Conversation {
        id: conversation_id,
        title: "Test Conversation".into(),
        participant_ids: vec![user_id],
        messages: vec![
            Message {
                id: Uuid::new_v4(),
                sender_id: user_id,
                conversation_id,
                content: "Hello, world!".into(),
                timestamp: Timestamp(Utc::now()),
            },
            Message {
                id: Uuid::new_v4(),
                sender_id: user_id,
                conversation_id,
                content: "This is a test message".into(),
                timestamp: Timestamp(Utc::now()),
            },
        ],
        last_updated: Timestamp(Utc::now()),
    }
}

#[wasm_bindgen_test]
fn chat_view_renders_with_conversation() {
    // Create test data
    let conversation = create_test_conversation();

    // Create the component
    let props = yew::props!(ChatView {
        conversation: Some(conversation.clone()),
    });

    // Render the component
    let rendered = yew::ServerRenderer::<ChatView>::with_props(props)
        .render()
        .expect("Failed to render component");

    // Assert that the rendered HTML contains the expected elements
    assert!(rendered.contains("<div class=\"chat-view\">"));
    assert!(rendered.contains("<h2>Test Conversation</h2>"));
    assert!(rendered.contains("<div class=\"messages\">"));
    assert!(rendered.contains("<div class=\"message\">"));
    assert!(rendered.contains("<p>Hello, world!</p>"));
    assert!(rendered.contains("<p>This is a test message</p>"));
}

#[wasm_bindgen_test]
fn chat_view_renders_without_conversation() {
    // Create the component with no conversation
    let props = yew::props!(ChatView { conversation: None });

    // Render the component
    let rendered = yew::ServerRenderer::<ChatView>::with_props(props)
        .render()
        .expect("Failed to render component");

    // Assert that the rendered HTML contains the expected elements
    assert!(rendered.contains("<div class=\"chat-view\">"));
    assert!(rendered.contains("<p>Select a conversation to start chatting</p>"));
    assert!(!rendered.contains("<h2>"));
    assert!(!rendered.contains("<div class=\"messages\">"));
}

#[wasm_bindgen_test]
fn chat_view_renders_empty_conversation() {
    // Create a conversation with no messages
    let conversation_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let conversation = Conversation {
        id: conversation_id,
        title: "Empty Conversation".into(),
        participant_ids: vec![user_id],
        messages: vec![],
        last_updated: Timestamp(Utc::now()),
    };

    // Create the component
    let props = yew::props!(ChatView {
        conversation: Some(conversation),
    });

    // Render the component
    let rendered = yew::ServerRenderer::<ChatView>::with_props(props)
        .render()
        .expect("Failed to render component");

    // Assert that the rendered HTML contains the expected elements
    assert!(rendered.contains("<div class=\"chat-view\">"));
    assert!(rendered.contains("<h2>Empty Conversation</h2>"));
    assert!(rendered.contains("<div class=\"messages\">"));
    assert!(!rendered.contains("<div class=\"message\">"));
}
