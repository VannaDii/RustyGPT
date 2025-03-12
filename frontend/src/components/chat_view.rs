use crate::YewI18n;
use crate::components::streaming_message::StreamingMessage;
use crate::models::{Conversation, MessageChunk};
use std::collections::HashMap;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, console};
use yew::{
    Callback, Html, NodeRef, Properties, function_component, html, use_context, use_effect_with,
    use_state,
};

#[derive(Properties, PartialEq)]
pub struct ChatViewProps {
    pub conversation: Option<Conversation>,
}

#[function_component(ChatView)]
pub fn chat_view(props: &ChatViewProps) -> Html {
    // State to track streaming messages
    let streaming_messages = use_state(HashMap::<Uuid, String>::new);

    // State to track which message is being copied
    let copying_message = use_state(|| None::<Uuid>);

    // Ref for the chat container to auto-scroll
    let chat_container_ref = NodeRef::default();

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

    // Auto-scroll to bottom when messages change
    {
        let chat_ref = chat_container_ref.clone();
        let messages_count = props.conversation.as_ref().map_or(0, |c| c.messages.len());
        let streaming_count = streaming_messages.len();

        use_effect_with((messages_count, streaming_count), move |_| {
            if let Some(element) = chat_ref.cast::<HtmlElement>() {
                element.set_scroll_top(element.scroll_height());
            }
            || ()
        });
    }

    // Function to copy message content
    let copy_message = {
        let copying_message = copying_message.clone();

        Callback::from(move |content: String| {
            let window = web_sys::window().unwrap();
            let navigator = window.navigator();

            // Use clipboard API to copy text
            let promise = navigator.clipboard().write_text(&content);

            let copying_message_clone = copying_message.clone();
            let msg_id = Uuid::new_v4(); // Just a temporary ID for UI feedback

            copying_message.set(Some(msg_id));

            // Reset copying state after 2 seconds
            let closure = Closure::once_into_js(move || {
                copying_message_clone.set(None);
            });

            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    2000,
                )
                .unwrap();

            // Prevent memory leaks
            let _ = promise;
        })
    };

    // Function to render markdown content
    let render_markdown = {
        Callback::from(move |content: String| {
            let window = web_sys::window().unwrap();
            let marked = js_sys::Reflect::get(&window, &JsValue::from_str("marked")).unwrap();
            let marked_fn = marked.dyn_into::<js_sys::Function>().unwrap();

            let result = js_sys::Reflect::apply(
                &marked_fn,
                &JsValue::NULL,
                &js_sys::Array::of1(&JsValue::from_str(&content)),
            )
            .unwrap();

            result.as_string().unwrap_or(content)
        })
    };

    if let Some(conversation) = &props.conversation {
        html! {
            <div ref={chat_container_ref.clone()} class="flex-1 overflow-y-auto p-4 custom-scrollbar">
                <div class="max-w-3xl mx-auto">
                    <h2 class="text-xl font-bold text-center mb-6">{ &conversation.title }</h2>

                    // Add the streaming component
                    <StreamingMessage user_id={user_id} on_message_chunk={on_message_chunk.clone()} />

                    <div class="space-y-6">
                        // Regular messages
                        { for conversation.messages.iter().map(|msg| {
                            let is_user = msg.sender_id == user_id;
                            let msg_id = msg.id;
                            let msg_content = msg.content.clone();
                            let copy_callback = copy_message.clone();
                            let is_copying = copying_message.as_ref() == Some(&msg_id);
                            let markdown_content = render_markdown.emit(msg.content.clone());

                            html! {
                                <div class={if is_user { "chat chat-end group" } else { "chat chat-start group" }}>
                                    <div class="chat-image avatar">
                                        <div class={if is_user {
                                            "w-10 h-10 rounded-full bg-secondary text-secondary-content flex items-center justify-center"
                                        } else {
                                            "w-10 h-10 rounded-full bg-primary text-primary-content flex items-center justify-center"
                                        }}>
                                            <span class="text-lg font-medium">{if is_user { "Y" } else { "A" }}</span>
                                        </div>
                                    </div>
                                    <div class="chat-header opacity-70 text-xs">
                                        {if is_user { "You" } else { "Assistant" }}
                                        <time class="ml-2">
                                            { format!("{}", msg.timestamp.0.format("%H:%M")) }
                                        </time>
                                    </div>
                                    <div class="chat-bubble relative">
                                        <div class="message-actions absolute -top-8 right-0">
                                            <button
                                                class="message-action-button"
                                                onclick={Callback::from(move |_| copy_callback.emit(msg_content.clone()))}
                                                title="Copy message"
                                            >
                                                {
                                                    if is_copying {
                                                        html! { <i class="fas fa-check"></i> }
                                                    } else {
                                                        html! { <i class="fas fa-copy"></i> }
                                                    }
                                                }
                                            </button>
                                            {
                                                if !is_user {
                                                    html! {
                                                        <button class="message-action-button" title="Regenerate response">
                                                            <i class="fas fa-refresh"></i>
                                                        </button>
                                                    }
                                                } else {
                                                    html! {}
                                                }
                                            }
                                        </div>
                                        <div class="markdown" dangerously_set_inner_html={markdown_content}></div>
                                    </div>
                                </div>
                            }
                        })}

                        // Streaming messages
                        { for streaming_messages.iter().map(|(_id, content)| {
                            let markdown_content = render_markdown.emit(content.clone());

                            html! {
                                <div class="chat chat-start">
                                    <div class="chat-image avatar">
                                        <div class="w-10 h-10 rounded-full bg-primary text-primary-content flex items-center justify-center">
                                            <span class="text-lg font-medium">{"A"}</span>
                                        </div>
                                    </div>
                                    <div class="chat-header opacity-70 text-xs">
                                        {"Assistant"}
                                    </div>
                                    <div class="chat-bubble">
                                        <div class="markdown" dangerously_set_inner_html={markdown_content}></div>
                                    </div>
                                    <div class="chat-footer">
                                        <div class="flex items-center">
                                            <div class="typing-dot"></div>
                                            <div class="typing-dot"></div>
                                            <div class="typing-dot"></div>
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
