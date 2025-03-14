use i18nrs::yew::use_translation;
use web_sys::window;
use yew::{Html, classes, function_component, html, use_effect, use_effect_with, use_state};
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;
use yewdux::{use_dispatch, use_selector};

use crate::{
    models::{
        header::HeaderState,
        right_sidebar::{RightSidebarAction, RightSidebarBodyType, RightSidebarState},
    },
    routes::Routes,
    utils::local_storage,
};

#[function_component(Header)]
pub fn header() -> Html {
    let (i18n, ..) = use_translation();
    let dispatch_right_sidebar = use_dispatch::<RightSidebarState>();
    let state = use_selector(|header: &HeaderState| header.clone());

    let theme = use_state(|| local_storage::get("theme"));

    let theme_clone = theme.clone();
    use_effect(move || {
        let prefers_dark = window()
            .unwrap()
            .match_media("(prefers-color-scheme: dark)")
            .ok()
            .flatten()
            .map(|m| m.matches())
            .unwrap_or(false);
        let target_theme = if prefers_dark { "dark" } else { "light" };
        theme_clone.set(Some(target_theme.to_string()));
        local_storage::set("theme", target_theme);
        || {}
    });

    let HeaderState {
        page_title,
        notification_count,
        ..
    } = &*state;

    let notification_label = i18n.t("notifications.title");
    let open_notifications = dispatch_right_sidebar.apply_callback(move |_| {
        RightSidebarAction::OpenSidebar(
            RightSidebarBodyType::Notifications,
            None,
            notification_label.clone(),
        )
    });
    let logout = |_| local_storage::clear();
    let logout_label = i18n.t("header.logout");
    let sun_class_augment = if *theme == Some("dark".to_string()) {
        "swap-on"
    } else {
        "swap-off"
    };
    let moon_class_augment = if *theme == Some("light".to_string()) {
        "swap-on"
    } else {
        "swap-off"
    };

    use_effect_with((), |_| {
        move || {
            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                if let Ok(Some(element)) = document.query_selector("[data-set-theme='light']") {
                    let _ = element.set_attribute("data-set-theme", "light");
                    let _ = element.set_attribute("data-act-class", "ACTIVECLASS");
                }
                if let Ok(Some(element)) = document.query_selector("[data-set-theme='dark']") {
                    let _ = element.set_attribute("data-set-theme", "dark");
                    let _ = element.set_attribute("data-act-class", "ACTIVECLASS");
                }
            }
        }
    });

    html! {
      <>
        <div class="navbar sticky top-0 bg-base-100  z-10 shadow-md ">
            <div class="flex-1">
                <label htmlFor="left-sidebar-drawer" class="btn btn-primary drawer-button lg:hidden">
                <Icon icon_id={IconId::HeroiconsSolidBars3} class="h-5 inline-block w-5"/></label>
                <h1 class="text-2xl font-semibold ml-2">{page_title}</h1>
            </div>
            <div class="flex-none ">
                <label class="swap ">
                    <input type="checkbox"/>
                    <Icon
                        icon_id={IconId::HeroiconsSolidSun}
                        class={classes!("fill-current", "w-6", "h-6", sun_class_augment)}/>
                    <Icon
                        icon_id={IconId::HeroiconsSolidMoon}
                        class={classes!("fill-current", "w-6", "h-6", moon_class_augment)}/>
                </label>
                <button class="btn btn-ghost ml-4  btn-circle" onclick={open_notifications}>
                    <div class="indicator">
                        <Icon icon_id={IconId::HeroiconsSolidBell} class="h-6 w-6"/>
                        if *notification_count > 0 {
                            <span class="indicator-item badge badge-secondary badge-sm">{notification_count}</span>
                        }
                    </div>
                </button>
                <div class="dropdown dropdown-end ml-4">
                    <label tabIndex={0} class="btn btn-ghost btn-circle avatar">
                        <div class="w-10 rounded-full">
                            <Icon icon_id={IconId::HeroiconsSolidUser} class="h-6 w-6"/>
                        </div>
                    </label>
                    <ul tabIndex={0} class="menu menu-compact dropdown-content mt-3 p-2 shadow bg-base-100 rounded-box w-52">
                        <li class="justify-between">
                            <Link<Routes> to={Routes::Settings}>
                            {"Profile Settings"}
                            <span class="badge">{"New"}</span>
                            </Link<Routes>>
                        </li>
                        <div class="divider mt-0 mb-0"></div>
                        <li><a onclick={logout}>{logout_label}</a></li>
                    </ul>
                </div>
            </div>
        </div>
        </>
    }
}
