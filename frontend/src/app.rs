use crate::components::chat_input::ChatInput;
use crate::components::chat_list::ChatList;
use crate::components::chat_view::ChatView;
use crate::models::{Conversation, Message, Timestamp};
use chrono::Utc;
use uuid::Uuid;
use yew::{Callback, Html, function_component, html, use_state};

#[function_component(App)]
pub fn app() -> Html {
    let conversations = use_state(|| vec![]);
    let selected_conversation = use_state(|| None);

    let on_select = {
        let selected_conversation = selected_conversation.clone();
        let conversations = conversations.clone();
        Callback::from(move |id: String| {
            let id_uuid = Uuid::parse_str(&id).unwrap();
            if let Some(con) = conversations
                .iter()
                .find(|c: &&Conversation| c.id == id_uuid)
                .cloned()
            {
                selected_conversation.set(Some(con));
            }
        })
    };

    let on_delete = {
        let conversations = conversations.clone();
        let selected_conversation = selected_conversation.clone();
        Callback::from(move |id: String| {
            let id_uuid = Uuid::parse_str(&id).unwrap();
            conversations.set(
                conversations
                    .iter()
                    .cloned()
                    .filter(|c| c.id != id_uuid)
                    .collect(),
            );
            selected_conversation.set(None);
        })
    };

    let on_send = {
        let conversations = conversations.clone();
        let selected_conversation = selected_conversation.clone();
        Callback::from(move |msg: String| {
            if let Some(mut conv) = selected_conversation.as_ref().cloned() {
                let new_msg = Message {
                    id: Uuid::new_v4(),
                    content: msg,
                    timestamp: Timestamp(Utc::now()),
                    sender_id: Uuid::new_v4(),
                    conversation_id: conv.id,
                };
                conv.messages.push(new_msg);
                selected_conversation.set(Some(conv.clone()));
                conversations.set(
                    conversations
                        .iter()
                        .cloned()
                        .map(|c| if c.id == conv.id { conv.clone() } else { c })
                        .collect(),
                );
            }
        })
    };

    let start_new_chat = {
        let conversations = conversations.clone();
        let selected_conversation = selected_conversation.clone();
        Callback::from(move |_| {
            let new_conversation = Conversation {
                id: Uuid::new_v4(),
                title: "New Chat".into(),
                participant_ids: vec![],
                messages: vec![],
                last_updated: Timestamp(Utc::now()),
            };
            let mut convs = conversations.to_vec();
            convs.push(new_conversation.clone());
            conversations.set(convs);
            selected_conversation.set(Some(new_conversation));
        })
    };

    html! {
        <div class="app">
            <ChatList conversations={(*conversations).clone()} on_select={on_select} on_delete={on_delete} />
            <button onclick={start_new_chat}>{ "New Chat" }</button>
            <ChatView conversation={(*selected_conversation).clone()} />
            <ChatInput on_send={on_send} />
        </div>
    }
}
