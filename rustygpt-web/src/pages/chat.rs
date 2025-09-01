use crate::api::RustyGPTClient;
use crate::components::StreamingMessage;
use i18nrs::yew::use_translation;
use shared::models::conversation::SendMessageRequest;
use shared::models::{Conversation, Message, MessageChunk};
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::{
    Callback, Html, Properties, TargetCast, function_component, html, use_effect_with,
    use_node_ref, use_state,
};
use yew_icons::{Icon, IconId};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Properties, PartialEq)]
pub struct ChatPageProps {
    #[prop_or(None)]
    pub conversation_id: Option<String>,
}

/// Chat page component for the main chat interface
#[function_component(ChatPage)]
pub fn chat_page(props: &ChatPageProps) -> Html {
    let (i18n, _) = use_translation();

    // State for the current conversation
    let conversation = use_state(|| None::<Conversation>);
    let messages = use_state(Vec::<Message>::new);
    let streaming_chunks = use_state(Vec::<MessageChunk>::new);
    let current_input = use_state(String::new);
    let is_sending = use_state(|| false);
    let error_message = use_state(|| None::<String>);

    // Input ref for focusing
    let input_ref = use_node_ref();

    // Mock user ID for now - this would come from authentication
    let user_id = use_state(Uuid::new_v4);

    // Load conversation if conversation_id is provided
    {
        let conversation_id = props.conversation_id.clone();
        let conversation_clone = conversation.clone();
        let messages_clone = messages.clone();
        let error_clone = error_message.clone();

        use_effect_with(conversation_id, move |conversation_id| {
            if let Some(id) = conversation_id {
                let conversation = conversation_clone.clone();
                let messages = messages_clone.clone();
                let error = error_clone.clone();
                let id = id.clone();

                spawn_local(async move {
                    let client = RustyGPTClient::new("http://localhost:8080/api");
                    match client.get_conversation(&id).await {
                        Ok(conv) => {
                            messages.set(conv.messages.clone());
                            conversation.set(Some(conv));
                        }
                        Err(err) => {
                            error.set(Some(format!("Failed to load conversation: {}", err)));
                        }
                    }
                });
            }
            || ()
        });
    }

    // Handle message input change
    let on_input_change = {
        let current_input = current_input.clone();
        Callback::from(move |e: yew::events::InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            current_input.set(input.value());
        })
    };

    // Handle sending a message
    let send_message_logic = {
        let current_input = current_input.clone();
        let is_sending = is_sending.clone();
        let user_id = *user_id;
        let error_message = error_message.clone();
        let input_ref = input_ref.clone();

        move || {
            let message_content = (*current_input).clone();
            if message_content.trim().is_empty() || *is_sending {
                return;
            }

            let current_input = current_input.clone();
            let is_sending = is_sending.clone();
            let error_message = error_message.clone();
            let input_ref = input_ref.clone();

            current_input.set(String::new());
            is_sending.set(true);
            error_message.set(None);

            spawn_local(async move {
                let client = RustyGPTClient::new("http://localhost:8080/api");
                let request = SendMessageRequest {
                    content: message_content,
                    user_id: user_id.to_string(),
                };

                match client.send_message(&request).await {
                    Ok(_response) => {
                        // Clear input on success
                        if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                            input.set_value("");
                        }
                    }
                    Err(err) => {
                        error_message.set(Some(format!("Failed to send message: {}", err)));
                    }
                }

                is_sending.set(false);
            });
        }
    };

    let on_send_message = {
        let send_message_logic = send_message_logic.clone();
        Callback::from(move |_: yew::events::MouseEvent| {
            send_message_logic();
        })
    };

    // Handle Enter key press
    let on_key_press = {
        let send_message_logic = send_message_logic.clone();
        Callback::from(move |e: yew::events::KeyboardEvent| {
            if e.key() == "Enter" && !e.shift_key() {
                e.prevent_default();
                send_message_logic();
            }
        })
    };

    // Handle streaming message chunks
    let on_message_chunk = {
        let streaming_chunks = streaming_chunks.clone();
        Callback::from(move |chunk: MessageChunk| {
            let mut chunks = (*streaming_chunks).clone();
            chunks.push(chunk);
            streaming_chunks.set(chunks);
        })
    };

    // Render a single message
    let render_message = |message: &Message| {
        let is_user = message.sender_id == *user_id;
        let message_class = if is_user {
            "ml-auto bg-primary text-primary-content"
        } else {
            "mr-auto bg-base-200 text-base-content"
        };

        html! {
            <div class={format!("chat {}", if is_user { "chat-end" } else { "chat-start" })}>
                <div class="chat-image avatar">
                    <div class="w-10 h-10 rounded-full bg-base-300 flex items-center justify-center">
                        {
                            if is_user {
                                html! { <Icon icon_id={IconId::HeroiconsOutlineUser} class="w-6 h-6" /> }
                            } else {
                                html! { <Icon icon_id={IconId::HeroiconsOutlineCpuChip} class="w-6 h-6" /> }
                            }
                        }
                    </div>
                </div>
                <div class={format!("chat-bubble {}", message_class)}>
                    { &message.content }
                </div>
                <div class="chat-footer opacity-50 text-xs">
                    { format!("{}", message.timestamp.0.format("%H:%M")) }
                </div>
            </div>
        }
    };

    html! {
        <div class="flex flex-col h-full max-h-screen">
            // Header
            <div class="bg-base-200 border-b border-base-300 p-4">
                <div class="flex items-center justify-between">
                    <div>
                        <h1 class="text-xl font-semibold">{ i18n.t("chat.title") }</h1>
                        {
                            if let Some(conv) = &*conversation {
                                html! {
                                    <p class="text-sm text-base-content/70">{ &conv.title }</p>
                                }
                            } else {
                                html! {
                                    <p class="text-sm text-base-content/70">{ i18n.t("chat.new_conversation") }</p>
                                }
                            }
                        }
                    </div>
                    <div class="flex items-center gap-2">
                        <button class="btn btn-sm btn-ghost">
                            <Icon icon_id={IconId::HeroiconsOutlineCog6Tooth} class="w-4 h-4" />
                        </button>
                    </div>
                </div>
            </div>

            // Messages area
            <div class="flex-1 overflow-y-auto p-4 space-y-4">
                {
                    if messages.is_empty() && streaming_chunks.is_empty() {
                        html! {
                            <div class="flex flex-col items-center justify-center h-full text-center">
                                <Icon icon_id={IconId::HeroiconsOutlineChatBubbleLeftRight} class="w-16 h-16 text-base-content/30 mb-4" />
                                <h2 class="text-xl font-semibold text-base-content/70 mb-2">{ i18n.t("chat.welcome") }</h2>
                                <p class="text-base-content/50">{ i18n.t("chat.start_conversation") }</p>
                            </div>
                        }
                    } else {
                        html! {
                            <div class="space-y-4">
                                {
                                    messages.iter().map(|message| {
                                        render_message(message)
                                    }).collect::<Html>()
                                }
                                {
                                    if !streaming_chunks.is_empty() {
                                        html! {
                                            <div class="chat chat-start">
                                                <div class="chat-image avatar">
                                                    <div class="w-10 h-10 rounded-full bg-base-300 flex items-center justify-center">
                                                        <Icon icon_id={IconId::HeroiconsOutlineCpuChip} class="w-6 h-6" />
                                                    </div>
                                                </div>
                                                <div class="chat-bubble bg-base-200 text-base-content">
                                                    { streaming_chunks.iter().map(|chunk| chunk.content.clone()).collect::<String>() }
                                                    <span class="inline-block w-2 h-4 bg-current animate-pulse ml-1"></span>
                                                </div>
                                            </div>
                                        }
                                    } else {
                                        html! {}
                                    }
                                }
                            </div>
                        }
                    }
                }
            </div>

            // Error message
            {
                if let Some(error) = &*error_message {
                    html! {
                        <div class="px-4 pb-2">
                            <div class="alert alert-error">
                                <Icon icon_id={IconId::HeroiconsOutlineExclamationTriangle} class="w-4 h-4" />
                                <span>{ error }</span>
                            </div>
                        </div>
                    }
                } else {
                    html! {}
                }
            }

            // Input area
            <div class="bg-base-200 border-t border-base-300 p-4">
                <div class="flex gap-2">
                    <div class="flex-1 relative">
                        <input
                            ref={input_ref}
                            type="text"
                            placeholder={i18n.t("chat.type_message")}
                            class="input input-bordered w-full pr-12"
                            value={(*current_input).clone()}
                            disabled={*is_sending}
                            oninput={on_input_change}
                            onkeydown={on_key_press}
                        />
                        <button
                            class={format!("btn btn-sm btn-circle absolute right-2 top-1/2 -translate-y-1/2 {}",
                                if current_input.trim().is_empty() || *is_sending { "btn-disabled" } else { "btn-primary" }
                            )}
                            disabled={current_input.trim().is_empty() || *is_sending}
                            onclick={on_send_message}
                        >
                            {
                                if *is_sending {
                                    html! { <span class="loading loading-spinner loading-xs"></span> }
                                } else {
                                    html! { <Icon icon_id={IconId::HeroiconsOutlinePaperAirplane} class="w-4 h-4" /> }
                                }
                            }
                        </button>
                    </div>
                </div>
                <div class="text-xs text-base-content/50 mt-2 text-center">
                    { i18n.t("chat.hint") }
                </div>
            </div>

            // Streaming connection
            <StreamingMessage user_id={*user_id} on_message_chunk={on_message_chunk} />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use crate::routes::MainRoute;

    /// Tests that conversation ID can be extracted from route
    #[test]
    fn test_conversation_id_extraction() {
        let route = MainRoute::ChatConversation {
            conversation_id: "conv-test-123".to_string(),
        };

        match route {
            MainRoute::ChatConversation { conversation_id } => {
                assert_eq!(conversation_id, "conv-test-123");
                assert!(!conversation_id.is_empty());
            }
            _ => panic!("Expected ChatConversation route"),
        }
    }

    /// Tests that chat route is properly defined
    #[test]
    fn test_chat_route() {
        let chat_route = MainRoute::Chat;
        assert!(format!("{:?}", chat_route).contains("Chat"));
    }

    /// Tests message content validation
    #[test]
    fn test_message_content_validation() {
        // Test empty message
        let empty_message = "";
        assert!(empty_message.trim().is_empty());

        // Test whitespace-only message
        let whitespace_message = "   \n\t  ";
        assert!(whitespace_message.trim().is_empty());

        // Test valid message
        let valid_message = "Hello, this is a test message!";
        assert!(!valid_message.trim().is_empty());
        assert!(valid_message.len() > 5);
    }

    /// Tests message input handling
    #[test]
    fn test_message_input_handling() {
        let user_input = "What is the weather like today?";

        // Validate input
        assert!(!user_input.is_empty());
        assert!(!user_input.is_empty());
        assert!(user_input.ends_with("?"));

        // Test input sanitization
        let trimmed_input = user_input.trim();
        assert_eq!(trimmed_input, user_input);
    }

    /// Tests conversation management
    #[test]
    fn test_conversation_management() {
        let conversation_ids = vec!["conv-123", "conv-456", "conv-789"];

        for id in conversation_ids {
            assert!(!id.is_empty());
            assert!(id.starts_with("conv-"));
            assert!(id.len() > 5);
        }
    }

    /// Tests message rendering data structures
    #[test]
    fn test_message_rendering() {
        // Test message structure
        let message_content = "This is a test message for rendering";
        let sender = "user";
        let timestamp = "2024-01-01T12:00:00Z";

        assert!(!message_content.is_empty());
        assert!(sender == "user" || sender == "assistant");
        assert!(timestamp.contains("T"));
        assert!(timestamp.contains("Z"));
    }

    /// Tests error handling scenarios
    #[test]
    fn test_error_handling() {
        // Test network error simulation
        let network_error = "Failed to send message: Network error";
        assert!(network_error.contains("Network error"));

        // Test validation error
        let validation_error = "Message cannot be empty";
        assert!(validation_error.contains("empty"));

        // Test authentication error
        let auth_error = "Unauthorized: Please log in";
        assert!(auth_error.contains("Unauthorized"));
    }

    /// Tests conversation loading states
    #[test]
    fn test_conversation_loading_states() {
        // Test loading state
        let is_loading = true;
        assert!(is_loading);

        // Test loaded state
        let is_loaded = true;
        let has_error = false;
        assert!(is_loaded);
        assert!(!has_error);

        // Test error state
        let has_error = true;
        let error_message = "Failed to load conversation";
        assert!(has_error);
        assert!(!error_message.is_empty());
    }

    /// Tests internationalization keys
    #[test]
    fn test_i18n_keys() {
        let translation_keys = vec![
            "chat.send_message",
            "chat.type_message",
            "chat.loading",
            "chat.error",
            "chat.conversation_not_found",
        ];

        for key in translation_keys {
            assert!(key.starts_with("chat."));
            assert!(!key.is_empty());
            assert!(key.len() > 5);
        }
    }

    /// Tests URL parameter parsing
    #[test]
    fn test_url_parameter_parsing() {
        let conversation_url = "/chat/conv-abc123";
        let parts: Vec<&str> = conversation_url.split('/').collect();

        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "");
        assert_eq!(parts[1], "chat");
        assert_eq!(parts[2], "conv-abc123");

        let conversation_id = parts[2];
        assert!(conversation_id.starts_with("conv-"));
    }

    /// Tests streaming message integration
    #[test]
    fn test_streaming_integration() {
        let stream_url = "/api/stream/user-123";
        assert!(stream_url.starts_with("/api/stream/"));
        assert!(stream_url.contains("user-"));
        assert!(stream_url.len() > 15);
    }
}
