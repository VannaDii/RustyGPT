use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use web_sys::window;
use yew::{Html, function_component, html, use_effect_with};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[function_component(Setup)]
pub fn setup() -> Html {
    let (i18n, _) = use_translation();
    // Adds data-theme attribute to html tag for theme support
    use_effect_with((), |_| {
        if let Some(window) = window() {
            if let Some(document) = window.document() {
                if let Some(html_element) = document.document_element() {
                    html_element
                        .set_attribute("data-theme", "dark")
                        .unwrap_or_default();
                }
            }
        }
        || {}
    });

    html! {
      <div class="flex justify-center">
          <div class="flex flex-col items-center text-center gap-6 max-w-xl">
              <span class="text-sm text-accent">{i18n.t("app.title")}</span>
              <h1 class="text-5xl font-bold">{i18n.t("setup.heading")}</h1>
              <span class="">
                  {i18n.t("setup.description")}
              </span>
              <div class="flex gap-4">
                  <a class="btn btn-primary">
                      {"We should"}
                      <i class="fa-solid fa-arrow-right text-sm"></i>
                  </a>
                  <a class="btn btn-neutral">
                      {"Build out this form"}
                      <i class="fa-solid fa-blog"></i>
                  </a>
              </div>
          </div>
      </div>
    }
}
