use shared::models::ThreadSummary;
use std::collections::HashMap;
use uuid::Uuid;
use yew::{Callback, Html, Properties, classes, function_component, html};

#[derive(Properties, PartialEq)]
pub struct ThreadListProps {
    pub threads: Vec<ThreadSummary>,
    #[prop_or(None)]
    pub selected: Option<Uuid>,
    pub on_select: Callback<Uuid>,
    #[prop_or_default]
    pub unread_counts: HashMap<Uuid, i64>,
}

#[function_component(ThreadList)]
pub fn thread_list(props: &ThreadListProps) -> Html {
    if props.threads.is_empty() {
        return html! {
            <div class="p-4 text-sm text-base-content/70">
                {"No threads yet. Start a new conversation to begin."}
            </div>
        };
    }

    html! {
        <ul class="divide-y divide-base-300">
            { for props.threads.iter().map(|thread| {
                let is_selected = props.selected.is_some_and(|id| id == thread.root_id);
                let summary = thread.clone();
                let on_select = props.on_select.clone();
                let unread = props.unread_counts.get(&summary.root_id).copied().unwrap_or(0);
                let class = if is_selected {
                    classes!("p-3", "bg-base-300", "cursor-pointer")
                } else {
                    classes!("p-3", "hover:bg-base-200", "cursor-pointer")
                };
                html! {
                    <li
                        class={class}
                        onclick={Callback::from(move |_| on_select.emit(summary.root_id))}
                    >
                        <div class="text-sm font-medium text-base-content">{ summary.root_excerpt.clone() }</div>
                        <div class="text-xs text-base-content/70 mt-1">
                            { format!("Updated {}", summary.last_activity_at.0.format("%Y-%m-%d %H:%M")) }
                        </div>
                        <div class="text-xs text-base-content/50 mt-1">
                            { format!("Messages: {} · Participants: {}", summary.message_count, summary.participant_count) }
                            {
                                if unread > 0 {
                                    html! { <span class="badge badge-primary badge-sm ml-2">{ unread }</span> }
                                } else {
                                    Html::default()
                                }
                            }
                        </div>
                    </li>
                }
            })}
        </ul>
    }
}
