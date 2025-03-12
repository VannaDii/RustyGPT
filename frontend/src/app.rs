use crate::YewI18n;
use crate::components::chat_input::ChatInput;
use crate::components::chat_list::ChatList;
use crate::components::chat_view::ChatView;
use crate::components::language_selector::LanguageSelector;
use crate::models::{Conversation, Message, Timestamp};
use chrono::Utc;
use uuid::Uuid;
use web_sys::HtmlInputElement;
use yew::TargetCast;
use yew::{Callback, Html, UseStateHandle, function_component, html, use_context, use_state};

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

    // Get i18n context
    let i18n = use_context::<YewI18n>().expect("No I18n context found");

    // Helper function to get translations
    let t = |key: &str| i18n.translate(key);

    html! {
        <div class="flex h-screen">
            // Sidebar
            <div class="w-80 hidden md:flex md:flex-col bg-base-200 border-r border-border-color">
                <div class="p-4 border-b border-border-color">
                    <button class="btn btn-primary w-full flex items-center justify-center gap-2" onclick={_start_new_chat.clone()}>
                        <span class="text-lg">{"+"}</span>
                        <span>{ t("sidebar.new_chat") }</span>
                    </button>
                </div>

                <div class="flex-1 overflow-y-auto">
                    <div class="p-3">
                        <input
                            type="text"
                            placeholder={ t("sidebar.search") }
                            class="w-full p-3 rounded-lg border border-border-color bg-base-100 text-base-content"
                            oninput={on_search.reform(|e: yew::events::InputEvent| -> String {
                                let input: HtmlInputElement = e.target_dyn_into().unwrap();
                                input.value()
                            })}
                        />
                    </div>

                    if filtered_conversations.is_empty() {
                        <div class="flex flex-col items-center justify-center h-40 text-base-content/60">
                            <p class="text-center">{ t("sidebar.no_conversations") }</p>
                            <p class="text-center text-sm">{ t("sidebar.start_new") }</p>
                        </div>
                    } else {
                        <ChatList conversations={filtered_conversations} on_select={on_select} on_delete={on_delete} />
                    }
                </div>

                <div class="p-4 border-t border-border-color">
                    <div class="flex items-center justify-between mb-3">
                        <LanguageSelector />
                    </div>
                    {
                        if let Some(user) = (*user).clone() {
                            html! {
                                <div class="flex items-center gap-3">
                                    <div class="w-10 h-10 rounded-full overflow-hidden">
                                        <img src="https://via.placeholder.com/40" alt={user.clone()} />
                                    </div>
                                    <span class="text-base-content">{ user }</span>
                                </div>
                            }
                        } else {
                            html! {
                                <button class="btn btn-outline w-full">{ t("sidebar.login") }</button>
                            }
                        }
                    }
                </div>
            </div>

            // Mobile sidebar toggle
            <div class="md:hidden absolute top-4 left-4 z-10">
                <button class="btn btn-circle btn-outline">
                    <span class="text-xl">{"☰"}</span>
                </button>
            </div>

            // Main content
            <div class="flex-1 flex flex-col bg-base-100 relative">
                <ChatView conversation={(*selected_conversation).clone()} />

                <div class="p-4 border-t border-border-color">
                    <div class="max-w-3xl mx-auto w-full">
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
