mod api;
mod app;
mod components;
mod containers;
mod language;
mod models;
mod pages;
mod routes;

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

    let config = I18nProviderConfig {
        translations,
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
    // Disable truncation of panic payloads to debug any panics
    std::panic::set_hook(Box::new(|info| {
        if let Some(s) = info.payload().downcast_ref::<String>() {
            web_sys::console::log_1(&format!("Panic: {}", s).into());
        } else if let Some(s) = info.payload().downcast_ref::<&str>() {
            web_sys::console::log_1(&format!("Panic: {}", s).into());
        } else {
            web_sys::console::log_1(&"Unknown panic".into());
        }
        if let Some(location) = info.location() {
            web_sys::console::log_1(
                &format!(
                    "  at {}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
                .into(),
            );
        }
    }));

    web_sys::console::log_1(&"Starting Rusty GPT Application".into());

    // Mount the app to the element with id="app"
    Renderer::<InternationalApp>::with_root(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_elements_by_tag_name("body")
            .item(0)
            .unwrap(),
    )
    .render();
}
