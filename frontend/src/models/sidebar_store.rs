use crate::models::sidebar::MenuItem;
use std::rc::Rc;
use yewdux::prelude::*;

#[derive(Clone, Default, PartialEq, Store)]
pub struct SidebarStore {
    pub state: SidebarState,
}

#[derive(Clone, Default, PartialEq)]
pub struct SidebarState {
    pub is_collapsed: bool,
    pub menu_items: Vec<MenuItem>,
    pub active_index: Option<usize>,
}

#[derive(Clone)]
pub enum SidebarAction {
    SetCollapsed(bool),
    ToggleCollapsed,
    SetMenuItems(Vec<MenuItem>),
    SetActive(usize),
    ToggleSubmenu(usize),
}

// Implementation of the reducer pattern for the sidebar state
impl Reducer<SidebarStore> for SidebarAction {
    fn apply(self, store: Rc<SidebarStore>) -> Rc<SidebarStore> {
        let mut state = store.state.clone();

        match self {
            SidebarAction::SetCollapsed(is_collapsed) => {
                state.is_collapsed = is_collapsed;
            }
            SidebarAction::ToggleCollapsed => {
                state.is_collapsed = !state.is_collapsed;
            }
            SidebarAction::SetMenuItems(menu_items) => {
                state.menu_items = menu_items;
            }
            SidebarAction::SetActive(index) => {
                state.active_index = Some(index);
            }
            SidebarAction::ToggleSubmenu(index) => {
                if let Some(item) = state.menu_items.get_mut(index) {
                    item.is_submenu_open = !item.is_submenu_open;
                }
            }
        }

        Rc::new(SidebarStore { state })
    }
}
