use crate::{
    containers::layout::Layout,
    models::app_state::AppState,
    pages::{
        ChatPage, DashboardPage, ErrorPage, LoginPage, ProfilePage, RolesPage, SettingsPage,
        UsersPage,
    },
};
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
#[derive(Debug, Clone, PartialEq, Eq, Routable, EnumIter)]
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
#[derive(Debug, Clone, PartialEq, Eq, Routable, EnumIter)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppRoute {
    Main(MainRoute),
    Admin(AdminRoute),
}

impl Default for AppRoute {
    fn default() -> Self {
        Self::Main(MainRoute::Home)
    }
}

impl From<AdminRoute> for AppRoute {
    fn from(route: AdminRoute) -> Self {
        Self::Admin(route)
    }
}

impl From<MainRoute> for AppRoute {
    fn from(route: MainRoute) -> Self {
        Self::Main(route)
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
    let is_admin = user_opt.as_ref().is_some_and(|user| {
        user.roles
            .iter()
            .any(|role| matches!(role, UserRole::Admin))
    });
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
            let logout = props.on_logout.clone();
            html! {
                <Layout current_route={AppRoute::Main(MainRoute::Home)} on_logout={Some(logout)}>
                    <DashboardPage />
                </Layout>
            }
        }
        MainRoute::Chat => {
            if !is_authenticated {
                return html! { <Redirect<MainRoute> to={MainRoute::Login} /> };
            }
            let logout = props.on_logout.clone();
            html! {
                <Layout current_route={AppRoute::Main(MainRoute::Chat)} on_logout={Some(logout)}>
                    <ChatPage />
                </Layout>
            }
        }
        MainRoute::ChatConversation { conversation_id } => {
            if !is_authenticated {
                return html! { <Redirect<MainRoute> to={MainRoute::Login} /> };
            }
            let logout = props.on_logout.clone();
            html! {
                <Layout current_route={AppRoute::Main(MainRoute::ChatConversation { conversation_id: conversation_id.clone() })} on_logout={Some(logout)}>
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
            let logout = props.on_logout.clone();
            html! {
                <Switch<AdminRoute> render={
                    move |route| switch_admin(route, logout.clone())
                } />
            }
        }
        MainRoute::NotFound => {
            if !is_authenticated {
                return html! { <Redirect<MainRoute> to={MainRoute::Login} /> };
            }
            let logout = props.on_logout.clone();
            html! {
                <Layout current_route={AppRoute::Main(MainRoute::NotFound)} on_logout={Some(logout)}>
                    <ErrorPage />
                </Layout>
            }
        }
    }
}

/// Switch function for the main routes.
pub fn switch_with_logout(route: MainRoute, on_logout: Callback<()>) -> Html {
    log(std::format!("Switching to main route: {route:?}").as_str());
    html! { <MainRouteView {route} {on_logout} /> }
}

/// Switch function for the admin routes.
fn switch_admin(route: AdminRoute, on_logout: Callback<()>) -> Html {
    log(std::format!("Switching to admin route: {route:?}").as_str());
    let header_routes = AdminRoute::iter()
        .filter(|route| {
            // Filter out the error routes
            route != &AdminRoute::NotFound
        })
        .map(AppRoute::Admin)
        .collect::<Vec<_>>();
    match (route, on_logout) {
        (AdminRoute::Profile, logout) => {
            html! {<Layout {header_routes} current_route={AppRoute::Admin(AdminRoute::Profile)} on_logout={Some(logout)}>
            <ProfilePage /></Layout>}
        }
        (AdminRoute::System, logout) => {
            html! {<Layout {header_routes} current_route={AppRoute::Admin(AdminRoute::System)} on_logout={Some(logout)}>
            <SettingsPage /></Layout>}
        }
        (AdminRoute::Users, logout) => {
            html! {<Layout {header_routes} current_route={AppRoute::Admin(AdminRoute::Users)} on_logout={Some(logout)}>
            <UsersPage /></Layout>}
        }
        (AdminRoute::UserRoles, logout) => {
            html! {<Layout {header_routes} current_route={AppRoute::Admin(AdminRoute::UserRoles)} on_logout={Some(logout)}>
            <RolesPage /></Layout>}
        }
        (AdminRoute::NotFound, _) => html! {<Redirect<MainRoute> to={MainRoute::NotFound}/>},
    }
}
