use crate::{containers::layout::Layout, models::app_state::AppState, pages::*};
use shared::models::UserRole;
use strum::{EnumIter, IntoEnumIterator};
use wasm_bindgen::prelude::*;
use yew::Callback;
use yew::prelude::*;
use yew_router::prelude::*;
use yewdux::prelude::use_selector;

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
    #[at("/login")]
    Login,
    #[at("/chat")]
    Chat,
    #[at("/chat/:conversation_id")]
    ChatConversation { conversation_id: String },
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

#[derive(Properties, PartialEq)]
pub struct MainRouteViewProps {
    pub route: MainRoute,
    pub on_logout: Callback<()>,
}

#[function_component(MainRouteView)]
fn main_route_view(props: &MainRouteViewProps) -> Html {
    let user = use_selector(|state: &AppState| state.user.clone());
    let user_opt = (*user).clone();
    let is_authenticated = user_opt.is_some();
    let is_admin = user_opt
        .as_ref()
        .map(|user| {
            user.roles
                .iter()
                .any(|role| matches!(role, UserRole::Admin))
        })
        .unwrap_or(false);
    let on_logout = props.on_logout.clone();

    match props.route.clone() {
        MainRoute::Login => {
            if is_authenticated {
                html! { <Redirect<MainRoute> to={MainRoute::Home} /> }
            } else {
                html! { <LoginPage /> }
            }
        }
        MainRoute::Home => {
            if !is_authenticated {
                return html! { <Redirect<MainRoute> to={MainRoute::Login} /> };
            }
            let logout_cb = on_logout.clone();
            html! {
                <Layout current_route={AppRoute::Main(MainRoute::Home)} on_logout={Some(logout_cb)}>
                    <DashboardPage />
                </Layout>
            }
        }
        MainRoute::Chat => {
            if !is_authenticated {
                return html! { <Redirect<MainRoute> to={MainRoute::Login} /> };
            }
            let logout_cb = on_logout.clone();
            html! {
                <Layout current_route={AppRoute::Main(MainRoute::Chat)} on_logout={Some(logout_cb)}>
                    <ChatPage />
                </Layout>
            }
        }
        MainRoute::ChatConversation { conversation_id } => {
            if !is_authenticated {
                return html! { <Redirect<MainRoute> to={MainRoute::Login} /> };
            }
            let route_clone = MainRoute::ChatConversation {
                conversation_id: conversation_id.clone(),
            };
            let logout_cb = on_logout.clone();
            html! {
                <Layout current_route={AppRoute::Main(route_clone)} on_logout={Some(logout_cb)}>
                    <ChatPage conversation_id={Some(conversation_id)} />
                </Layout>
            }
        }
        MainRoute::AdminRoot | MainRoute::Admin => {
            if !is_authenticated {
                return html! { <Redirect<MainRoute> to={MainRoute::Login} /> };
            }
            if !is_admin {
                return html! { <Redirect<MainRoute> to={MainRoute::Home} /> };
            }
            let logout_cb = on_logout.clone();
            html! {
                <Switch<AdminRoute> render={move |route| {
                    let logout_cb = logout_cb.clone();
                    switch_admin(route, logout_cb.clone())
                }} />
            }
        }
        MainRoute::NotFound => {
            if !is_authenticated {
                return html! { <Redirect<MainRoute> to={MainRoute::Login} /> };
            }
            let logout_cb = on_logout.clone();
            html! {
                <Layout current_route={AppRoute::Main(MainRoute::NotFound)} on_logout={Some(logout_cb)}>
                    <ErrorPage />
                </Layout>
            }
        }
    }
}

/// Switch function for the main routes.
pub fn switch_with_logout(route: MainRoute, on_logout: Callback<()>) -> Html {
    log(std::format!("Switching to main route: {:?}", route).as_str());
    html! { <MainRouteView {route} {on_logout} /> }
}

/// Switch function for the admin routes.
fn switch_admin(route: AdminRoute, on_logout: Callback<()>) -> Html {
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
            let logout_cb = on_logout.clone();
            html! {<Layout {header_routes} current_route={AppRoute::Admin(route)} on_logout={Some(logout_cb)}>
            <ProfilePage /></Layout>}
        }
        AdminRoute::System => {
            let logout_cb = on_logout.clone();
            html! {<Layout {header_routes} current_route={AppRoute::Admin(route)} on_logout={Some(logout_cb)}>
            <SettingsPage /></Layout>}
        }
        AdminRoute::Users => {
            let logout_cb = on_logout.clone();
            html! {<Layout {header_routes} current_route={AppRoute::Admin(route)} on_logout={Some(logout_cb)}>
            <UsersPage /></Layout>}
        }
        AdminRoute::UserRoles => {
            let logout_cb = on_logout.clone();
            html! {<Layout {header_routes} current_route={AppRoute::Admin(route)} on_logout={Some(logout_cb)}>
            <RolesPage /></Layout>}
        }
        AdminRoute::NotFound => html! {<Redirect<MainRoute> to={MainRoute::NotFound}/>},
    }
}
