use i18nrs::yew::use_translation;
use yew::{Callback, Html, function_component, html};
use yew_router::hooks::use_navigator;

use crate::routes::{AdminRoute, MainRoute};

#[function_component(UserDropdown)]
pub fn user_dropdown() -> Html {
    let navigator = use_navigator().unwrap();
    let (i18n, ..) = use_translation();

    let settings_button = {
        let navigator = navigator.clone();
        let onclick = Callback::from(move |event: yew::MouseEvent| {
            event.prevent_default();
            navigator.push(&AdminRoute::Profile)
        });
        html! {
            <li><a {onclick}>{i18n.t("sidebar.settings")}</a></li>
        }
    };

    let logout_button = {
        let navigator = navigator.clone();
        let onclick = Callback::from(move |event: yew::MouseEvent| {
            event.prevent_default();
            navigator.push(&MainRoute::Home);
        });
        html! {
            <li><a {onclick}>{i18n.t("header.logout")}</a></li>
        }
    };

    html! {
        <div class="dropdown dropdown-end">
            <div tabindex="0" role="button" class="btn btn-ghost btn-circle mb-1">
                <i class="fa-solid fa-user text-lg"></i>
            </div>
            <ul tabIndex={0} class="dropdown-content z-[1] menu p-2 shadow bg-base-200 rounded-box w-52">
                {settings_button}
                <div class="divider my-0"></div>
                {logout_button}
            </ul>
        </div>
    }
}
