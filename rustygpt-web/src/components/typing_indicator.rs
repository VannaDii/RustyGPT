use yew::{Html, Properties, function_component, html};

#[derive(Properties, PartialEq)]
pub struct TypingIndicatorProps {
    #[prop_or(false)]
    pub active: bool,
    #[prop_or_default]
    pub label: Option<String>,
}

#[function_component(TypingIndicator)]
pub fn typing_indicator(props: &TypingIndicatorProps) -> Html {
    if !props.active {
        return Html::default();
    }

    let label = props
        .label
        .clone()
        .unwrap_or_else(|| "Assistant is typing…".to_string());

    html! {
        <div class="text-xs text-base-content/70 animate-pulse py-2">
            { label }
        </div>
    }
}
