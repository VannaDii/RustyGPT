use crate::models::Conversation;
use yew::{Callback, Html, Properties, function_component, html};

#[derive(Properties, PartialEq)]
pub struct ChatListProps {
    pub conversations: Vec<Conversation>,
    pub on_select: Callback<String>,
    pub on_delete: Callback<String>,
}

#[function_component(ChatList)]
pub fn chat_list(props: &ChatListProps) -> Html {
    html! {
        <div class="chat-list">
            { for props.conversations.iter().map(|con| {
                let con_id = con.id;
                let on_select = props.on_select.clone();
                let on_delete = props.on_delete.clone();

                html! {
                    <div class="chat-item">
                        <span onclick={Callback::from(move |_| on_select.emit(con_id.to_string()))}>
                            { &con.title }
                        </span>
                        <button onclick={Callback::from(move |_| on_delete.emit(con_id.to_string()))}>
                            { "🗑" }
                        </button>
                    </div>
                }
            })}
        </div>
    }
}
