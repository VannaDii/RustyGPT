use crate::{api::RustyGPTClient, routes::MainRoute};
use reqwest::StatusCode;
use shared::models::{LoginRequest, LoginResponse};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::hooks::use_navigator;

#[derive(yew::Properties, PartialEq)]
pub struct LoginPageProps {
    #[prop_or_default]
    pub on_success: Option<Callback<LoginResponse>>,
}

#[function_component(LoginPage)]
pub fn login_page(props: &LoginPageProps) -> Html {
    let email = use_state(String::new);
    let password = use_state(String::new);
    let error = use_state(|| None::<String>);
    let loading = use_state(|| false);
    let navigator = use_navigator();

    let onsubmit = {
        let email_handle = email.clone();
        let password_handle = password.clone();
        let error_handle = error.clone();
        let loading_handle = loading.clone();
        let on_success = props.on_success.clone();
        let navigator = navigator;
        Callback::from(move |event: SubmitEvent| {
            event.prevent_default();
            let email_value = (*email_handle).clone();
            let password_value = (*password_handle).clone();
            loading_handle.set(true);
            error_handle.set(None);
            let on_success_cb = on_success.clone();
            let loading_ref = loading_handle.clone();
            let error_ref = error_handle.clone();
            let navigator_handle = navigator.clone();
            spawn_local(async move {
                let client = RustyGPTClient::shared();
                let request = LoginRequest {
                    email: email_value,
                    password: password_value,
                };
                match client.login(&request).await {
                    Ok(response) => {
                        if let Some(ref nav) = navigator_handle {
                            nav.push(&MainRoute::Home);
                        }
                        if let Some(callback) = on_success_cb {
                            callback.emit(response);
                        }
                    }
                    Err(err) => {
                        let message = err.status().map_or_else(
                            || "Unable to connect to server".to_string(),
                            |status| match status {
                                StatusCode::UNAUTHORIZED => "Invalid credentials".to_string(),
                                _ => format!("Login failed: {status}"),
                            },
                        );
                        error_ref.set(Some(message));
                    }
                }
                loading_ref.set(false);
            });
        })
    };

    let on_email_change = {
        let email = email.clone();
        Callback::from(move |event: InputEvent| {
            if let Some(input) = event.target_dyn_into::<HtmlInputElement>() {
                email.set(input.value());
            }
        })
    };

    let on_password_change = {
        let password = password.clone();
        Callback::from(move |event: InputEvent| {
            if let Some(input) = event.target_dyn_into::<HtmlInputElement>() {
                password.set(input.value());
            }
        })
    };

    let is_busy = *loading;
    let disable_submit = (*email).is_empty() || (*password).is_empty() || is_busy;

    html! {
        <div class="flex items-center justify-center min-h-screen bg-base-200">
            <div class="card w-full max-w-md shadow-lg bg-base-100">
                <form class="card-body" onsubmit={onsubmit}>
                    <h2 class="card-title text-2xl">{"Sign in"}</h2>
                    if let Some(message) = &*error {
                        <div class="alert alert-error">
                            <span>{message.clone()}</span>
                        </div>
                    }
                    <div class="form-control">
                        <label class="label" for="email">
                            <span class="label-text">{"Email"}</span>
                        </label>
                        <input
                            id="email"
                            class="input input-bordered"
                            type="email"
                            required=true
                            value={(*email).clone()}
                            oninput={on_email_change}
                        />
                    </div>
                    <div class="form-control">
                        <label class="label" for="password">
                            <span class="label-text">{"Password"}</span>
                        </label>
                        <input
                            id="password"
                            class="input input-bordered"
                            type="password"
                            required=true
                            value={(*password).clone()}
                            oninput={on_password_change}
                        />
                    </div>
                    <div class="form-control mt-6">
                        <button class="btn btn-primary" type="submit" disabled={disable_submit}>
                            {if is_busy { "Signing in..." } else { "Sign in" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
