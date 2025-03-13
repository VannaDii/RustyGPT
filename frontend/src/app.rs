use crate::routes::{Routes, switch};
use crate::components::loading::Loading;
use yew::{Html, function_component, html, Suspense};
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
