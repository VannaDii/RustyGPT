use yew::{Callback, Html, Properties, function_component, html};

use crate::LanguageInfo;

#[derive(Properties, PartialEq)]
pub struct LanguageSelectorButtonProps {
    pub is_active: bool,
    pub info: LanguageInfo,
    pub on_click: Callback<String>,
}

#[function_component(LanguageSelectorButton)]
pub fn language_selector(props: &LanguageSelectorButtonProps) -> Html {
    let info = &props.info;
    let code = info.code.to_string();
    let on_click = props.on_click.clone();
    html! {
        <li>
            <a
                class={if props.is_active { "active" } else { "" }}
                onclick={move |event: yew::MouseEvent| {
                    event.prevent_default();
                    on_click.emit(code.clone());
                }}>
                <span>{props.info.flag}</span>
                <span>{props.info.native_name}</span>
            </a>
        </li>
    }
}
