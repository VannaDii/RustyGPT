use i18nrs::yew::use_translation;
use strum::IntoEnumIterator;
use wasm_bindgen::prelude::*;
use yew::{Html, classes, function_component, html};
use yew_router::{Routable, prelude::Link};

use crate::routes::AdminRoute;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// ProfilePage page component
#[function_component(ProfilePage)]
pub fn profile_page() -> Html {
    let (i18n, _) = use_translation();

    let routes = AdminRoute::iter()
        .map(|route| {
            let route_path = route.to_path().replace("/admin", "").replace("/", ".");
            let route_name = i18n.t(format!("admin.routes{}.title", route_path).as_str());
            let route_icon = i18n.t(format!("admin.routes{}.icon", route_path).as_str());
            (route_name, route_icon, route)
        })
        .collect::<Vec<_>>();

    html! {
    <div class="p-4 space-y-6">
        <nav class="navbar justify-center bg-base-300">
            <ul class="menu menu-horizontal flex-nowrap overflow-x-auto">
            { for routes.iter().map(|route| html! {
                <li>
                    <Link<AdminRoute> to={route.2.clone()}>
                        <i class={classes!("fa-solid", "fa-fw", format!("fa-{}", &route.1))}></i>
                        {&route.0}
                    </Link<AdminRoute>>
                </li>
            })}
            </ul>
        </nav>
    </div>
    }
}
