use crate::api::RustyGPTClient;
use crate::containers::setup::Setup;
use crate::models::app_state::AppState;
use crate::pages::login::LoginPage;
use crate::routes::MainRoute;
use reqwest::StatusCode;
use shared::models::{LoginResponse, MeResponse};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::suspense::Suspense;
use yew::{Callback, Html, function_component, html, use_effect_with, use_state};
use yew_router::prelude::*;
use yewdux::prelude::use_store;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[function_component(App)]
pub fn app() -> Html {
    let (_store_state, store_dispatch) = use_store::<AppState>();
    let app_state = use_state(|| None::<AppState>);

    {
        let app_state_handle = app_state.clone();
        let store_dispatch_handle = store_dispatch.clone();
        use_effect_with((), move |_| {
            let app_state_handle = app_state_handle.clone();
            spawn_local(async move {
                let client = RustyGPTClient::shared();
                let is_setup = client
                    .get_setup()
                    .await
                    .map(|response| response.is_setup)
                    .unwrap_or(false);

                if !is_setup {
                    let state = AppState {
                        is_setup: Some(false),
                        ..Default::default()
                    };
                    app_state_handle.set(Some(state.clone()));
                    store_dispatch_handle.set(state);
                    return;
                }

                match client.get_profile().await {
                    Ok(MeResponse { user, session }) => {
                        let state = AppState {
                            is_setup: Some(true),
                            user: Some(user),
                            session: Some(session),
                            csrf_token: client.current_csrf_token(),
                        };
                        app_state_handle.set(Some(state.clone()));
                        store_dispatch_handle.set(state);
                    }
                    Err(err) => {
                        let unauthorized = err
                            .status()
                            .map(|status| status == StatusCode::UNAUTHORIZED)
                            .unwrap_or(false);
                        if unauthorized {
                            client.set_csrf_token(None);
                        }
                        let state = if unauthorized {
                            AppState {
                                is_setup: Some(true),
                                user: None,
                                session: None,
                                csrf_token: None,
                            }
                        } else {
                            AppState {
                                is_setup: Some(true),
                                csrf_token: client.current_csrf_token(),
                                ..Default::default()
                            }
                        };
                        app_state_handle.set(Some(state.clone()));
                        store_dispatch_handle.set(state);
                    }
                }
            });
            || ()
        });
    }

    let logout_callback = {
        let state_setter = app_state.clone();
        let logout_dispatch = store_dispatch.clone();
        Callback::from(move |_| {
            let client = RustyGPTClient::shared();
            client.set_csrf_token(None);
            let state = AppState {
                is_setup: Some(true),
                user: None,
                session: None,
                csrf_token: None,
            };
            state_setter.set(Some(state.clone()));
            logout_dispatch.set(state);
        })
    };

    html! {
        <Suspense fallback={ html!{ <crate::components::loading::Loading/> } }>
            {
                match *app_state {
                    None => html!{ /* Pending API response */ },
                    Some(ref state) if state.is_setup == Some(false) => html!{
                        <Setup />
                    },
                    Some(ref state) if state.is_setup == Some(true) && state.user.is_none() => {
                        let on_success = {
                            let state_setter = app_state.clone();
                            let store_dispatch = store_dispatch;
                            Callback::from(move |login: LoginResponse| {
                                let client = RustyGPTClient::shared();
                                let LoginResponse {
                                    user,
                                    session,
                                    csrf_token,
                                } = login;
                                client.set_csrf_token(Some(csrf_token.clone()));
                                let state = AppState {
                                    is_setup: Some(true),
                                    user: Some(user),
                                    session: Some(session),
                                    csrf_token: Some(csrf_token),
                                };
                                state_setter.set(Some(state.clone()));
                                store_dispatch.set(state);
                            })
                        };
                        html! { <LoginPage {on_success} /> }
                    }
                    Some(ref state) if state.is_setup == Some(true) => {
                        html! {
                            <BrowserRouter>
                                <Switch<MainRoute> render={move |route| crate::routes::switch_with_logout(route, logout_callback.clone())} />
                            </BrowserRouter>
                        }
                    },
                    _ => html!{}
                }
            }
        </Suspense>
    }
}
