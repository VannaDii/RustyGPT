use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::{Html, function_component, html};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// SettingsPage page component
#[function_component(SettingsPage)]
pub fn settings_page() -> Html {
    let (_i18n, _) = use_translation();

    html! {
        <div class="p-4 space-y-6">
            <h1 class="text-2xl font-bold">{ "Profile Page" }</h1>
            <p>{ "This is the profile page." }</p>
            <p>{ "You can edit your profile here." }</p>
        </div>
    }
}
