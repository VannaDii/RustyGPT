use crate::YewI18n;
use crate::components::streaming_message::StreamingMessage;
use crate::models::{Conversation, MessageChunk};
use std::collections::HashMap;
use uuid::Uuid;
use web_sys::console;
use yew::{Callback, Html, Properties, function_component, html, use_context, use_state};

#[derive(Properties, PartialEq)]
pub struct ChatViewProps {
    pub conversation: Option<Conversation>,
}

#[function_component(ChatView)]
pub fn chat_view(props: &ChatViewProps) -> Html {
    // State to track streaming messages
    let streaming_messages = use_state(HashMap::<Uuid, String>::new);

    // Callback for handling message chunks
    let on_message_chunk = {
        let streaming_messages = streaming_messages.clone();
        Callback::from(move |chunk: MessageChunk| {
            console::log_1(&format!("Received chunk: {:?}", chunk.content).into());

            streaming_messages.set({
                let mut map = (*streaming_messages).clone();

                // Append the chunk to the existing content or create a new entry
                let content = map.entry(chunk.message_id).or_insert_with(String::new);
                content.push_str(&chunk.content);

                // If this is the final chunk, we could do something special
                if chunk.is_final {
                    console::log_1(&"Final chunk received".into());
                }

                map
            });
        })
    };

    // Get the user ID from the conversation if available
    let user_id = props
        .conversation
        .as_ref()
        .and_then(|conv| conv.participant_ids.first())
        .cloned()
        .unwrap_or_else(Uuid::new_v4);

    // Get i18n context
    let i18n = use_context::<YewI18n>().expect("No I18n context found");

    // Helper function to get translations
    let t = |key: &str| i18n.translate(key);

    if let Some(conversation) = &props.conversation {
        html! {
            <div class="flex-1 overflow-y-auto p-4">
                <div class="max-w-3xl mx-auto">
                    <h2 class="text-xl font-bold text-center mb-6">{ &conversation.title }</h2>

                    // Add the streaming component
                    <StreamingMessage user_id={user_id} on_message_chunk={on_message_chunk.clone()} />

                    <div class="space-y-6">
                        // Regular messages
                        { for conversation.messages.iter().map(|msg| {
                            let is_user = msg.sender_id == user_id;
                            html! {
                                <div class={if is_user { "chat chat-end" } else { "chat chat-start" }}>
                                    <div class="chat-avatar">
                                        <img src={if is_user {
                                            "https://via.placeholder.com/40"
                                        } else {
                                            "https://via.placeholder.com/40?text=AI"
                                        }} alt={if is_user { "You" } else { "AI" }} />
                                    </div>
                                    <div>
                                        <div class="chat-bubble">{ &msg.content }</div>
                                        <div class="chat-footer">
                                            { format!("{:?}", msg.timestamp) }
                                        </div>
                                    </div>
                                </div>
                            }
                        })}

                        // Streaming messages
                        { for streaming_messages.iter().map(|(_id, content)| {
                            html! {
                                <div class="chat chat-start">
                                    <div class="chat-avatar">
                                        <img src="https://via.placeholder.com/40?text=AI" alt="AI" />
                                    </div>
                                    <div>
                                        <div class="chat-bubble">{ content }</div>
                                        <div class="chat-footer">
                                            <span class="loading-dots">{ t("chat.typing") }</span>
                                        </div>
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                </div>
            </div>
        }
    } else {
        html! {
            <div class="h-full flex flex-col items-center justify-center text-base-content/60">
                <div class="text-xl mb-2">{ t("chat.select_conversation") }</div>
                <div class="text-sm">{ t("chat.or_start_new") }</div>
            </div>
        }
    }
}
