use i18nrs::yew::use_translation;
use yew::{Html, Properties, classes, function_component, html};
use yew_router::prelude::{Link, Routable};

use crate::routes::AppRoute;

#[derive(Properties, PartialEq, Eq)]
pub struct HeaderNavItemProps<R: Routable + Clone + Eq + Into<AppRoute> + 'static> {
    pub route: R,
    pub current_route: Option<AppRoute>,
}

#[function_component(HeaderNavItem)]
pub fn header_nav_item_dropdown<R: Routable + Clone + PartialEq + Eq + Into<AppRoute> + 'static>(
    props: &HeaderNavItemProps<R>,
) -> Html {
    let (i18n, ..) = use_translation();

    let route = props.route.clone();
    let route_path = format!(
        "admin.routes{}",
        route.to_path().replace("/admin", "").replace('/', ".")
    );
    let route_name = i18n.t(&format!("{route_path}.title"));
    let route_icon = i18n.t(&format!("{route_path}.icon"));

    // Convert R to AppRoute for comparison
    let app_route: AppRoute = props.route.clone().into();
    let active_route_class = if props.current_route.as_ref() == Some(&app_route) {
        "btn-soft"
    } else {
        ""
    };

    html! {
      <li>
          <Link<R> to={props.route.clone()} classes={classes!("btn", "btn-ghost", "gap-2", active_route_class)}>
              <i class={classes!("fa-solid", "fa-fw", format!("fa-{route_icon}"))}></i>
              {route_name}
          </Link<R>>
      </li>
    }
}
