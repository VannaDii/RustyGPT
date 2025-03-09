use uuid::Uuid;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{HtmlInputElement, Request, RequestInit, RequestMode, Response, console};
use yew::{Callback, Html, Properties, events::SubmitEvent, function_component, html, use_state};

#[derive(Properties, PartialEq)]
pub struct ChatInputProps {
    pub on_send: Callback<String>,
    #[prop_or_default]
    pub conversation_id: Option<Uuid>,
    #[prop_or_default]
    pub user_id: Option<Uuid>,
}

#[function_component(ChatInput)]
pub fn chat_input(props: &ChatInputProps) -> Html {
    let input_value = use_state(|| String::new());
    let is_sending = use_state(|| false);
    let error = use_state(|| None::<String>);

    let on_input = {
        let input_value = input_value.clone();
        Callback::from(move |e: yew::events::InputEvent| {
            if let Some(target) = e.target() {
                if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                    input_value.set(input.value());
                }
            }
        })
    };

    let on_submit = {
        let input_value = input_value.clone();
        let on_send = props.on_send.clone();
        let conversation_id = props.conversation_id;
        let user_id = props.user_id;
        let is_sending = is_sending.clone();
        let error = error.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if input_value.is_empty() || *is_sending {
                return;
            }

            // Check if we have the required IDs
            let conversation_id = match conversation_id {
                Some(id) => id,
                None => {
                    error.set(Some("No conversation selected".to_string()));
                    return;
                }
            };

            let user_id = match user_id {
                Some(id) => id,
                None => {
                    error.set(Some("User ID not available".to_string()));
                    return;
                }
            };

            let message = (*input_value).clone();
            is_sending.set(true);

            // Send the message to the server
            let cloned_message = message.clone();
            let input_value_clone = input_value.clone();
            let error_clone = error.clone();
            let is_sending_clone = is_sending.clone();
            let on_send_clone = on_send.clone();

            spawn_local(async move {
                match send_message(conversation_id, user_id, &cloned_message).await {
                    Ok(_) => {
                        // Message sent successfully
                        on_send_clone.emit(cloned_message);
                        input_value_clone.set(String::new());
                        error_clone.set(None);
                    }
                    Err(err) => {
                        error_clone.set(Some(format!("Failed to send message: {}", err)));
                    }
                }
                is_sending_clone.set(false);
            });
        })
    };

    html! {
        <form class="chat-input" onsubmit={on_submit}>
            {
                if let Some(err) = (*error).clone() {
                    html! { <div class="error">{ err }</div> }
                } else {
                    html! {}
                }
            }
            <input
                type="text"
                value={(*input_value).clone()}
                oninput={on_input}
                disabled={*is_sending}
                placeholder={if *is_sending { "Sending..." } else { "Type a message..." }}
            />
            <button type="submit" disabled={*is_sending}>
                { if *is_sending { "Sending..." } else { "Send" } }
            </button>
        </form>
    }
}

// Function to send a message to the server
async fn send_message(conversation_id: Uuid, user_id: Uuid, content: &str) -> Result<(), String> {
    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.mode(RequestMode::Cors);

    // Create the request body
    let body = serde_json::json!({
        "content": content,
        "user_id": user_id.to_string()
    });

    opts.body(Some(&JsValue::from_str(
        &serde_json::to_string(&body).unwrap(),
    )));

    let url = format!("/api/conversations/{}/messages", conversation_id);

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("Failed to set headers: {:?}", e))?;

    let window = web_sys::window().ok_or("No window found")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Failed to fetch: {:?}", e))?;

    let resp: Response = resp_value
        .dyn_into::<Response>()
        .map_err(|_| "Failed to convert response".to_string())?;

    if !resp.ok() {
        return Err(format!("Server error: {}", resp.status()));
    }

    Ok(())
}
