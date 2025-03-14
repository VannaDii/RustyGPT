use serde::{Deserialize, Serialize};
use std::rc::Rc;
use yewdux::{Reducer, Store};

#[derive(Serialize, Deserialize, Clone, PartialEq, Store)]
pub struct HeaderState {
    /// The title of the page
    pub page_title: String,
    /// The notification count
    pub notification_count: i16,
    /// The notification message
    pub notification_message: String,
    /// The notification status
    pub notification_status: NotificationStatus,
}

impl Default for HeaderState {
    fn default() -> Self {
        Self {
            page_title: "RustyGPT".to_string(),
            notification_count: 0,
            notification_message: "".to_string(),
            notification_status: NotificationStatus::None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum NotificationStatus {
    Success,
    Error,
    Info,
    #[default]
    None,
}

pub enum HeaderAction {
    PageTitle(String),
    NotificationCount(i16),
    NotificationMessage(String),
    NotificationStatus(NotificationStatus),
}

impl Reducer<HeaderState> for HeaderAction {
    fn apply(self, state: Rc<HeaderState>) -> Rc<HeaderState> {
        match self {
            HeaderAction::PageTitle(title) => HeaderState {
                page_title: title.clone(),
                ..state.as_ref().clone()
            }
            .into(),
            HeaderAction::NotificationCount(count) => HeaderState {
                notification_count: count,
                ..state.as_ref().clone()
            }
            .into(),
            HeaderAction::NotificationMessage(message) => HeaderState {
                notification_message: message.clone(),
                ..state.as_ref().clone()
            }
            .into(),
            HeaderAction::NotificationStatus(status) => HeaderState {
                notification_status: status,
                ..state.as_ref().clone()
            }
            .into(),
        }
    }
}
