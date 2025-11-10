use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::{Html, function_component, html};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// `ErrorPage` page component
#[function_component(ErrorPage)]
pub fn error_page() -> Html {
    let (_i18n, _) = use_translation();

    html! {
        <div class="p-4 space-y-6">
            <h1 class="text-2xl font-bold">{ "Error Page" }</h1>
            <p>{ "This is the error page." }</p>
            <p>{ "Errors will be shown here." }</p>
        </div>
    }
}
