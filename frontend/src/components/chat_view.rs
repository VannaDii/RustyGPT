use crate::models::Conversation;
use yew::{Html, Properties, function_component, html};

#[derive(Properties, PartialEq)]
pub struct ChatViewProps {
    pub conversation: Option<Conversation>,
}

#[function_component(ChatView)]
pub fn chat_view(props: &ChatViewProps) -> Html {
    if let Some(conversation) = &props.conversation {
        html! {
            <div class="chat-view">
                <h2>{ &conversation.title }</h2>
                <div class="messages">
                    { for conversation.messages.iter().map(|msg| html! {
                        <div class="message">
                            <p>{ &msg.content }</p>
                            <span>{ &msg.timestamp }</span>
                        </div>
                    })}
                </div>
            </div>
        }
    } else {
        html! { <div class="chat-view"><p>{ "Select a conversation to start chatting" }</p></div> }
    }
}
