mod app;
mod components;
mod models;

use app::App;
use serde_json::Value;
use std::collections::HashMap;
use std::rc::Rc;
use yew::Renderer;
use yew::{ContextProvider, Html, function_component, html, use_state_eq};

// Define our own YewI18n struct
#[derive(Clone, PartialEq)]
pub struct YewI18n {
    translations: HashMap<String, Value>,
    language: String,
}

impl YewI18n {
    pub fn new(translations: HashMap<String, Value>, language: String) -> Self {
        Self {
            translations,
            language,
        }
    }

    pub fn translate(&self, key: &str) -> String {
        if let Some(translations) = self.translations.get(&self.language) {
            if let Some(value) = translations.get(key) {
                if let Some(text) = value.as_str() {
                    return text.to_string();
                }
            }
        }
        key.to_string()
    }

    pub fn set_language(&mut self, language: &str) {
        self.language = language.to_string();
    }

    pub fn language(&self) -> String {
        self.language.clone()
    }
}

fn main() {
    Renderer::<AppWithI18n>::new().render();
}

#[function_component(AppWithI18n)]
fn app_with_i18n() -> Html {
    let mut translations = HashMap::new();

    // English translations
    let en_json = include_str!("../translations/en.json");
    let en_translations: Value = serde_json::from_str(en_json).unwrap();
    translations.insert("en".to_string(), en_translations);

    // Spanish translations
    let es_json = include_str!("../translations/es.json");
    let es_translations: Value = serde_json::from_str(es_json).unwrap();
    translations.insert("es".to_string(), es_translations);

    let i18n = use_state_eq(|| Rc::new(YewI18n::new(translations, "en".to_string())));

    html! {
        <ContextProvider<Rc<YewI18n>> context={(*i18n).clone()}>
            <App />
        </ContextProvider<Rc<YewI18n>>>
    }
}
