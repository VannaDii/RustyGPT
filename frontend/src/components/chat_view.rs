use crate::components::streaming_message::StreamingMessage;
use crate::models::{Conversation, MessageChunk};
use std::collections::HashMap;
use uuid::Uuid;
use web_sys::console;
use yew::{Callback, Html, Properties, function_component, html, use_state};

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

    if let Some(conversation) = &props.conversation {
        html! {
            <div class="chat-view">
                <h2>{ &conversation.title }</h2>

                // Add the streaming component
                <StreamingMessage user_id={user_id} on_message_chunk={on_message_chunk.clone()} />

                <div class="messages">
                    // Regular messages
                    { for conversation.messages.iter().map(|msg| html! {
                        <div class="message" id={format!("msg-{}", msg.id)}>
                            <p>{ &msg.content }</p>
                            <span>{ &msg.timestamp }</span>
                        </div>
                    })}

                    // Streaming messages
                    { for streaming_messages.iter().map(|(id, content)| {
                        let message_id = *id;
                        html! {
                            <div class="message streaming" id={format!("streaming-msg-{}", message_id)}>
                                <p>{ content }</p>
                                <span class="streaming-indicator">{ "..." }</span>
                            </div>
                        }
                    })}
                </div>
            </div>
        }
    } else {
        html! { <div class="chat-view"><p>{ "Select a conversation to start chatting" }</p></div> }
    }
}
