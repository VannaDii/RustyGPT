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
        <div class="space-y-1 px-2">
            { for props.conversations.iter().map(|con| {
                let con_id = con.id;
                let on_select = props.on_select.clone();
                let on_delete = props.on_delete.clone();

                html! {
                    <div class="conversation-item group">
                        <div
                            class="flex-1 truncate cursor-pointer"
                            onclick={Callback::from(move |_| on_select.emit(con_id.to_string()))}
                        >
                            <div class="font-medium truncate">{ &con.title }</div>
                            if let Some(last_msg) = con.messages.last() {
                                <div class="text-xs opacity-70 truncate">
                                    { &last_msg.content }
                                </div>
                            }
                        </div>
                        <button
                            class="opacity-0 group-hover:opacity-100 transition-opacity"
                            onclick={Callback::from(move |_| on_delete.emit(con_id.to_string()))}
                        >
                            <i class="fas fa-trash text-base-content/70 hover:text-base-content"></i>
                        </button>
                    </div>
                }
            })}
        </div>
    }
}
