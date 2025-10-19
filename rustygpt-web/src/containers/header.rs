use crate::{
    components::{
        header_nav_item::HeaderNavItem, language_selector::LanguageSelector,
        theme_switcher::ThemeSwitcher, user_dropdown::UserDropdown,
    },
    models::app_state::AppState,
    routes::{AdminRoute, AppRoute, MainRoute},
};
use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_router::prelude::Link;
use yewdux::prelude::use_selector;

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
    #[prop_or_default]
    pub on_logout: Option<Callback<()>>,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let (i18n, ..) = use_translation();
    let user = use_selector(|state: &AppState| state.user.clone());
    let user_opt = (*user).clone();
    let is_authenticated = user_opt.is_some();

    let render_routes = |routes: &[AppRoute]| -> Html {
        html! {
            { for routes.iter().map(|route| match route {
                AppRoute::Admin(admin_route) => html! {
                    <HeaderNavItem<AdminRoute>
                        current_route={props.current_route.clone()}
                        route={admin_route.clone()}
                    />
                },
                AppRoute::Main(main_route) => html! {
                    <HeaderNavItem<MainRoute>
                        current_route={props.current_route.clone()}
                        route={main_route.clone()}
                    />
                },
            }) }
        }
    };

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
                {
                    props
                        .header_routes
                        .as_ref()
                        .map_or_else(|| html! {}, |routes| render_routes(routes))
                }
                </ul>
            </div>
            <ul class="hidden menu sm:menu-horizontal">
                {
                    props
                        .header_routes
                        .as_ref()
                        .map_or_else(|| html! {}, |routes| render_routes(routes))
                }
            </ul>
            <div class="hidden sm:flex">
                <LanguageSelector />
                <ThemeSwitcher />
                {
                    user_opt.as_ref().map_or_else(
                        || html! {
                            <Link<MainRoute> to={MainRoute::Login} classes="btn btn-primary btn-sm">
                                {i18n.t("header.login")}
                            </Link<MainRoute>>
                        },
                        |user| html! {
                            <>
                                <span class="text-sm text-base-content/80 mr-2">{ &user.username }</span>
                                <UserDropdown on_logout={props.on_logout.clone()} />
                            </>
                        },
                    )
                }
            </div>
            <div class="sm:hidden flex items-center gap-2">
                {
                    if is_authenticated {
                        html! { <UserDropdown on_logout={props.on_logout.clone()} /> }
                    } else {
                        html! {
                            <Link<MainRoute> to={MainRoute::Login} classes="btn btn-ghost btn-sm">
                                <i class="fa-solid fa-right-to-bracket text-lg"></i>
                            </Link<MainRoute>>
                        }
                    }
                }
            </div>
        </nav>
    }
}
