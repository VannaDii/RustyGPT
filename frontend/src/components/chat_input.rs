use crate::YewI18n;
use uuid::Uuid;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{HtmlInputElement, Request, RequestInit, RequestMode, Response};
use yew::{
    Callback, Html, Properties, events::SubmitEvent, function_component, html, use_context,
    use_state,
};

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
    let input_value = use_state(String::new);
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

    // Get i18n context
    let i18n = use_context::<YewI18n>().expect("No I18n context found");

    // Helper function to get translations
    let t = |key: &str| i18n.translate(key);

    html! {
        <div class="chat-input">
            <form onsubmit={on_submit}>
                {
                    if let Some(err) = (*error).clone() {
                        html! { <div class="text-red-500 text-sm mb-2">{ err }</div> }
                    } else {
                        html! {}
                    }
                }
                <textarea
                    class="w-full p-3 rounded-lg border border-border-color bg-base-100 text-base-content resize-none min-h-[60px] pr-12"
                    value={(*input_value).clone()}
                    oninput={on_input}
                    disabled={*is_sending}
                    placeholder={if *is_sending { t("input.sending") } else { t("input.placeholder") }}
                    rows="1"
                ></textarea>
                <button
                    type="submit"
                    class="absolute right-3 bottom-3 rounded-full w-8 h-8 flex items-center justify-center"
                    disabled={*is_sending || input_value.is_empty()}
                    class={if *is_sending || input_value.is_empty() {
                        "bg-primary/50 text-primary-content/50"
                    } else {
                        "bg-primary text-primary-content"
                    }}
                >
                    {
                        if *is_sending {
                            html! { <span class="loading-dots"></span> }
                        } else {
                            html! { <i class="fas fa-paper-plane"></i> }
                        }
                    }
                </button>
            </form>
        </div>
    }
}

// Function to send a message to the server
async fn send_message(conversation_id: Uuid, user_id: Uuid, content: &str) -> Result<(), String> {
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    // Create the request body
    let body = serde_json::json!({
        "content": content,
        "user_id": user_id.to_string()
    });

    // Convert to JsValue and set as body
    let body_str = serde_json::to_string(&body).unwrap();
    let js_value = JsValue::from_str(&body_str);
    // The set_body method expects a JsValue directly, not an Option<&JsValue>
    opts.set_body(&js_value);

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
