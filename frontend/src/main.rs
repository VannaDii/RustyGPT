mod app;
mod components;
mod containers;
mod language;
mod models;
mod routes;
mod utils;

use app::App;
use i18nrs::yew::I18nProvider;
use i18nrs::yew::I18nProviderConfig;
use language::{LanguageInfo, supported_languages};
use models::app_state::AppState;
use std::collections::HashMap;
use yew::Renderer;
use yew::{Html, function_component, html};
use yewdux::Dispatch;
use yewdux::YewduxRoot;

#[function_component(InternationalApp)]
fn international_app() -> Html {
    let cx = yewdux::Context::new();
    Dispatch::<AppState>::new(&cx).set(AppState::default());
    let translations: HashMap<&str, &str> = supported_languages()
        .iter()
        .map(|(&key, value)| (key, value.translation))
        .collect();
    let languages: Vec<&str> = translations.iter().map(|(&key, _)| key).collect();

    let config = I18nProviderConfig {
        translations,
        languages,
        default_language: "en".to_string(),
        ..Default::default()
    };

    html! {
        <YewduxRoot>
            <I18nProvider ..config>
                <App />
            </I18nProvider>
        </YewduxRoot>
    }
}

fn main() {
    Renderer::<InternationalApp>::new().render();
}
