use shared::models::{MessageRole, MessageView, Timestamp};
use yew::{Callback, Html, Properties, classes, function_component, html};

#[derive(Properties, PartialEq, Clone)]
pub struct MessageNodeProps {
    pub message: MessageView,
    pub on_reply: Callback<MessageView>,
}

const fn role_classes(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::User => "bg-primary text-primary-content",
        MessageRole::Assistant => "bg-base-200 text-base-content",
        MessageRole::System => "bg-base-300 text-base-content",
        MessageRole::Tool => "bg-neutral text-neutral-content",
    }
}

const fn role_label(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::User => "User",
        MessageRole::Assistant => "Assistant",
        MessageRole::System => "System",
        MessageRole::Tool => "Tool",
    }
}

fn indent_style(depth: i32) -> String {
    let level = depth.saturating_sub(1) as f32;
    let rem = level * 1.25;
    format!("margin-left: {}rem;", rem)
}

fn format_timestamp(timestamp: &Timestamp) -> String {
    timestamp.0.format("%H:%M:%S").to_string()
}

#[function_component(MessageNode)]
pub fn message_node(props: &MessageNodeProps) -> Html {
    let message = props.message.clone();
    let on_reply = props.on_reply.clone();

    let reply_callback = Callback::from(move |_| {
        on_reply.emit(message.clone());
    });

    let classes = classes!(
        "rounded-xl",
        "px-4",
        "py-3",
        "shadow-sm",
        role_classes(&props.message.role)
    );
    let style = indent_style(props.message.depth);

    html! {
        <div class="mb-3 space-y-1" style={style}>
            <div class="flex items-center gap-2 text-xs text-base-content/70">
                <span class="font-semibold">{ role_label(&props.message.role) }</span>
                <span>{ format_timestamp(&props.message.created_at) }</span>
            </div>
            <div class={classes}>
                { props.message.content.clone() }
            </div>
            <div class="flex items-center gap-2 text-xs">
                <button
                    class="btn btn-ghost btn-xs"
                    type="button"
                    onclick={reply_callback}
                >
                    {"Reply"}
                </button>
            </div>
        </div>
    }
}
