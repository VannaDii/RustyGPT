use chrono::Utc;
use uuid::Uuid;
use wasm_bindgen_test::*;
use yew::Callback;
use yew::prelude::*;

use crate::components::chat_list::ChatList;
use shared::models::{Conversation, Timestamp};

wasm_bindgen_test_configure!(run_in_browser);

fn create_test_conversations() -> Vec<Conversation> {
    vec![
        Conversation {
            id: Uuid::new_v4(),
            title: "Test Conversation 1".into(),
            participant_ids: vec![Uuid::new_v4()],
            messages: vec![],
            last_updated: Timestamp(Utc::now()),
        },
        Conversation {
            id: Uuid::new_v4(),
            title: "Test Conversation 2".into(),
            participant_ids: vec![Uuid::new_v4()],
            messages: vec![],
            last_updated: Timestamp(Utc::now()),
        },
    ]
}

#[wasm_bindgen_test]
fn chat_list_renders() {
    // Create test data
    let conversations = create_test_conversations();

    // Create callbacks
    let on_select = Callback::from(|_id: String| {
        // In a real test, we would assert something about the selected conversation
    });

    let on_delete = Callback::from(|_id: String| {
        // In a real test, we would assert something about the deleted conversation
    });

    // Create the component
    let props = yew::props!(ChatList {
        conversations: conversations.clone(),
        on_select,
        on_delete,
    });

    // Render the component
    let rendered = yew::ServerRenderer::<ChatList>::with_props(props)
        .render()
        .expect("Failed to render component");

    // Assert that the rendered HTML contains the expected elements
    assert!(rendered.contains("<div class=\"chat-list\">"));
    assert!(rendered.contains("Test Conversation 1"));
    assert!(rendered.contains("Test Conversation 2"));
    assert!(rendered.contains("<button>"));
    assert!(rendered.contains("🗑"));
}

#[wasm_bindgen_test]
fn chat_list_empty() {
    // Create an empty list of conversations
    let conversations = Vec::new();

    // Create callbacks
    let on_select = Callback::from(|_id: String| {
        // In a real test, we would assert something about the selected conversation
    });

    let on_delete = Callback::from(|_id: String| {
        // In a real test, we would assert something about the deleted conversation
    });

    // Create the component
    let props = yew::props!(ChatList {
        conversations,
        on_select,
        on_delete,
    });

    // Render the component
    let rendered = yew::ServerRenderer::<ChatList>::with_props(props)
        .render()
        .expect("Failed to render component");

    // Assert that the rendered HTML contains the chat-list div but no chat items
    assert!(rendered.contains("<div class=\"chat-list\">"));
    assert!(!rendered.contains("<div class=\"chat-item\">"));
}

#[wasm_bindgen_test]
fn chat_list_has_delete_buttons() {
    // Create test data
    let conversations = create_test_conversations();

    // Create callbacks
    let on_select = Callback::from(|_id: String| {
        // In a real test, we would assert something about the selected conversation
    });

    let on_delete = Callback::from(|_id: String| {
        // In a real test, we would assert something about the deleted conversation
    });

    // Create the component
    let props = yew::props!(ChatList {
        conversations: conversations.clone(),
        on_select,
        on_delete,
    });

    // Render the component
    let rendered = yew::ServerRenderer::<ChatList>::with_props(props)
        .render()
        .expect("Failed to render component");

    // Count the number of delete buttons
    let button_count = rendered.matches("<button").count();

    // There should be one delete button per conversation
    assert_eq!(button_count, conversations.len());
}
