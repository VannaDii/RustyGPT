use serde::{Deserialize, Serialize};
use std::{any::Any, rc::Rc};
use yewdux::{Reducer, Store};

#[derive(Serialize, Deserialize, Clone, Store)]
pub struct RightSidebarState {
    pub is_open: bool,
    pub body_type: RightSidebarBodyType,
    #[serde(skip)]
    pub extra_object: Option<Rc<dyn Any>>,
    pub header: String,
}

impl PartialEq for RightSidebarState {
    fn eq(&self, other: &Self) -> bool {
        self.is_open == other.is_open
            && self.body_type == other.body_type
            && self.header == other.header
    }
}

impl Default for RightSidebarState {
    fn default() -> Self {
        Self {
            is_open: false,
            body_type: RightSidebarBodyType::Default,
            extra_object: None,
            header: "".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum RightSidebarBodyType {
    Notifications,
    Events,
    #[default]
    Default,
}

pub enum RightSidebarAction {
    OpenSidebar(RightSidebarBodyType, Option<Rc<dyn Any>>, String),
    CloseSidebar,
}

impl Reducer<RightSidebarState> for RightSidebarAction {
    fn apply(self, _: Rc<RightSidebarState>) -> Rc<RightSidebarState> {
        match self {
            RightSidebarAction::OpenSidebar(body_type, extra_object, header) => RightSidebarState {
                is_open: true,
                body_type,
                extra_object: extra_object.clone(),
                header: header.clone(),
            }
            .into(),
            RightSidebarAction::CloseSidebar => RightSidebarState {
                is_open: false,
                body_type: RightSidebarBodyType::Default,
                extra_object: None,
                header: "".into(),
            }
            .into(),
        }
    }
}
