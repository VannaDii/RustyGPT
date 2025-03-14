use wasm_bindgen::prelude::*;
use yew::{Html, function_component, html};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[function_component(TestComponent)]
pub fn test_component() -> Html {
    log("Test component rendering");

    html! {
        <div class="p-4">
            <h1 class="text-3xl font-bold">{"Simple Test Component"}</h1>
            <p class="mt-4">{"If you can see this, Yew is rendering correctly!"}</p>
            <div class="mt-4 p-4 bg-primary text-primary-content rounded-box">
                <p>{"This is a styled box from DaisyUI"}</p>
            </div>
        </div>
    }
}
