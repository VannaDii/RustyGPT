use i18nrs::yew::use_translation;
use std::ops::Deref;
use yew::use_state;
use yew::{Callback, function_component, html};

use crate::components::language_selector_button::LanguageSelectorButton;
use crate::language;

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

    let lang_code = language_state.deref();
    let lang_info = language::get_language_info(lang_code.as_str()).unwrap();
    let active_lang_flag = lang_info.flag;
    let supported = language::supported_languages();
    let mut languages: Vec<_> = supported.iter().collect();
    languages.sort_by(|a, b| a.1.native_name.cmp(b.1.native_name));

    html! {
        <div class="dropdown dropdown-end">
            <div tabindex="0" role="button" class="btn btn-ghost btn-circle mb-1">
                <span>{active_lang_flag}</span>
            </div>
            <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-200 rounded-box w-52">
            {
                for languages.into_iter().map(|(_, info)| {
                    html! {
                        <LanguageSelectorButton
                            is_active={info.code == lang_code}
                            info={info.clone()}
                            on_click={on_click.clone()}
                        />
                    }
                })
            }
            </ul>
        </div>
    }
}
