use i18nrs::yew::use_translation;
use std::ops::Deref;
use yew::use_state;
use yew::{Callback, function_component, html};

#[function_component(LanguageSelector)]
pub fn language_selector() -> yew::Html {
    let (_i18n, set_language) = use_translation();
    let language_state: yew::UseStateHandle<String> = use_state(|| "en".to_string());
    let on_click = {
        let language_state = language_state.clone();
        Callback::from(move |value: String| {
            let value_clone = value.clone();
            language_state.set(value_clone);
            set_language.emit(value);
        })
    };

    let current_language = language_state.deref();
    let active_lang_flag = match current_language.as_str() {
        "en" => "🇺🇸",
        "es" => "🇪🇸",
        _ => "🌐",
    };
    let active_lang = match current_language.as_str() {
        "en" => "English",
        "es" => "Español",
        _ => "Language",
    };
    html! {
        <div class="dropdown dropdown-end">
            <label tabindex="0" class="btn btn-ghost btn-sm gap-1 normal-case">
                <span>{active_lang_flag}</span>
                <span class="hidden md:inline">{active_lang}</span>
                <i class="fas fa-chevron-down text-xs opacity-60"></i>
            </label>
            <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-200 rounded-box w-52">
                <li>
                    <button
                        class={if current_language == "en" { "active" } else { "" }}
                        value="en"
                        onclick={ {
                            let on_click = on_click.clone();
                            move |_| on_click.emit(String::from("en"))
                        }}>
                        <span>{"🇺🇸"}</span>
                        <span>{"English"}</span>
                    </button>
                </li>
                <li>
                    <button
                        class={if current_language == "es" { "active" } else { "" }}
                        onclick={ {
                            let on_click = on_click.clone();
                            move |_| on_click.emit(String::from("es"))
                        }}>
                        <span>{"🇪🇸"}</span>
                        <span>{"Español"}</span>
                    </button>
                </li>
            </ul>
        </div>
    }
}
