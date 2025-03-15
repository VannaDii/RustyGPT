use crate::models::sidebar::{BadgeVariant, MenuItem as MenuItemModel};
use crate::routes::Routes;
use i18nrs::yew::use_translation;
use web_sys::MouseEvent;
use yew::{Callback, Html, Properties, classes, function_component, html};
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MenuItemProps {
    pub item: MenuItemModel,
    pub index: usize,
    pub on_click: Callback<(usize, MouseEvent)>,
    pub on_toggle_submenu: Callback<usize>,
    pub is_sidebar_collapsed: bool,
}

#[function_component(MenuItem)]
pub fn menu_item(props: &MenuItemProps) -> Html {
    let (i18n, ..) = use_translation();
    let navigator = use_navigator().unwrap();
    let item = &props.item;
    let index = props.index;
    let is_collapsed = props.is_sidebar_collapsed;
    let has_submenu = item.submenu.as_ref().map_or(false, |s| !s.is_empty());
    let is_active = item.is_active || item.submenu.as_ref().map_or(false, |s| s.iter().any(|i| i.is_active));
    let is_submenu_open = item.is_submenu_open;
    let on_click = props.on_click.clone();

    let list_item_classes = classes!(
        "rounded-lg",
        "hover:bg-base-300",
        "transition-all",
        "duration-200",
        if is_active { "bg-base-300" } else { "" }
    );

    let link_classes = classes!(
        "flex",
        "items-center",
        "p-2",
        "gap-2",
        "rounded-lg",
        "font-medium",
        if is_active { "text-primary font-semibold" } else { "text-base-content" },
        if is_collapsed && !has_submenu { "justify-center" } else { "" }
    );

    let on_menu_click = {
        let on_click = on_click.clone();
        let navigator = navigator.clone();
        let item_for_callback = item.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            web_sys::console::log_1(&format!("Menu item {} clicked", index).into());

            if let Some(route) = item_for_callback.route.clone() {
                navigator.push(&route);
            } else if let Some(url) = item_for_callback.url.clone() {
                web_sys::window()
                    .and_then(|win| win.location().set_href(&url).ok())
                    .unwrap_or_else(|| {
                        web_sys::console::error_1(&"Failed to navigate".into());
                    });
            }

            on_click.emit((index, e.clone()));
        })
    };

    html! {
        <li class={list_item_classes}>
            <a href="#" onclick={on_menu_click} class={link_classes}>
                { render_icon(item) }
                <span class={classes!("transition-opacity", if is_collapsed { "hidden" } else { "" })}>
                    { i18n.t(&item.i18n_key) }
                </span>
                { render_badge(item, is_collapsed) }
                { if has_submenu && !is_collapsed {
                    html! { <Icon icon_id={IconId::HeroiconsOutlineChevronDown}
                                  class={classes!("w-4", "h-4", "ml-auto", "transition-transform",
                                                  "duration-300", if is_submenu_open { "rotate-180" } else { "" })} /> }
                } else { html! {} } }
            </a>

            { if has_submenu && !is_collapsed {
                html! {
                    <ul class={classes!("menu", "menu-sm", "pl-4", "mt-1", "overflow-hidden", "transition-all",
                                        "duration-300", if is_submenu_open { "max-h-[500px]" } else { "max-h-0" })}>
                        { for item.submenu.as_ref().unwrap().iter().enumerate().map(|(subindex, subitem)| html! {
                            <MenuItem
                                item={subitem.clone()}
                                index={(index * 100) + subindex}
                                on_click={on_click.clone()}
                                on_toggle_submenu={props.on_toggle_submenu.clone()}
                                is_sidebar_collapsed={is_collapsed}
                            />
                        }) }
                    </ul>
                }
            } else {
                html! {}
            } }
        </li>
    }
}

fn render_icon(item: &MenuItemModel) -> Html {
    if let Some(icon) = item.icon {
        html! { <Icon icon_id={icon} class="w-5 h-5" /> }
    } else {
        html! {}
    }
}

fn render_badge(item: &MenuItemModel, is_collapsed: bool) -> Html {
    if let Some(badge) = &item.badge {
        let badge_class = match item.badge_variant.clone().unwrap_or(BadgeVariant::Primary) {
            BadgeVariant::Primary => "badge-primary",
            BadgeVariant::Secondary => "badge-secondary",
            BadgeVariant::Accent => "badge-accent",
            BadgeVariant::Info => "badge-info",
            BadgeVariant::Success => "badge-success",
            BadgeVariant::Warning => "badge-warning",
            BadgeVariant::Error => "badge-error",
        };

        if !is_collapsed {
            html! { <span class={classes!("badge", badge_class, "badge-sm", "ml-auto")}>{ badge }</span> }
        } else {
            html! {}
        }
    } else {
        html! {}
    }
}