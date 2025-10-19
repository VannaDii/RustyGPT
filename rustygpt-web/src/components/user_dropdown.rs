use crate::{
    api::RustyGPTClient,
    models::app_state::AppState,
    routes::{AdminRoute, MainRoute},
};
use i18nrs::yew::use_translation;
use reqwest::StatusCode;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::hooks::use_navigator;
use yewdux::prelude::use_selector;

#[derive(yew::Properties, PartialEq)]
pub struct UserDropdownProps {
    #[prop_or_default]
    pub on_logout: Option<Callback<()>>,
}

#[function_component(UserDropdown)]
pub fn user_dropdown(props: &UserDropdownProps) -> Html {
    let navigator = use_navigator().unwrap();
    let (i18n, ..) = use_translation();
    let user_state = use_selector(|state: &AppState| state.user.clone());
    let Some(user) = (*user_state).clone() else {
        return html! {};
    };

    let settings_button = {
        let settings_navigator = navigator.clone();
        let onclick = Callback::from(move |event: yew::MouseEvent| {
            event.prevent_default();
            settings_navigator.push(&AdminRoute::Profile)
        });
        html! {
            <li><a {onclick}>{i18n.t("sidebar.settings")}</a></li>
        }
    };

    let logout_button = {
        let navigator = navigator;
        let on_logout = props.on_logout.clone();
        let onclick = Callback::from(move |event: yew::MouseEvent| {
            event.prevent_default();
            let navigator = navigator.clone();
            let on_logout = on_logout.clone();
            spawn_local(async move {
                let client = RustyGPTClient::shared();
                let result = client.logout().await;
                if let Err(err) = result {
                    if err
                        .status()
                        .map(|status| status != StatusCode::UNAUTHORIZED)
                        .unwrap_or(true)
                    {
                        log::error!("logout failed: {}", err);
                    }
                }
                if let Some(callback) = on_logout {
                    callback.emit(());
                }
                navigator.push(&MainRoute::Login);
            });
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
                <li class="px-2 py-1 text-left">
                    <div class="text-sm font-semibold text-base-content">{ user.display_name.clone().unwrap_or_else(|| user.username.clone()) }</div>
                    <div class="text-xs text-base-content/70">{ &user.email }</div>
                </li>
                <div class="divider my-0"></div>
                {settings_button}
                <div class="divider my-0"></div>
                {logout_button}
            </ul>
        </div>
    }
}
