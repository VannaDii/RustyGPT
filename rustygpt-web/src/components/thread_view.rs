use chrono::Utc;
use shared::models::{MessageRole, MessageView, Timestamp};
use uuid::Uuid;
use yew::{Callback, Html, Properties, function_component, html};

use super::message_node::MessageNode;

#[derive(Clone, PartialEq, Eq)]
pub struct StreamingDisplay {
    pub message_id: Uuid,
    pub root_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub conversation_id: Uuid,
    pub depth: i32,
    pub content: String,
}

#[derive(Properties, PartialEq)]
pub struct ThreadViewProps {
    pub messages: Vec<MessageView>,
    #[prop_or_default]
    pub streaming: Vec<StreamingDisplay>,
    pub on_reply: Callback<MessageView>,
}

#[function_component(ThreadView)]
pub fn thread_view(props: &ThreadViewProps) -> Html {
    if props.messages.is_empty() {
        return html! {
            <div class="p-6 text-sm text-base-content/70">
                {"Select a thread to see its messages."}
            </div>
        };
    }

    html! {
        <div class="flex flex-col gap-2">
            { for props.messages.iter().cloned().map(|message| {
                let on_reply = props.on_reply.clone();
                html! { <MessageNode message={message} on_reply={on_reply} /> }
            }) }
            { for props.streaming.iter().cloned().map(|entry| {
                let on_reply = props.on_reply.clone();
                let placeholder = MessageView {
                    id: entry.message_id,
                    root_id: entry.root_id,
                    parent_id: entry.parent_id,
                    conversation_id: entry.conversation_id,
                    author_user_id: None,
                    role: MessageRole::Assistant,
                    content: format!("{} ▌", entry.content),
                    path: String::new(),
                    depth: entry.depth,
                    created_at: Timestamp(Utc::now()),
                };
                html! {
                    <div class="animate-pulse opacity-80" key={entry.message_id.to_string()}>
                        <MessageNode message={placeholder} on_reply={on_reply} />
                    </div>
                }
            }) }
        </div>
    }
}
