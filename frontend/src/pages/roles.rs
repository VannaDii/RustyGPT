use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::{Html, function_component, html};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// SettingsPage page component
#[function_component(RolesPage)]
pub fn roles_page() -> Html {
    let (_i18n, _) = use_translation();

    html! {
        <div class="p-4 space-y-6">
            <h1 class="text-2xl font-bold">{ "User Roles Page" }</h1>
            <p>{ "This is the user roles page." }</p>
            <p>{ "You can edit user roles and permissions here." }</p>
        </div>
    }
}
