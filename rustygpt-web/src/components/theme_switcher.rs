use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use web_sys::window;
use yew::{
    Callback, Classes, Html, Properties, function_component, html, use_effect_with, use_state,
};
use yew_icons::{Icon, IconId};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Properties, PartialEq, Eq)]
pub struct ThemeSwitcherProps {
    #[prop_or_default]
    pub class: Classes,
}

#[function_component(ThemeSwitcher)]
pub fn theme_switcher(props: &ThemeSwitcherProps) -> Html {
    // Initialize theme state
    let (i18n, ..) = use_translation();
    let current_theme = use_state(|| "dark".to_string());

    // Get the current theme from system preference or HTML attribute on component mount
    {
        let current_theme = current_theme.clone();
        use_effect_with((), move |()| {
            if let Some(window) = window() {
                let system_prefers_dark = window
                    .match_media("(prefers-color-scheme: dark)")
                    .ok()
                    .flatten()
                    .is_some_and(|media_query| media_query.matches());

                let default_theme = if system_prefers_dark { "dark" } else { "light" };

                if let Some(document) = window.document()
                    && let Some(html_element) = document.document_element()
                {
                    let theme = html_element
                        .get_attribute("data-theme")
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| default_theme.to_string());

                    current_theme.set(theme.clone());
                    let _ = html_element.set_attribute("data-theme", &theme);
                }
            }
            || {}
        });
    }

    // Function to toggle the theme
    let toggle_theme = {
        let current_theme = current_theme.clone();

        Callback::from(move |_: yew::MouseEvent| {
            // Toggle between dark and light
            let new_theme = if *current_theme == "dark" {
                "light"
            } else {
                "dark"
            };

            // Update theme state
            current_theme.set(new_theme.to_string());

            if let Some(window) = window()
                && let Some(document) = window.document()
                && let Some(html_element) = document.document_element()
            {
                let _ = html_element.set_attribute("data-theme", new_theme);
            }
        })
    };

    // Show sun icon in dark mode (to switch to light) and moon icon in light mode (to switch to dark)
    let theme_icon = match current_theme.as_str() {
        "light" => IconId::HeroiconsSolidMoon,
        _ => IconId::HeroiconsSolidSun,
    };

    html! {
        <div class={props.class.clone()}>
            <button
                class="btn btn-ghost btn-circle"
                onclick={toggle_theme}
                aria-label={i18n.t("theme.selector")}
            >
                <Icon icon_id={theme_icon} class="h-5 w-5" />
            </button>
        </div>
    }
}
