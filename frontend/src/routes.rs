use crate::containers::layout::Layout;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Routable)]
pub enum Routes {
    #[at("/")]
    Layout,
    #[at("/settings")]
    Settings,
}

pub fn switch(routes: Routes) -> Html {
    match routes {
        Routes::Layout => html! { <Layout/> },
        Routes::Settings => html! { <div>{"Settings"}</div> },
    }
}
