use crate::routes::MainRoute;
use gloo_timers::future::TimeoutFuture;
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
    let setup_state = use_state(|| None::<bool>);

    {
        let setup_state = setup_state.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                // Simulate API call delay to /api/setup
                TimeoutFuture::new(1000).await;
                // TODO: Call /api/setup and update the state accordingly.
                // For now, we stub the system as set up (true).
                setup_state.set(Some(true));
            });
            || ()
        });
    }

    html! {
        <Suspense fallback={ html!{ <crate::components::loading::Loading/> } }>
            {
                match *setup_state {
                    None => html!{ /* Pending API response */ },
                    Some(false) => html!{
                        // ...Setup Component Stub...
                        <div>{"Setup Component Stub"}</div>
                    },
                    Some(true) => html!{
                        // ...Login Component Stub...
                        <BrowserRouter>
                            <Switch<MainRoute> render={crate::routes::switch} />
                        </BrowserRouter>
                    },
                }
            }
        </Suspense>
    }
}
