use shared::models::MessageChunk;
use uuid::Uuid;
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{EventSource, MessageEvent};
use yew::{Callback, Html, Properties, function_component, html, use_effect, use_state};

#[derive(Properties, PartialEq)]
pub struct StreamingMessageProps {
    pub user_id: Uuid,
    pub on_message_chunk: Callback<MessageChunk>,
}

#[function_component(StreamingMessage)]
pub fn streaming_message(props: &StreamingMessageProps) -> Html {
    let connected = use_state(|| false);
    let error = use_state(|| None::<String>);

    // Set up EventSource when component mounts
    {
        let user_id = props.user_id;
        let connected_clone = connected.clone();
        let error_clone = error.clone();
        let on_message_chunk = props.on_message_chunk.clone();

        use_effect(move || {
            let event_source = EventSource::new(&format!("/api/stream/{}", user_id))
                .expect("Failed to create EventSource");

            let connected = connected_clone.clone();
            let on_open = Closure::wrap(Box::new(move || {
                connected.set(true);
            }) as Box<dyn FnMut()>);

            let error = error_clone.clone();
            let connected = connected_clone.clone();
            let on_error = Closure::wrap(Box::new(move |e: JsValue| {
                error.set(Some(format!("SSE Error: {:?}", e)));
                connected.set(false);
            }) as Box<dyn FnMut(JsValue)>);

            let on_message = {
                let on_message_chunk = on_message_chunk.clone();
                Closure::wrap(Box::new(move |e: MessageEvent| {
                    let data = e.data().as_string().unwrap_or_default();
                    match serde_json::from_str::<MessageChunk>(&data) {
                        Ok(chunk) => {
                            on_message_chunk.emit(chunk);
                        }
                        Err(err) => {
                            web_sys::console::error_1(
                                &format!("Failed to parse message: {}", err).into(),
                            );
                        }
                    }
                }) as Box<dyn FnMut(MessageEvent)>)
            };

            event_source.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            event_source.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            event_source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

            // Keep closures alive
            on_open.forget();
            on_error.forget();
            on_message.forget();

            // Cleanup function
            move || {
                event_source.close();
            }
        });
    }

    html! {
        <div class="streaming-status">
            {
                if let Some(err) = (*error).clone() {
                    html! { <div class="error">{ err }</div> }
                } else if *connected {
                    html! { <div class="connected">{ "Connected to streaming server" }</div> }
                } else {
                    html! { <div class="connecting">{ "Connecting to streaming server..." }</div> }
                }
            }
        </div>
    }
}
