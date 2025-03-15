use crate::routes::Routes;
use std::any::Any;
use std::rc::Rc;
use yew_icons::IconId;

/// A badge variant for menu items
#[derive(Clone, Debug, PartialEq)]
pub enum BadgeVariant {
    Primary,
    Secondary,
    Accent,
    Info,
    Success,
    Warning,
    Error,
}

/// A menu item in the sidebar
#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub i18n_key: String,
    pub icon: Option<IconId>,
    pub url: Option<String>,
    pub route: Option<Routes>,
    pub badge: Option<String>,
    pub badge_variant: Option<BadgeVariant>,
    pub submenu: Option<Vec<MenuItem>>,
    pub is_active: bool,
    pub is_submenu_open: bool,
    pub data: Option<Rc<dyn Any>>,
}

impl PartialEq for MenuItem {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label
            && self.i18n_key == other.i18n_key
            && self.icon == other.icon
            && self.url == other.url
            && self.route == other.route
            && self.badge == other.badge
            && self.badge_variant == other.badge_variant
            && self.submenu == other.submenu
            && self.is_active == other.is_active
            && self.is_submenu_open == other.is_submenu_open
        // data field is deliberately excluded from equality check
    }
}

impl MenuItem {
    /// Create a new menu item
    pub fn new(label: impl Into<String>, i18n_key: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            i18n_key: i18n_key.into(),
            icon: None,
            url: None,
            route: None,
            badge: None,
            badge_variant: None,
            submenu: None,
            is_active: false,
            is_submenu_open: false,
            data: None,
        }
    }

    /// Set the icon for the menu item
    pub fn with_icon(mut self, icon: IconId) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the URL for the menu item
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the route for the menu item using Yew router
    pub fn with_route(mut self, route: Routes) -> Self {
        self.route = Some(route);
        self
    }

    /// Set whether the menu item is active
    pub fn with_active(mut self, is_active: bool) -> Self {
        self.is_active = is_active;
        self
    }

    /// Set the badge for the menu item
    pub fn with_badge(mut self, badge: impl Into<String>, variant: BadgeVariant) -> Self {
        self.badge = Some(badge.into());
        self.badge_variant = Some(variant);
        self
    }

    /// Set the submenu for the menu item
    pub fn with_submenu(mut self, submenu: Vec<MenuItem>) -> Self {
        self.submenu = Some(submenu);
        self
    }

    /// Set whether the submenu is open
    pub fn with_submenu_open(mut self, is_open: bool) -> Self {
        self.is_submenu_open = is_open;
        self
    }

    /// Set the data for the menu item
    pub fn with_data(mut self, data: Rc<dyn Any>) -> Self {
        self.data = Some(data);
        self
    }
}
