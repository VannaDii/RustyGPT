use crate::components::chat_input::ChatInput;
use crate::components::chat_list::ChatList;
use crate::components::chat_view::ChatView;
use crate::models::{Conversation, Message, Timestamp};
use chrono::Utc;
use uuid::Uuid;
use web_sys::HtmlInputElement;
use yew::TargetCast;
use yew::{Callback, Html, UseStateHandle, function_component, html, use_state};

#[function_component(App)]
pub fn app() -> Html {
    let conversations = use_state(Vec::new);
    let selected_conversation = use_state(|| None);
    let search_query = use_state(String::new);
    let user: UseStateHandle<Option<String>> = use_state(|| None);

    let on_search = {
        let search_query = search_query.clone();
        Callback::from(move |query: String| {
            search_query.set(query);
        })
    };

    let filtered_conversations = conversations
        .iter()
        .filter(|c: &&Conversation| {
            c.title
                .to_lowercase()
                .contains(&search_query.to_lowercase())
        })
        .cloned()
        .collect::<Vec<_>>();

    let conversations_select = conversations.clone();
    let on_select = {
        let selected_conversation = selected_conversation.clone();
        Callback::from(move |id: String| {
            let id_uuid = Uuid::parse_str(&id).unwrap();
            if let Some(con) = conversations_select
                .iter()
                .find(|c| c.id == id_uuid)
                .cloned()
            {
                selected_conversation.set(Some(con));
            }
        })
    };

    let conversations_delete = conversations.clone();
    let on_delete = {
        let selected_conversation = selected_conversation.clone();
        Callback::from(move |id: String| {
            let id_uuid = Uuid::parse_str(&id).unwrap();
            conversations_delete.set(
                conversations_delete
                    .iter()
                    .filter(|&c| c.id != id_uuid)
                    .cloned()
                    .collect(),
            );
            selected_conversation.set(None);
        })
    };

    let conversations_send = conversations.clone();
    let on_send = {
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
                conversations_send.set(
                    conversations_send
                        .iter()
                        .cloned()
                        .map(|c| if c.id == conv.id { conv.clone() } else { c })
                        .collect(),
                );
            }
        })
    };

    let conversations_new = conversations.clone();
    let _start_new_chat = {
        let selected_conversation = selected_conversation.clone();
        Callback::from(move |_: yew::MouseEvent| {
            let new_conversation = Conversation {
                id: Uuid::new_v4(),
                title: "New Chat".into(),
                participant_ids: vec![],
                messages: vec![],
                last_updated: Timestamp(Utc::now()),
            };
            let mut convs = conversations_new.to_vec();
            convs.push(new_conversation.clone());
            conversations_new.set(convs);
            selected_conversation.set(Some(new_conversation));
        })
    };

    html! {
        <div class="flex h-screen">
            <div class="w-2/5 md:w-1/3 lg:w-1/4 xl:w-1/5 bg-base-200 p-4 flex flex-col relative">
                <input
                    type="text"
                    placeholder="Search conversations..."
                    class="w-full p-3 rounded-lg border border-gray-400 bg-gray-800 text-white"
                    oninput={on_search.reform(|e: yew::events::InputEvent| -> String {
                        let input: HtmlInputElement = e.target_dyn_into().unwrap();
                        input.value()
                    })}
                />
                if filtered_conversations.is_empty() {
                    <p class="text-gray-400 mt-6 text-center text-lg">{"No conversations found. Start a new one!"}</p>
                } else {
                    <ChatList conversations={filtered_conversations} on_select={on_select} on_delete={on_delete} />
                }
                <div class="absolute bottom-4 left-4 w-full flex items-center space-x-2 p-4 bg-opacity-80 bg-base-300 rounded-lg">
                    {
                        if let Some(user) = (*user).clone() {
                            html! {
                                <div class="flex items-center space-x-3">
                                    <img src="https://via.placeholder.com/40" class="rounded-full w-10 h-10" />
                                    <span class="text-white text-lg">{ user }</span>
                                </div>
                            }
                        } else {
                            html! {
                                <button class="btn btn-primary w-full">{"Login / Sign Up"}</button>
                            }
                        }
                    }
                </div>
            </div>
            <div class="flex-1 flex flex-col bg-base-100">
                <ChatView conversation={(*selected_conversation).clone()} />
                <div class="p-4 flex justify-center">
                    <div class="w-full md:w-3/5">
                        <ChatInput
                            on_send={on_send}
                            conversation_id={selected_conversation.as_ref().map(|c| c.id)}
                            user_id={selected_conversation.as_ref().and_then(|c| c.participant_ids.first().cloned())}
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}
