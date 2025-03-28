use crate::{containers::layout::Layout, pages::*};
use strum::{EnumIter, IntoEnumIterator};
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_router::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// The main routes
#[derive(Debug, Clone, PartialEq, Routable, EnumIter)]
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
#[derive(Debug, Clone, PartialEq, Routable, EnumIter)]
pub enum AdminRoute {
    #[at("/admin")]
    Profile,
    #[at("/admin/system")]
    System,
    #[at("/admin/users")]
    Users,
    #[at("/admin/roles")]
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

impl From<AdminRoute> for AppRoute {
    fn from(route: AdminRoute) -> Self {
        AppRoute::Admin(route)
    }
}

impl From<MainRoute> for AppRoute {
    fn from(route: MainRoute) -> Self {
        AppRoute::Main(route)
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
            html! {<Layout current_route={AppRoute::Main(route)}><ErrorPage /></Layout>}
        }
    }
}

/// Switch function for the admin routes.
fn switch_admin(route: AdminRoute) -> Html {
    log(std::format!("Switching to admin route: {:?}", route).as_str());
    let header_routes = AdminRoute::iter()
        .filter(|route| {
            // Filter out the error routes
            route != &AdminRoute::NotFound
        })
        .map(AppRoute::Admin)
        .collect::<Vec<_>>();
    match route {
        AdminRoute::Profile => {
            html! {<Layout {header_routes} current_route={AppRoute::Admin(route)}>
            <ProfilePage /></Layout>}
        }
        AdminRoute::System => {
            html! {<Layout {header_routes} current_route={AppRoute::Admin(route)}>
            <SettingsPage /></Layout>}
        }
        AdminRoute::Users => {
            html! {<Layout {header_routes} current_route={AppRoute::Admin(route)}>
            <UsersPage /></Layout>}
        }
        AdminRoute::UserRoles => {
            html! {<Layout {header_routes} current_route={AppRoute::Admin(route)}>
            <RolesPage /></Layout>}
        }
        AdminRoute::NotFound => html! {<Redirect<MainRoute> to={MainRoute::NotFound}/>},
    }
}
