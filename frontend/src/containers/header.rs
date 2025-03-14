use crate::components::language_selector::LanguageSelector;
use crate::components::theme_switcher::ThemeSwitcher;
use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::{Html, Properties, classes, function_component, html};
use yew_icons::{Icon, IconId};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

use crate::routes::Routes;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    #[prop_or_default]
    pub current_route: Option<Routes>,
}

#[function_component(Header)]
pub fn header(_props: &HeaderProps) -> Html {
    // Initialize translations

    let (i18n, ..) = use_translation();
    let notification_count = 15; // Example notification count matching DashWind

    html! {
      <>
        <div class="navbar sticky top-0 z-10 shadow-sm border-b border-base-200 bg-base-100">
            <div class="flex-1">
                <label htmlFor="left-sidebar-drawer" class="btn btn-square btn-ghost drawer-button lg:hidden">
                    <Icon icon_id={IconId::HeroiconsSolidBars3} class="h-5 w-5"/>
                </label>
                {
                    // Page title could be dynamic based on active route
                    _props.current_route.as_ref().map_or_else(
                        || html! { <h1 class="text-2xl font-semibold ml-2">{i18n.t("sidebar.dashboard")}</h1> },
                        |route| html! { <h1 class="text-2xl font-semibold ml-2">{format!("{:?}", route)}</h1> }
                    )
                }
            </div>
            <div class="flex-none flex items-center gap-2">
                // Language Selector
                <LanguageSelector />

                // Theme Switcher
                <ThemeSwitcher />

                // Notifications dropdown
                <div class="dropdown dropdown-end">
                    <label tabIndex={0} class="btn btn-ghost btn-circle">
                        <div class="indicator">
                            <Icon icon_id={IconId::HeroiconsSolidBell} class="h-5 w-5"/>
                            <span class="indicator-item badge badge-primary badge-sm">{notification_count}</span>
                        </div>
                    </label>
                    <div tabIndex={0} class="mt-3 z-[1] card card-compact dropdown-content w-80 bg-base-100 shadow">
                        <div class="card-body">
                            <div class="flex justify-between">
                                <span class="font-bold text-lg">{notification_count} {" "}{i18n.t("notifications.title")}</span>
                                <a class="text-info text-sm">{i18n.t("header.mark_all_read")}</a>
                            </div>
                            <div class="divider my-1"></div>

                            // Recent notifications
                            <div class="flex flex-col gap-3 max-h-96 overflow-y-auto">
                                // Example notification
                                <div class="flex gap-3 items-start">
                                    <div class="avatar">
                                        <div class="w-10 rounded-full bg-primary text-primary-content flex items-center justify-center">
                                            <span>{"JD"}</span> // TODO: Add to translations
                                        </div>
                                    </div>
                                    <div class="flex-1">
                                <p class="text-sm font-medium">{i18n.t("header.user_notification")}</p>
                                <p class="text-xs text-base-content/70">{i18n.t("header.time_recent")}</p>
                                    </div>
                                </div>

                                <div class="flex gap-3 items-start">
                                    <div class="avatar">
                                        <div class="w-10 rounded-full bg-secondary text-secondary-content flex items-center justify-center">
                                            <Icon icon_id={IconId::HeroiconsSolidBell} class="h-5 w-5" />
                                        </div>
                                    </div>
                                    <div class="flex-1">
                                        <p class="text-sm font-medium">{i18n.t("header.transaction_notification")}</p>
                                        <p class="text-xs text-base-content/70">{i18n.t("header.time_earlier")}</p>
                                    </div>
                                </div>
                            </div>

                            <div class="card-actions mt-2">
                                <button class="btn btn-primary btn-sm btn-block">{i18n.t("header.view_all_notifications")}</button>
                            </div>
                        </div>
                    </div>
                </div>

                // User profile dropdown
                <div class="dropdown dropdown-end">
                    <label tabIndex={0} class="btn btn-ghost btn-circle avatar">
                        <div class="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center">
                            <span class="text-primary font-semibold">{"AD"}</span>
                        </div>
                    </label>
                    <ul tabIndex={0} class="menu menu-sm dropdown-content mt-3 p-2 shadow bg-base-100 rounded-box w-52 z-[1]">
                        <li>
                            <a class="justify-between">
                                {i18n.t("header.profile")}
                                <span class="badge badge-accent badge-sm">{i18n.t("header.new_badge")}</span>
                            </a>
                        </li>
                        <li><a>{i18n.t("sidebar.settings")}</a></li>
                        <div class="divider my-1"></div>
                        <li><a>{i18n.t("header.logout")}</a></li>
                    </ul>
                </div>
            </div>
        </div>
      </>
    }
}
