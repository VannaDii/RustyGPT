use wasm_bindgen_test::*;
use yew::prelude::*;

use crate::app::App;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn app_renders() {
    // Render the App component
    let rendered = yew::ServerRenderer::<App>::new()
        .render()
        .expect("Failed to render component");

    // Assert that the rendered HTML contains the expected elements
    assert!(rendered.contains("<div class=\"flex h-screen\">"));

    // Check for the search input
    assert!(rendered.contains("Search conversations"));

    // Check for the login button (since user is initially None)
    assert!(rendered.contains("Login / Sign Up"));

    // Check for the chat components
    assert!(rendered.contains("<div class=\"chat-view\">"));
    assert!(rendered.contains("<div class=\"chat-input\">"));
}

#[wasm_bindgen_test]
fn app_has_search_input() {
    // Render the App component
    let rendered = yew::ServerRenderer::<App>::new()
        .render()
        .expect("Failed to render component");

    // Check for the search input
    assert!(rendered.contains("<input"));
    assert!(rendered.contains("placeholder=\"Search conversations...\""));
}

#[wasm_bindgen_test]
fn app_has_chat_components() {
    // Render the App component
    let rendered = yew::ServerRenderer::<App>::new()
        .render()
        .expect("Failed to render component");

    // Check for the chat components
    assert!(rendered.contains("<div class=\"chat-view\">"));
    assert!(rendered.contains("<div class=\"chat-input\">"));
}

#[wasm_bindgen_test]
fn app_has_empty_state_message() {
    // Render the App component
    let rendered = yew::ServerRenderer::<App>::new()
        .render()
        .expect("Failed to render component");

    // Check for the empty state message (since conversations is initially empty)
    assert!(rendered.contains("No conversations found"));
    assert!(rendered.contains("Start a new one!"));
}

#[wasm_bindgen_test]
fn app_has_login_button() {
    // Render the App component
    let rendered = yew::ServerRenderer::<App>::new()
        .render()
        .expect("Failed to render component");

    // Check for the login button (since user is initially None)
    assert!(rendered.contains("Login / Sign Up"));
}
