mod app;
mod components;
mod models;
mod routes;
mod containers;
mod language;

use app::App;
use i18nrs::yew::I18nProvider;
use i18nrs::yew::I18nProviderConfig;
use std::collections::HashMap;
use yew::Renderer;
use yew::{Html, function_component, html};
use language::{supported_languages, LanguageInfo};


#[function_component(InternationalApp)]
fn international_app() -> Html {
    let translations: HashMap<&str, &str> = supported_languages()
        .iter()
        .map(|(&key, value)| (key, value.translation))
        .collect();
    let languages: Vec<&str> = translations
        .iter()
        .map(|(&key, _)| key)
        .collect();

    let config = I18nProviderConfig {
        translations,
        languages,
        default_language: "en".to_string(),
        ..Default::default()
    };

    html! {
        <I18nProvider ..config>
            <App />
        </I18nProvider>
    }
}

fn main() {
    Renderer::<InternationalApp>::new().render();
}
