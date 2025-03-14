use crate::components::menu_item::MenuItem;
use crate::models::sidebar::{BadgeVariant, MenuItem as MenuItemModel};
use crate::models::sidebar_store::{SidebarAction, SidebarStore};
use i18nrs::yew::use_translation;
use web_sys::MouseEvent;
use yew::{Callback, Html, classes, function_component, html, use_effect_with};
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::routes::Routes;

#[function_component(LeftSidebar)]
pub fn left_sidebar() -> Html {
    let (i18n, ..) = use_translation();
    let (sidebar_state, sidebar_dispatch) = use_store::<SidebarStore>();
    let navigator = use_navigator();
    let location = use_location();

    // Create menu items
    use_effect_with(sidebar_dispatch.clone(), move |sidebar_dispatch| {
        // Create the initial menu items structure based on DashWind
        let menu_items = vec![
            MenuItemModel::new("Dashboard", "sidebar.dashboard")
                .with_icon(IconId::HeroiconsOutlineHome)
                .with_url("/")
                .with_active(true),
            MenuItemModel::new("Leads", "sidebar.leads")
                .with_icon(IconId::HeroiconsOutlineUserGroup)
                .with_url("/leads")
                .with_active(false),
            MenuItemModel::new("Transactions", "sidebar.transactions")
                .with_icon(IconId::HeroiconsOutlineCurrencyDollar)
                .with_url("/transactions")
                .with_active(false),
            MenuItemModel::new("Analytics", "sidebar.analytics")
                .with_icon(IconId::HeroiconsOutlineChartBar)
                .with_url("/analytics")
                .with_active(false),
            MenuItemModel::new("Integration", "sidebar.integration")
                .with_icon(IconId::HeroiconsOutlineCog)
                .with_url("/integration")
                .with_active(false),
            MenuItemModel::new("Calendar", "sidebar.calendar")
                .with_icon(IconId::HeroiconsOutlineCalendar)
                .with_url("/calendar")
                .with_active(false),
            MenuItemModel::new("Pages", "sidebar.pages")
                .with_icon(IconId::HeroiconsOutlineDocument)
                .with_submenu(vec![
                    MenuItemModel::new("Profile", "sidebar.profile")
                        .with_url("/profile")
                        .with_active(false),
                    MenuItemModel::new("Login", "sidebar.login")
                        .with_url("/login")
                        .with_active(false),
                    MenuItemModel::new("Register", "sidebar.register")
                        .with_url("/register")
                        .with_active(false),
                    MenuItemModel::new("Error", "sidebar.error")
                        .with_url("/error")
                        .with_active(false),
                ]),
            MenuItemModel::new("Settings", "sidebar.settings")
                .with_icon(IconId::HeroiconsOutlineCog)
                .with_url("/settings")
                .with_active(false),
            MenuItemModel::new("Documentation", "sidebar.documentation")
                .with_icon(IconId::HeroiconsOutlineInformationCircle)
                .with_submenu(vec![
                    MenuItemModel::new("Getting Started", "sidebar.getting_started")
                        .with_url("/documentation")
                        .with_active(false),
                    MenuItemModel::new("Features", "sidebar.features")
                        .with_url("/documentation/features")
                        .with_active(false),
                    MenuItemModel::new("Components", "sidebar.components")
                        .with_url("/documentation/components")
                        .with_active(false)
                        .with_badge("9+", BadgeVariant::Primary),
                ]),
        ];

        // Set menu items in the store
        sidebar_dispatch.apply(SidebarAction::SetMenuItems(menu_items));

        || {}
    });

    let handle_menu_click = {
        let navigator = navigator.clone();
        let sidebar_state = sidebar_state.clone();
        Callback::from(move |(index, _e): (usize, MouseEvent)| {
            // Handle main menu items
            if index < 100 {
                if let Some(item) = sidebar_state.state.menu_items.get(index) {
                    if let Some(_url) = &item.url {
                        web_sys::console::log_1(&"Menu item clicked".into());
                    }
                }
            } else {
                // Handle submenu items
                let main_index = index / 100;
                let sub_index = index % 100;

                if let Some(item) = sidebar_state.state.menu_items.get(main_index) {
                    if let Some(submenu) = &item.submenu {
                        if let Some(sub_item) = submenu.get(sub_index) {
                            if let Some(_url) = &sub_item.url {
                                web_sys::console::log_1(&"Submenu item clicked".into());
                            }
                        }
                    }
                }
            }
        })
    };

    let handle_toggle_submenu = {
        let sidebar_dispatch = sidebar_dispatch.clone();
        Callback::from(move |index: usize| {
            sidebar_dispatch.apply(SidebarAction::ToggleSubmenu(index));
        })
    };

    let toggle_sidebar_collapsed = {
        let sidebar_dispatch = sidebar_dispatch.clone();
        Callback::from(move |_| {
            sidebar_dispatch.apply(SidebarAction::ToggleCollapsed);
        })
    };

    // Create local copies for use in the template
    let is_collapsed = sidebar_state.state.is_collapsed;
    let menu_items = sidebar_state.state.menu_items.clone();
    let toggle_sidebar_collapsed_clone = toggle_sidebar_collapsed.clone();

    html! {
        <div class="drawer-side z-30">
            <label htmlFor="left-sidebar-drawer" class="drawer-overlay"></label>
            <div class={classes!(
                "flex",
                "flex-col",
                "bg-base-200", // DashWind uses a slightly darker background
                "h-full",
                "transition-all",
                "duration-300",
                "shadow-xl",
                if is_collapsed { "w-20" } else { "w-80" }
            )}>
                <div class="flex items-center justify-between p-4 border-b border-base-300/30">
                    <div class={classes!(
                        "flex",
                        "items-center",
                        "gap-3",
                        if is_collapsed { "justify-center w-full" } else { "" }
                    )}>
                        <span class="flex-shrink-0 text-primary text-2xl bg-primary text-primary-content w-10 h-10 rounded-lg flex items-center justify-center">
                            <span>{"DW"}</span>
                        </span>
                        <span class={classes!(
                            "font-bold",
                            "text-xl",
                            "transition-opacity",
                            if is_collapsed { "opacity-0 w-0 overflow-hidden" } else { "opacity-100" }
                        )}>
                            { "DashWind" }
                        </span>
                    </div>
                    <button
                        class={classes!(
                            "btn",
                            "btn-ghost",
                            "btn-sm",
                            "btn-circle",
                            if is_collapsed { "hidden" } else { "" }
                        )}
                        onclick={toggle_sidebar_collapsed_clone.clone()}
                    >
                        <Icon icon_id={IconId::HeroiconsOutlineChevronLeft} class="w-5 h-5" />
                    </button>
                </div>
                <div class="flex-1 px-3 py-3 overflow-y-auto">
                    <ul class="menu menu-sm gap-1">
                        {
                            menu_items.iter().enumerate().map(|(index, item)| {
                                html! {
                                    <MenuItem
                                        key={index}
                                        item={item.clone()}
                                        {index}
                                        on_click={handle_menu_click.clone()}
                                        on_toggle_submenu={handle_toggle_submenu.clone()}
                                        is_sidebar_collapsed={is_collapsed}
                                    />
                                }
                            }).collect::<Html>()
                        }
                    </ul>
                </div>
                <div class="p-4 mt-auto border-t border-base-300/30">
                    <button
                        class={classes!(
                            "btn",
                            "btn-ghost",
                            "btn-sm",
                            "justify-center",
                            "w-full",
                            if !is_collapsed { "justify-start" } else { "" }
                        )}
                        onclick={toggle_sidebar_collapsed_clone.clone()}
                    >
                        <Icon
                            icon_id={if is_collapsed { IconId::HeroiconsOutlineChevronRight } else { IconId::HeroiconsOutlineChevronLeft }}
                            class="w-4 h-4"
                        />
                        <span class={classes!(
                            "ml-2",
                            if is_collapsed { "hidden" } else { "" }
                        )}>
                            { i18n.t(if is_collapsed { "sidebar.expand" } else { "sidebar.collapse" }) }
                        </span>
                    </button>
                </div>
            </div>
        </div>
    }
}
