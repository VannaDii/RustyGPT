use wasm_bindgen_test::*;
use yew::Callback;
use yew::prelude::*;

use crate::components::chat_input::ChatInput;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn chat_input_renders() {
    // Create a callback that will be triggered when a message is sent
    let on_send = Callback::from(|_msg: String| {
        // In a real test, we would assert something about the message
    });

    // Create the component
    let props = yew::props!(ChatInput { on_send });

    // Render the component
    let rendered = yew::ServerRenderer::<ChatInput>::with_props(props)
        .render()
        .expect("Failed to render component");

    // Assert that the rendered HTML contains the expected elements
    assert!(rendered.contains("<div class=\"chat-input\">"));
    assert!(rendered.contains("<input type=\"text\""));
    assert!(rendered.contains("<button>"));
    assert!(rendered.contains("Send"));
}

#[wasm_bindgen_test]
fn chat_input_button_exists() {
    // Create a callback that will be triggered when a message is sent
    let on_send = Callback::from(|_msg: String| {
        // In a real test, we would assert something about the message
    });

    // Create the component
    let props = yew::props!(ChatInput { on_send });

    // Render the component
    let rendered = yew::ServerRenderer::<ChatInput>::with_props(props)
        .render()
        .expect("Failed to render component");

    // Assert that the rendered HTML contains a button
    assert!(rendered.contains("<button>"));
}

#[wasm_bindgen_test]
fn chat_input_has_text_input() {
    // Create a callback that will be triggered when a message is sent
    let on_send = Callback::from(|_msg: String| {
        // In a real test, we would assert something about the message
    });

    // Create the component
    let props = yew::props!(ChatInput { on_send });

    // Render the component
    let rendered = yew::ServerRenderer::<ChatInput>::with_props(props)
        .render()
        .expect("Failed to render component");

    // Assert that the rendered HTML contains a text input
    assert!(rendered.contains("<input type=\"text\""));
}
