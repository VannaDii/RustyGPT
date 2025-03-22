use yewdux::Store;

use super::header::HeaderState;

#[derive(Clone, PartialEq, Store)]
pub struct AppState {
    pub header: HeaderState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            header: HeaderState::default(),
        }
    }
}
