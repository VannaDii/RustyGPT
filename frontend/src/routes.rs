use crate::containers::layout::Layout;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_router::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Debug, Clone, Copy, PartialEq, Routable)]
pub enum Routes {
    #[at("/")]
    Dashboard,
    #[at("/transactions")]
    Transactions,
    #[at("/settings")]
    Settings,
    #[at("/users")]
    Users,
    #[at("/users/roles")]
    UserRoles,
    #[at("/users/permissions")]
    UserPermissions,
    #[at("/reports/sales")]
    ReportsSales,
    #[at("/reports/users")]
    ReportsUsers,
    #[at("/reports/performance")]
    ReportsPerformance,
}

pub fn switch(routes: Routes) -> Html {
    log("Switch function called");

    match routes {
        Routes::Dashboard => {
            log("Route: Dashboard");
            html! { <Layout current_route={routes}>
                <div>{format!("Content for {:?} page", routes)}</div>
            </Layout> }
        }
        Routes::Transactions => {
            log("Route: Transactions");
            html! { <Layout current_route={routes}>
                <div>{format!("Content for {:?} page", routes)}</div>
            </Layout> }
        }
        Routes::Settings => {
            log("Route: Settings");
            html! { <Layout current_route={routes}>
                <div>{format!("Content for {:?} page", routes)}</div>
            </Layout> }
        }
        Routes::Users => {
            log("Route: Users");
            html! { <Layout current_route={routes}>
                <div>{format!("Content for {:?} page", routes)}</div>
            </Layout> }
        }
        Routes::UserRoles => {
            log("Route: UserRoles");
            html! { <Layout current_route={routes}>
                <div>{format!("Content for {:?} page", routes)}</div>
            </Layout> }
        }
        Routes::UserPermissions => {
            log("Route: UserPermissions");
            html! { <Layout current_route={routes}>
                <div>{format!("Content for {:?} page", routes)}</div>
            </Layout> }
        }
        Routes::ReportsSales => {
            log("Route: ReportsSales");
            html! { <Layout current_route={routes}>
                <div>{format!("Content for {:?} page", routes)}</div>
            </Layout> }
        }
        Routes::ReportsUsers => {
            log("Route: ReportsUsers");
            html! { <Layout current_route={routes}>
                <div>{format!("Content for {:?} page", routes)}</div>
            </Layout> }
        }
        Routes::ReportsPerformance => {
            log("Route: ReportsPerformance");
            html! { <Layout current_route={routes}>
                <div>{format!("Content for {:?} page", routes)}</div>
            </Layout> }
        }
    }
}
