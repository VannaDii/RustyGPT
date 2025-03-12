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
    let is_dark_mode = use_state(|| {
        window()
            .match_media("(prefers-color-scheme: dark)")
            .map(|m| m.unwrap().matches())
            .unwrap_or(false)
    });

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
    let start_new_chat = {
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

    let toggle_theme = {
        let is_dark_mode = is_dark_mode.clone();
        Callback::from(move |_: yew::MouseEvent| {
            let new_mode = !*is_dark_mode;
            is_dark_mode.set(new_mode);

            // Update the document theme
            let document = web_sys::window().unwrap().document().unwrap();
            let html = document.document_element().unwrap();

            if new_mode {
                html.set_attribute("data-theme", "dark").unwrap();
            } else {
                html.set_attribute("data-theme", "light").unwrap();
            }
        })
    };

    // Get i18n context
    let i18n = use_context::<YewI18n>().expect("No I18n context found");

    // Helper function to get translations
    let t = |key: &str| i18n.translate(key);

    // Sidebar content
    let sidebar_content = html! {
        <div class="h-screen flex flex-col bg-base-200">
            <div class="p-4 border-b border-base-300">
                <button class="btn btn-primary w-full flex items-center justify-center gap-2" onclick={start_new_chat.clone()}>
                    <i class="fas fa-plus"></i>
                    <span>{ t("sidebar.new_chat") }</span>
                </button>
            </div>

            <div class="flex-1 overflow-y-auto custom-scrollbar">
                <div class="p-3">
                    <div class="relative">
                        <input
                            type="text"
                            placeholder={ t("sidebar.search") }
                            class="w-full p-3 pl-10 rounded-lg border border-base-300 bg-base-100 text-base-content"
                            oninput={on_search.reform(|e: yew::events::InputEvent| -> String {
                                let input: HtmlInputElement = e.target_dyn_into().unwrap();
                                input.value()
                            })}
                        />
                        <i class="fas fa-search absolute left-3 top-1/2 transform -translate-y-1/2 text-base-content/50"></i>
                    </div>
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

            <div class="p-4 border-t border-base-300">
                <div class="flex items-center justify-between mb-3">
                    <LanguageSelector />
                    <button
                        class="theme-toggle"
                        onclick={toggle_theme}
                        title={if *is_dark_mode { "Switch to light mode" } else { "Switch to dark mode" }}
                    >
                        {
                            if *is_dark_mode {
                                html! { <i class="fas fa-sun"></i> }
                            } else {
                                html! { <i class="fas fa-moon"></i> }
                            }
                        }
                    </button>
                </div>
                {
                    if let Some(user) = (*user).clone() {
                        html! {
                            <div class="flex items-center gap-3">
                                <div class="avatar">
                                    <div class="w-10 h-10 rounded-full bg-primary text-primary-content flex items-center justify-center">
                                        <span class="text-lg font-medium">{ &user[0..1] }</span>
                                    </div>
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
    };

    // Main content
    let main_content = html! {
        <div class="flex-1 flex flex-col bg-base-100 h-screen">
            <div class="navbar bg-base-100 border-b border-base-300 px-4 md:hidden">
                <div class="flex-1">
                    <label for="drawer-toggle" class="btn btn-ghost drawer-button">
                        <i class="fas fa-bars"></i>
                    </label>
                </div>
                <div class="flex-none">
                    <button class="btn btn-ghost" onclick={start_new_chat.clone()}>
                        <i class="fas fa-plus"></i>
                    </button>
                </div>
            </div>

            <div class="flex-1 overflow-hidden">
                {
                    if selected_conversation.is_none() && conversations.is_empty() {
                        // Welcome screen when no conversations exist
                        html! {
                            <div class="welcome-screen">
                                <h1 class="welcome-title">{ t("welcome.title") }</h1>
                                <p class="welcome-subtitle">{ t("welcome.subtitle") }</p>

                                <div class="welcome-examples">
                                    <div class="example-card" onclick={start_new_chat.clone()}>
                                        <div class="example-title">{ t("welcome.example1.title") }</div>
                                        <div class="example-description">{ t("welcome.example1.description") }</div>
                                    </div>
                                    <div class="example-card" onclick={start_new_chat.clone()}>
                                        <div class="example-title">{ t("welcome.example2.title") }</div>
                                        <div class="example-description">{ t("welcome.example2.description") }</div>
                                    </div>
                                    <div class="example-card" onclick={start_new_chat.clone()}>
                                        <div class="example-title">{ t("welcome.example3.title") }</div>
                                        <div class="example-description">{ t("welcome.example3.description") }</div>
                                    </div>
                                </div>
                            </div>
                        }
                    } else {
                        // Chat view
                        html! {
                            <ChatView conversation={(*selected_conversation).clone()} />
                        }
                    }
                }
            </div>

            <div class="p-4 border-t border-base-300">
                <div class="max-w-3xl mx-auto w-full">
                    <ChatInput
                        on_send={on_send}
                        conversation_id={selected_conversation.as_ref().map(|c| c.id)}
                        user_id={selected_conversation.as_ref().and_then(|c| c.participant_ids.first().cloned())}
                    />
                </div>
            </div>
        </div>
    };

    html! {
        <div class="drawer lg:drawer-open">
            <input id="drawer-toggle" type="checkbox" class="drawer-toggle" />

            <div class="drawer-content">
                { main_content }
            </div>

            <div class="drawer-side z-10">
                <label for="drawer-toggle" class="drawer-overlay"></label>
                <div class="w-80">
                    { sidebar_content }
                </div>
            </div>
        </div>
    }
}

// Helper function to get window object
fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}
