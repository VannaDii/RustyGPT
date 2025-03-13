use crate::containers::layout::Layout;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Routable, PartialEq)]
pub enum Routes {
    #[at("/")]
    Layout,
}

pub fn switch(routes: Routes) -> Html {
    match routes {
        Routes::Layout => html! { <Layout/> },
    }
}
