use crate::routes::MainRoute;
use wasm_bindgen::prelude::*;
use yew::{Html, function_component, html};
use yew_router::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <div class="bg-base-200 min-h-screen">
            <BrowserRouter>
                <Switch<MainRoute> render={crate::routes::switch} />
            </BrowserRouter>
        </div>
    }
}
