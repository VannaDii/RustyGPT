use crate::components::loading::Loading;
use crate::routes::{Routes, switch};
use yew::{Html, Suspense, function_component, html};
use yew_router::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <BrowserRouter>
            <Suspense fallback={html! { <Loading /> }}>
                <Switch<Routes> render={switch} />
            </Suspense>
        </BrowserRouter>
    }
}
