use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{Timestamp, chat::ConversationRole};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    Online,
    Away,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct PresenceHeartbeatRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PresenceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct PresenceUpdate {
    pub user_id: Uuid,
    pub status: PresenceStatus,
    pub last_seen_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct TypingRequest {
    pub conversation_id: Uuid,
    pub root_id: Uuid,
    pub seconds: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct TypingUpdate {
    pub conversation_id: Uuid,
    pub root_id: Uuid,
    pub user_id: Uuid,
    pub expires_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MarkThreadReadRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UnreadThreadSummary {
    pub root_id: Uuid,
    pub unread: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UnreadSummaryResponse {
    pub threads: Vec<UnreadThreadSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MembershipChangeAction {
    Added,
    Removed,
    RoleUpdated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MembershipChangedEvent {
    pub user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<ConversationRole>,
    pub action: MembershipChangeAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct CreateInviteRequest {
    pub email: String,
    pub role: ConversationRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct CreateInviteResponse {
    pub token: String,
    pub expires_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct AcceptInviteRequest {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MessageDeleteRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MessageEditRequest {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UnreadUpdateEvent {
    pub root_id: Uuid,
    pub unread: i64,
}
