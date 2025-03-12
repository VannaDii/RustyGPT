use crate::YewI18n;
use crate::models::Conversation;
use chrono::Utc;
use yew::{Callback, Html, Properties, function_component, html, use_context};

#[derive(Properties, PartialEq)]
pub struct ChatListProps {
    pub conversations: Vec<Conversation>,
    pub on_select: Callback<String>,
    pub on_delete: Callback<String>,
}

#[function_component(ChatList)]
pub fn chat_list(props: &ChatListProps) -> Html {
    // Get i18n context
    let i18n = use_context::<YewI18n>().expect("No I18n context found");

    // Helper function to get translations
    let t = |key: &str| i18n.translate(key);

    // Sort conversations by last updated time (most recent first)
    let mut sorted_conversations = props.conversations.clone();
    sorted_conversations.sort_by(|a, b| b.last_updated.0.cmp(&a.last_updated.0));

    html! {
        <div class="space-y-1 px-2">
            { for sorted_conversations.iter().map(|con| {
                let con_id = con.id;
                let on_select = props.on_select.clone();
                let on_delete = props.on_delete.clone();

                // Calculate time difference for display
                let now = Utc::now();
                let diff = now.signed_duration_since(con.last_updated.0);

                let time_display = if diff.num_days() > 0 {
                    format!("{}d ago", diff.num_days())
                } else if diff.num_hours() > 0 {
                    format!("{}h ago", diff.num_hours())
                } else if diff.num_minutes() > 0 {
                    format!("{}m ago", diff.num_minutes())
                } else {
                    t("chat.just_now").to_string()
                };

                html! {
                    <div class="conversation-item group hover:bg-base-300 rounded-lg p-2">
                        <div class="flex justify-between items-start mb-1">
                            <div class="conversation-title">{ &con.title }</div>
                            <div class="flex items-center gap-2">
                                <span class="text-xs opacity-50">{ time_display }</span>
                                <button
                                    class="opacity-0 group-hover:opacity-100 transition-opacity p-1 hover:bg-base-200 rounded"
                                    onclick={Callback::from(move |e: yew::MouseEvent| {
                                        e.stop_propagation();
                                        on_delete.emit(con_id.to_string())
                                    })}
                                    title={t("sidebar.delete_conversation")}
                                >
                                    <i class="fas fa-trash text-base-content/70 hover:text-error text-xs"></i>
                                </button>
                            </div>
                        </div>
                        <div
                            class="conversation-preview cursor-pointer w-full"
                            onclick={Callback::from(move |_| on_select.emit(con_id.to_string()))}
                        >
                            if let Some(last_msg) = con.messages.last() {
                                <div class="text-xs opacity-70 truncate max-w-full">
                                    { &last_msg.content }
                                </div>
                            } else {
                                <div class="text-xs opacity-50 italic">
                                    { t("sidebar.empty_conversation") }
                                </div>
                            }
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
