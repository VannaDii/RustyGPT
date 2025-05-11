use crate::api::RustyGPTClient;
use crate::containers::setup::Setup;
use crate::models::app_state::AppState;
use crate::routes::MainRoute;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::suspense::Suspense;
use yew::{Html, function_component, html, use_effect_with, use_state};
use yew_router::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[function_component(App)]
pub fn app() -> Html {
    let app_state = use_state(|| None::<AppState>);

    {
        let app_state = app_state.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                let client = RustyGPTClient::new("http://localhost:8080/api");
                let response = client.get_setup().await;
                let is_setup = match response {
                    Ok(setup_response) => setup_response.is_setup,
                    Err(_) => false,
                };
                app_state.set(Some(AppState {
                    is_setup: Some(is_setup),
                }));
            });
            || ()
        });
    }

    html! {
        <Suspense fallback={ html!{ <crate::components::loading::Loading/> } }>
            {
                match *app_state {
                    None => html!{ /* Pending API response */ },
                    Some(ref state) if state.is_setup == Some(false) => html!{
                        <Setup />
                    },
                    Some(ref state) if state.is_setup == Some(true) => html!{
                        <BrowserRouter>
                            <Switch<MainRoute> render={crate::routes::switch} />
                        </BrowserRouter>
                    },
                    _ => html!{}
                }
            }
        </Suspense>
    }
}
