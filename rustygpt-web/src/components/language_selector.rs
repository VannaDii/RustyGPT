use i18nrs::yew::use_translation;
use yew::use_state_eq; // Changed to use_state_eq for better performance
use yew::{Callback, function_component, html, use_effect_with};

use crate::components::language_selector_button::LanguageSelectorButton;
use crate::language;

#[function_component(LanguageSelector)]
pub fn language_selector() -> yew::Html {
    let (i18n, set_language) = use_translation();
    // Initialize with the current language from i18n instead of hardcoded "en"
    let language_state = use_state_eq(|| i18n.get_current_language().to_string());

    // Clone language_state for use in the effect to avoid ownership issues
    let language_state_for_effect = language_state.clone();

    // Update local state when i18n language changes
    use_effect_with(i18n, move |i18n| {
        language_state_for_effect.set(i18n.get_current_language().to_string());
        || ()
    });

    let on_click = {
        let language_state = language_state.clone();
        Callback::from(move |value: String| {
            language_state.set(value.clone());
            // Ensure language change is properly triggered
            set_language.emit(value);
        })
    };

    let lang_code = &*language_state;
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
