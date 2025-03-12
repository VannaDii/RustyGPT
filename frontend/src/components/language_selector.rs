use i18nrs::yew::use_translation;
use web_sys::HtmlInputElement;
use yew::{Callback, function_component, html, use_node_ref, use_state};

#[function_component(LanguageSelector)]
pub fn language_selector() -> yew::Html {
    let (_i18n, set_language) = use_translation();

    let language_ref = use_node_ref();
    let language_state = use_state(|| "en".to_string());

    let onchange = {
        let language_ref = language_ref.clone();
        let language_state = language_state.clone();
        Callback::from(move |_| {
            if let Some(input) = language_ref.cast::<HtmlInputElement>() {
                let value = input.value();
                language_state.set(value);
                set_language.emit(input.value());
            }
        })
    };

    html! {
        <div class="language-selector">
            <select
                class="select select-bordered select-sm"
                ref={language_ref}
                onchange={onchange}
            >
                <option value="en">{ "🇺🇸 English" }</option>
                <option value="es">{ "🇪🇸 Español" }</option>
            </select>
        </div>
    }
}
