use crate::{
    components::{
        language_selector::LanguageSelector, theme_switcher::ThemeSwitcher,
        user_dropdown::UserDropdown,
    },
    routes::AppRoute,
};
use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_router::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

use crate::routes::MainRoute;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    #[prop_or_default]
    pub current_route: Option<AppRoute>,
}

#[function_component(Header)]
pub fn header(_props: &HeaderProps) -> Html {
    let (i18n, ..) = use_translation();
    let route_title = i18n.t(&format!("{:?}", _props.current_route.as_ref().unwrap())
        .to_lowercase()
        .replace("(", ".")
        .replace(")", ""));

    html! {
      <>
        <div class="navbar sticky top-0 z-10 shadow-sm border-b border-base-200 bg-base-100">
            <div class="flex-1">
                <Link<MainRoute> to={MainRoute::Home} classes="btn btn-square drawer-button lg:hidden">
                    <img src="/public/logo_46x46.png" class="h-5 w-5"/>
                </Link<MainRoute>>
                {route_title}
            </div>
            <div class="flex-none flex items-center gap-2">
                <LanguageSelector />
                <ThemeSwitcher />
                <UserDropdown />
            </div>
        </div>
      </>
    }
}
