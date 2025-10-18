use shared::models::{AuthenticatedUser, SessionSummary};
use yewdux::Store;

#[derive(Default, Clone, PartialEq, Store)]
pub struct AppState {
    pub is_setup: Option<bool>,
    pub user: Option<AuthenticatedUser>,
    pub session: Option<SessionSummary>,
    pub csrf_token: Option<String>,
}
