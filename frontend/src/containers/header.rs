use crate::{
    components::{
        header_nav_item::HeaderNavItem, language_selector::LanguageSelector,
        theme_switcher::ThemeSwitcher, user_dropdown::UserDropdown,
    },
    routes::{AdminRoute, AppRoute, MainRoute},
};
use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_router::prelude::Link;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    #[prop_or_default]
    pub current_route: Option<AppRoute>,
    #[prop_or_default]
    pub header_routes: Option<Vec<AppRoute>>,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let (i18n, ..) = use_translation();

    html! {
        <nav class="navbar justify-between bg-base-300">
            <a class="btn btn-ghost text-lg">
                <Link<MainRoute> to={MainRoute::Home} classes="text-lg">
                    {i18n.t("app.title")}
                </Link<MainRoute>>
            </a>
            <div class="dropdown dropdown-end sm:hidden">
                <button class="btn btn-soft">
                <i class="fa-solid fa-bars text-lg"></i>
                </button>
                <ul
                tabindex="0"
                class="dropdown-content menu z-[1] bg-base-200 p-6 rounded-box shadow w-56 gap-2"
                >
                { if let Some(routes) = &props.header_routes {
                    html! {
                        { for routes.iter().map(|route| html! {
                            if let AppRoute::Admin(admin_route) = route {
                                <HeaderNavItem<AdminRoute>
                                    current_route={props.current_route.clone()}
                                    route={admin_route.clone()}
                                />
                            } else if let AppRoute::Main(main_route) = route {
                                <HeaderNavItem<MainRoute>
                                    current_route={props.current_route.clone()}
                                    route={main_route.clone()}
                                />
                            }
                        })}
                    }
                } else {
                    html!{}
                }}
                </ul>
            </div>
            <ul class="hidden menu sm:menu-horizontal">
                { if let Some(routes) = &props.header_routes {
                    html! {
                        { for routes.iter().map(|route| html! {
                            if let AppRoute::Admin(admin_route) = route {
                                <HeaderNavItem<AdminRoute>
                                    current_route={props.current_route.clone()}
                                    route={admin_route.clone()}
                                />
                            } else if let AppRoute::Main(main_route) = route {
                                <HeaderNavItem<MainRoute>
                                    current_route={props.current_route.clone()}
                                    route={main_route.clone()}
                                />
                            }
                        })}
                    }
                } else {
                    html!{}
                }}
            </ul>
            <div class="hidden sm:flex">
                <LanguageSelector />
                <ThemeSwitcher />
                <UserDropdown />
            </div>
        </nav>
    }
}
