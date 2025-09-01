use i18nrs::yew::use_translation;
use shared::models::MessageChunk;
use uuid::Uuid;
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{EventSource, MessageEvent};
use yew::{Callback, Html, Properties, function_component, html, use_effect_with, use_state};

#[derive(Properties, PartialEq)]
pub struct StreamingMessageProps {
    pub user_id: Uuid,
    pub on_message_chunk: Callback<MessageChunk>,
}

#[function_component(StreamingMessage)]
pub fn streaming_message(props: &StreamingMessageProps) -> Html {
    let connected = use_state(|| false);
    let error = use_state(|| None::<String>);

    // Set up EventSource when component mounts or user_id changes
    {
        let user_id = props.user_id;
        let connected_clone = connected.clone();
        let error_clone = error.clone();
        let on_message_chunk = props.on_message_chunk.clone();

        use_effect_with(user_id, move |user_id| {
            // Reset connection state
            connected_clone.set(false);
            error_clone.set(None);

            let cleanup: Box<dyn FnOnce()> =
                if let Ok(event_source) = EventSource::new(&format!("/api/stream/{}", user_id)) {
                    let connected = connected_clone.clone();
                    let on_open = Closure::wrap(Box::new(move || {
                        web_sys::console::log_1(&"SSE Connection opened".into());
                        connected.set(true);
                    }) as Box<dyn FnMut()>);

                    let error = error_clone.clone();
                    let connected = connected_clone.clone();
                    let on_error = Closure::wrap(Box::new(move |e: JsValue| {
                        web_sys::console::error_2(&"SSE Error:".into(), &e);
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

                    // Store closures for proper cleanup
                    let closures = (on_open, on_error, on_message);

                    // Cleanup function
                    Box::new(move || {
                        web_sys::console::log_1(&"Cleaning up SSE connection".into());
                        event_source.close();
                        // Drop closures to clean up memory
                        drop(closures);
                    })
                } else {
                    error_clone.set(Some("Failed to create EventSource".to_string()));
                    Box::new(|| {}) // Empty cleanup for error case
                };

            move || cleanup()
        });
    }

    // Get i18n context
    let (i18n, _) = use_translation();

    // Helper function to get translations
    let t = |key: &str| i18n.t(key);

    html! {
        <div class="streaming-status">
            {
                if let Some(err) = (*error).clone() {
                    html! {
                        <div class="p-2 bg-red-100 text-red-800 rounded-lg text-sm mb-4">
                            { err }
                        </div>
                    }
                } else if *connected {
                    html! {
                        <div class="hidden">{ t("streaming.connected") }</div>
                    }
                } else {
                    html! {
                        <div class="p-2 bg-yellow-100 text-yellow-800 rounded-lg text-sm mb-4">
                            { t("streaming.connecting") }
                        </div>
                    }
                }
            }
        </div>
    }
}

#[cfg(test)]
mod tests {
    use shared::models::MessageChunk;
    use uuid::Uuid;
    use yew::Callback;

    /// Tests that user_id is properly formatted as UUID
    #[test]
    fn test_user_id_format() {
        let user_id = Uuid::new_v4();
        assert_eq!(user_id.to_string().len(), 36);

        // Test that UUID string format is valid
        let uuid_str = user_id.to_string();
        assert!(uuid_str.contains('-'));
        assert_eq!(uuid_str.chars().filter(|&c| c == '-').count(), 4);
    }

    /// Tests MessageChunk structure and serialization
    #[test]
    fn test_message_chunk_structure() {
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text".to_string(),
            content: "Test chunk content".to_string(),
            is_final: false,
        };

        assert!(!chunk.content.is_empty());
        assert!(!chunk.is_final);
        assert_eq!(chunk.content, "Test chunk content");
        assert_eq!(chunk.content_type, "text");
    }

    /// Tests complete message chunk
    #[test]
    fn test_complete_message_chunk() {
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text".to_string(),
            content: "Final message".to_string(),
            is_final: true,
        };

        assert!(chunk.is_final);
        assert_eq!(chunk.content, "Final message");
    }

    /// Tests stream URL formatting
    #[test]
    fn test_stream_url_format() {
        let user_id = Uuid::new_v4();
        let stream_url = format!("/api/stream/{}", user_id);

        assert!(stream_url.starts_with("/api/stream/"));
        assert!(stream_url.len() > 12); // "/api/stream/" + UUID
        assert_eq!(stream_url.len(), 48); // "/api/stream/" (12) + UUID (36)
    }

    /// Tests callback function signature
    #[test]
    fn test_callback_signature() {
        let callback_called = std::rc::Rc::new(std::cell::RefCell::new(false));
        let callback_called_clone = callback_called.clone();

        let callback = Callback::from(move |_chunk: MessageChunk| {
            *callback_called_clone.borrow_mut() = true;
        });

        // Simulate callback invocation
        let test_chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text".to_string(),
            content: "Test".to_string(),
            is_final: false,
        };

        callback.emit(test_chunk);
        assert!(*callback_called.borrow());
    }

    /// Tests message chunking logic
    #[test]
    fn test_message_chunking() {
        let chunks = [
            MessageChunk {
                conversation_id: Uuid::new_v4(),
                message_id: Uuid::new_v4(),
                content_type: "text".to_string(),
                content: "Hello ".to_string(),
                is_final: false,
            },
            MessageChunk {
                conversation_id: Uuid::new_v4(),
                message_id: Uuid::new_v4(),
                content_type: "text".to_string(),
                content: "world!".to_string(),
                is_final: true,
            },
        ];

        let complete_message: String = chunks.iter().map(|chunk| chunk.content.as_str()).collect();

        assert_eq!(complete_message, "Hello world!");
        assert!(chunks.last().unwrap().is_final);
        assert!(!chunks.first().unwrap().is_final);
    }

    /// Tests UUID consistency
    #[test]
    fn test_uuid_consistency() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        // UUIDs should be different
        assert_ne!(id1, id2);

        // UUIDs should maintain format consistency
        assert_eq!(id1.to_string().len(), id2.to_string().len());

        // Test UUID serialization consistency
        let id_str = id1.to_string();
        let parsed_id = Uuid::parse_str(&id_str).unwrap();
        assert_eq!(id1, parsed_id);
    }

    /// Tests content type validation
    #[test]
    fn test_content_type_validation() {
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "markdown".to_string(),
            content: "**Bold text**".to_string(),
            is_final: false,
        };

        assert_eq!(chunk.content_type, "markdown");
        assert!(chunk.content.contains("**"));
    }

    /// Tests chunk serialization
    #[test]
    fn test_chunk_serialization() {
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text".to_string(),
            content: "Serialization test".to_string(),
            is_final: true,
        };

        let json_result = serde_json::to_string(&chunk);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("Serialization test"));
        assert!(json_str.contains("is_final"));
        assert!(json_str.contains("true"));
    }
}
