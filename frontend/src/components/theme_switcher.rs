use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use web_sys::window;
use yew::{
    Callback, Classes, Html, Properties, classes, function_component, html, use_effect_with,
    use_state,
};
use yew_icons::{Icon, IconId};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

const THEMES: [&str; 5] = ["dark", "light", "cupcake", "cyberpunk", "business"];

#[derive(Properties, PartialEq)]
pub struct ThemeSwitcherProps {
    #[prop_or_default]
    pub class: Classes,
}

#[function_component(ThemeSwitcher)]
pub fn theme_switcher(props: &ThemeSwitcherProps) -> Html {
    // Initialize theme state
    let (_i18n, ..) = use_translation();
    let current_theme = use_state(|| "dark".to_string());
    let dropdown_open = use_state(|| false);

    // Get the current theme from HTML attribute on component mount
    {
        let current_theme = current_theme.clone();
        use_effect_with((), move |_| {
            if let Some(window) = window() {
                if let Some(document) = window.document() {
                    if let Some(html_element) = document.document_element() {
                        if let Some(theme) = html_element.get_attribute("data-theme") {
                            if !theme.is_empty() {
                                current_theme.set(theme);
                            }
                        }
                    }
                }
            }
            || {}
        });
    }

    // Function to change the theme
    let change_theme = {
        let current_theme = current_theme.clone();
        let dropdown_open = dropdown_open.clone();

        Callback::from(move |theme: String| {
            // Update theme state
            current_theme.set(theme.clone());

            // Close dropdown
            dropdown_open.set(false);

            // Apply theme to HTML element
            if let Some(window) = window() {
                if let Some(document) = window.document() {
                    if let Some(html_element) = document.document_element() {
                        let _ = html_element.set_attribute("data-theme", &theme);
                    }
                }
            }
        })
    };

    // Toggle dropdown open/closed
    let toggle_dropdown = {
        let dropdown_open = dropdown_open.clone();

        Callback::from(move |_| {
            dropdown_open.set(!*dropdown_open);
        })
    };

    let theme_icon = match current_theme.as_str() {
        "light" => IconId::HeroiconsSolidSun,
        "dark" => IconId::HeroiconsSolidMoon,
        _ => IconId::HeroiconsSolidSun,
    };

    html! {
        <div class={classes!("dropdown", "dropdown-end", props.class.clone())}>
            <button
                class="btn btn-ghost btn-circle"
                onclick={toggle_dropdown}
                aria-label={_i18n.t("theme.selector")}
            >
                <Icon icon_id={theme_icon} class="h-5 w-5" />
            </button>

            if *dropdown_open {
                <ul class="dropdown-content z-[1] mt-2 p-2 shadow-lg bg-base-200 rounded-box w-52">
                    <li class="menu-title text-xs font-semibold uppercase pl-4 pt-2">{_i18n.t("theme.title")}</li>
                    {
                        THEMES.iter().map(|theme| {
                            let theme_str = theme.to_string();
                            let is_active = *current_theme == theme_str;
                            let theme_change = {
                                let theme_str = theme_str.clone();
                                let change_theme = change_theme.clone();

                                Callback::from(move |_| {
                                    change_theme.emit(theme_str.clone());
                                })
                            };

                            html! {
                                <li>
                                    <button
                                        class={classes!(
                                            "flex",
                                            "items-center",
                                            "gap-4",
                                            "pl-4",
                                            "py-2",
                                            "hover:bg-base-300",
                                            "rounded-lg",
                                            "transition-colors",
                                            if is_active { "font-medium text-primary" } else { "" }
                                        )}
                                        onclick={theme_change}
                                    >
                                        <div class="flex-grow capitalize">
                                            {_i18n.t(&format!("theme.{}", theme))}
                                        </div>
                                        if is_active {
                                            <Icon icon_id={IconId::HeroiconsOutlineCheck} class="h-4 w-4 text-primary" />
                                        }
                                    </button>
                                </li>
                            }
                        }).collect::<Html>()
                    }
                </ul>
            }
        </div>
    }
}
