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
        let onclick = Callback::from(move |_| navigator.push(&AdminRoute::Profile));
        html! {
            <li><button {onclick}>{i18n.t("sidebar.settings")}</button></li>
        }
    };

    let logout_button = {
        let navigator = navigator.clone();
        let onclick = Callback::from(move |_| navigator.push(&MainRoute::Home));
        html! {
            <li><button {onclick}>{i18n.t("header.logout")}</button></li>
        }
    };

    html! {
      <div class="dropdown dropdown-end">
          <label tabIndex={0} class="btn btn-ghost btn-circle avatar">
              <div class="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center">
                  <span class="text-primary font-semibold">{"AD"}</span>
              </div>
          </label>
          <ul tabIndex={0} class="menu menu-sm dropdown-content mt-3 p-2 shadow bg-base-100 rounded-box w-52 z-[1]">
              {settings_button}
              <div class="divider my-1"></div>
              {logout_button}
          </ul>
      </div>
    }
}
