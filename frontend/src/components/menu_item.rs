use crate::models::sidebar::{BadgeVariant, MenuItem as MenuItemModel};
use i18nrs::yew::use_translation;
use web_sys::MouseEvent;
use yew::{Callback, Html, Properties, classes, function_component, html};
use yew_icons::{Icon, IconId};

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
    let item = &props.item;
    let index = props.index;
    let is_collapsed = props.is_sidebar_collapsed;
    let has_submenu = item.submenu.is_some() && !item.submenu.as_ref().unwrap().is_empty();
    let is_active = item.is_active
        || item
            .submenu
            .as_ref()
            .map_or(false, |submenu| submenu.iter().any(|item| item.is_active));
    let is_submenu_open = item.is_submenu_open;

    // Generate the classes for menu items - applying DashWind style
    let list_item_classes = classes!(
        "rounded-lg",
        "hover:bg-base-300",
        "transition-all",
        "duration-200",
        if is_active { "bg-base-300" } else { "" }
    );

    // Generate the classes for the menu item link
    let link_classes = classes!(
        "flex",
        "items-center",
        "p-2",
        "gap-2",
        "rounded-lg",
        "font-medium",
        if is_active {
            "text-primary font-semibold"
        } else {
            "text-base-content"
        },
        if is_collapsed && !has_submenu {
            "justify-center"
        } else {
            ""
        }
    );

    // For submenu
    let submenu_classes = classes!(
        "menu",
        "menu-sm",
        "pl-4",
        "mt-1",
        "overflow-hidden",
        "transition-all",
        "duration-300",
        "max-h-0", // Default state
        if is_submenu_open {
            "max-h-[500px]"
        } else {
            "max-h-0"
        }  // Dynamic height based on state
    );

    // Dynamic event when clicking on a menu item
    let on_menu_click = {
        let on_click = props.on_click.clone();
        let index = index;

        if has_submenu {
            let on_toggle_submenu = props.on_toggle_submenu.clone();

            Callback::from(move |e: MouseEvent| {
                e.prevent_default();
                on_toggle_submenu.emit(index);
            })
        } else {
            Callback::from(move |e: MouseEvent| {
                on_click.emit((index, e));
            })
        }
    };

    html! {
        <li class={list_item_classes}>
            <a
                href={item.url.clone().unwrap_or_else(|| "#".to_string())}
                onclick={on_menu_click}
                class={link_classes}
            >
                {
                    if let Some(icon) = item.icon {
                        html! {
                            <Icon icon_id={icon} class="w-5 h-5" />
                        }
                    } else {
                        html! {}
                    }
                }

                <span class={classes!(
                    "transition-opacity",
                    if is_collapsed { "hidden" } else { "" }
                )}>
                    {i18n.t(&item.i18n_key)}
                </span>

                {
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
                            html! {
                                <span class={classes!("badge", badge_class, "badge-sm", "ml-auto")}>
                                    {badge}
                                </span>
                            }
                        } else {
                            html! {}
                        }
                    } else {
                        html! {}
                    }
                }

                {
                    if has_submenu && !is_collapsed {
                        html! {
                            <Icon
                                icon_id={IconId::HeroiconsOutlineChevronDown}
                                class={classes!(
                                    "w-4",
                                    "h-4",
                                    "ml-auto",
                                    "transition-transform",
                                    "duration-300",
                                    if is_submenu_open { "rotate-180" } else { "" }
                                )}
                            />
                        }
                    } else {
                        html! {}
                    }
                }
            </a>

            // Submenu
            {
                if has_submenu && !is_collapsed {
                    let submenu = item.submenu.as_ref().unwrap();

                    html! {
                        <ul class={submenu_classes}>
                            {
                                submenu.iter().enumerate().map(|(subindex, subitem)| {
                                    let full_index = (index * 100) + subindex;
                                    let on_submenu_click = {
                                        let on_click = props.on_click.clone();
                                        let index = full_index;

                                        Callback::from(move |e: MouseEvent| {
                                            on_click.emit((index, e))
                                        })
                                    };

                                    html! {
                                        <li key={subindex} class={classes!(
                                            "rounded-lg",
                                            "hover:bg-base-300",
                                            if subitem.is_active { "bg-base-300" } else { "" }
                                        )}>
                                            <a
                                                href={subitem.url.clone().unwrap_or_else(|| "#".to_string())}
                                                onclick={on_submenu_click}
                                                class={classes!(
                                                    "block",
                                                    "py-2",
                                                    "px-3",
                                                    "rounded-lg",
                                                    "flex",
                                                    "items-center",
                                                    "text-sm",
                                                    if subitem.is_active { "text-primary font-medium" } else { "" }
                                                )}
                                            >
                                                <span class="ml-1">{i18n.t(&subitem.i18n_key)}</span>

                                                {
                                                    if let Some(badge) = &subitem.badge {
                                                        let badge_class = match subitem.badge_variant.clone().unwrap_or(BadgeVariant::Primary) {
                                                            BadgeVariant::Primary => "badge-primary",
                                                            BadgeVariant::Secondary => "badge-secondary",
                                                            BadgeVariant::Accent => "badge-accent",
                                                            BadgeVariant::Info => "badge-info",
                                                            BadgeVariant::Success => "badge-success",
                                                            BadgeVariant::Warning => "badge-warning",
                                                            BadgeVariant::Error => "badge-error",
                                                        };

                                                        html! {
                                                            <span class={classes!("badge", badge_class, "badge-sm", "ml-auto")}>
                                                                {badge}
                                                            </span>
                                                        }
                                                    } else {
                                                        html! {}
                                                    }
                                                }
                                            </a>
                                        </li>
                                    }
                                }).collect::<Html>()
                            }
                        </ul>
                    }
                } else {
                    html! {}
                }
            }
        </li>
    }
}
