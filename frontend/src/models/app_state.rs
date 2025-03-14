use yewdux::Store;

use super::{header::HeaderState, right_sidebar::RightSidebarState};

#[derive(Clone, PartialEq, Store)]
pub struct AppState {
    pub right_drawer: RightSidebarState,
    pub header: HeaderState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            right_drawer: RightSidebarState::default(),
            header: HeaderState::default(),
        }
    }
}
