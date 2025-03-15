use crate::{containers::layout::Layout, pages::DashboardPage};
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_router::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// The main routes
#[derive(Debug, Clone, PartialEq, Routable)]
pub enum MainRoute {
    #[at("/")]
    Home,
    #[at("/admin")]
    AdminRoot,
    #[at("/admin/*")]
    Admin,
    #[not_found]
    #[at("/404")]
    NotFound,
}

/// The admin routes.
#[derive(Debug, Clone, PartialEq, Routable)]
pub enum AdminRoute {
    #[at("/admin")]
    Profile,
    #[at("/admin/system")]
    System,
    #[at("/admin/users")]
    Users,
    #[at("/admin/users/roles")]
    UserRoles,
    #[not_found]
    #[at("/admin/404")]
    NotFound,
}

/// The app routes.
#[derive(Debug, Clone, PartialEq)]
pub enum AppRoute {
    Main(MainRoute),
    Admin(AdminRoute),
}

impl Default for AppRoute {
    fn default() -> Self {
        AppRoute::Main(MainRoute::Home)
    }
}

/// Switch function for the main routes.
pub fn switch(route: MainRoute) -> Html {
    log(std::format!("Switching to main route: {:?}", route).as_str());
    match route {
        MainRoute::Home => {
            html! {<Layout current_route={AppRoute::Main(route)}><DashboardPage /></Layout>}
        }
        MainRoute::AdminRoot | MainRoute::Admin => {
            html! { <Switch<AdminRoute> render={switch_admin} /> }
        }
        MainRoute::NotFound => {
            html! {<Layout current_route={AppRoute::Main(route)}><h1>{"Not Found"}</h1></Layout>}
        }
    }
}

/// Switch function for the admin routes.
fn switch_admin(route: AdminRoute) -> Html {
    log(std::format!("Switching to admin route: {:?}", route).as_str());
    match route {
        AdminRoute::Profile => {
            html! {<Layout current_route={AppRoute::Admin(route)}><h1>{"Profile"}</h1></Layout>}
        }
        AdminRoute::System => {
            html! {<Layout current_route={AppRoute::Admin(route)}><h1>{"System Settings"}</h1></Layout>}
        }
        AdminRoute::Users => {
            html! {<Layout current_route={AppRoute::Admin(route)}><h1>{"User Management"}</h1></Layout>}
        }
        AdminRoute::UserRoles => {
            html! {<Layout current_route={AppRoute::Admin(route)}><h1>{"User Role Management"}</h1></Layout>}
        }
        AdminRoute::NotFound => html! {<Redirect<MainRoute> to={MainRoute::NotFound}/>},
    }
}
