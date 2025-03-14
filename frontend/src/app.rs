use crate::containers::layout::Layout;
use crate::routes::Routes;
use wasm_bindgen::prelude::*;
use yew::{Html, function_component, html};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[function_component(App)]
pub fn app() -> Html {
    // Initialize app component

    html! {
        <div class="bg-base-200 min-h-screen">
            <Layout current_route={Routes::Dashboard}>
                <div>{"Dashboard Content"}</div>
            </Layout>
        </div>
    }
}
