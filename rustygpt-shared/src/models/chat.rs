use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{
    threads::{MembershipChangedEvent, PresenceUpdate, TypingUpdate, UnreadUpdateEvent},
    timestamp::Timestamp,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConversationRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl ConversationRole {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Viewer => "viewer",
        }
    }
}

impl TryFrom<&str> for ConversationRole {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "member" => Ok(Self::Member),
            "viewer" => Ok(Self::Viewer),
            _ => Err("invalid conversation role"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl MessageRole {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Tool => "tool",
        }
    }
}

impl TryFrom<&str> for MessageRole {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            "tool" => Ok(Self::Tool),
            _ => Err("invalid message role"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ConversationCreateRequest {
    pub title: String,
    #[serde(default)]
    pub is_group: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ConversationCreateResponse {
    pub conversation_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct AddParticipantRequest {
    pub user_id: Uuid,
    pub role: ConversationRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ThreadSummary {
    pub root_id: Uuid,
    #[serde(default)]
    pub root_excerpt: String,
    #[serde(default)]
    pub root_author: Option<Uuid>,
    pub created_at: Timestamp,
    pub last_activity_at: Timestamp,
    pub message_count: i64,
    pub participant_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ThreadListResponse {
    pub threads: Vec<ThreadSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_after: Option<Timestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MessageView {
    pub id: Uuid,
    pub root_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    pub conversation_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_user_id: Option<Uuid>,
    pub role: MessageRole,
    pub content: String,
    pub path: String,
    pub depth: i32,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ThreadTreeResponse {
    pub root_id: Uuid,
    pub messages: Vec<MessageView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct PostRootMessageRequest {
    pub content: String,
    #[serde(default)]
    pub role: Option<MessageRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct PostRootMessageResponse {
    pub message_id: Uuid,
    pub root_id: Uuid,
    pub conversation_id: Uuid,
    pub depth: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ReplyMessageRequest {
    pub content: String,
    #[serde(default)]
    pub role: Option<MessageRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ReplyMessageResponse {
    pub message_id: Uuid,
    pub root_id: Uuid,
    pub conversation_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    pub depth: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MessageChunkPayload {
    pub message_id: Uuid,
    pub chunks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ChatDeltaChunk {
    pub id: String,
    #[serde(default = "default_chunk_object")]
    pub object: String,
    pub root_id: Uuid,
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i32>,
    pub choices: Vec<ChatDeltaChoice>,
}

fn default_chunk_object() -> String {
    "chat.completion.chunk".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ChatDeltaChoice {
    pub index: u32,
    pub delta: ChatDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ChatDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MessageRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UsageBreakdown {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MessageDoneEvent {
    pub message_id: Uuid,
    pub root_id: Uuid,
    pub conversation_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ThreadNewEvent {
    pub conversation_id: Uuid,
    pub root_id: Uuid,
    pub summary: ThreadSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ThreadActivityEvent {
    pub root_id: Uuid,
    pub last_activity_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct StreamErrorEvent {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConversationStreamEvent {
    #[serde(rename = "thread.new")]
    ThreadNew { payload: ThreadNewEvent },
    #[serde(rename = "thread.activity")]
    ThreadActivity { payload: ThreadActivityEvent },
    #[serde(rename = "message.delta")]
    MessageDelta { payload: ChatDeltaChunk },
    #[serde(rename = "message.done")]
    MessageDone { payload: MessageDoneEvent },
    #[serde(rename = "presence.update")]
    PresenceUpdate { payload: PresenceUpdate },
    #[serde(rename = "typing.update")]
    TypingUpdate { payload: TypingUpdate },
    #[serde(rename = "unread.update")]
    UnreadUpdate { payload: UnreadUpdateEvent },
    #[serde(rename = "membership.changed")]
    MembershipChanged { payload: MembershipChangedEvent },
    #[serde(rename = "error")]
    Error { payload: StreamErrorEvent },
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn default_object_name() {
        let chunk = ChatDeltaChunk {
            id: "stream".to_string(),
            object: default_chunk_object(),
            root_id: Uuid::nil(),
            message_id: Uuid::nil(),
            conversation_id: Uuid::nil(),
            parent_id: None,
            depth: Some(1),
            choices: vec![],
        };

        assert_eq!(chunk.object, "chat.completion.chunk");
    }

    #[test]
    fn thread_summary_serializes() {
        let summary = ThreadSummary {
            root_id: Uuid::new_v4(),
            root_excerpt: "hello".into(),
            root_author: None,
            created_at: Timestamp(Utc::now()),
            last_activity_at: Timestamp(Utc::now()),
            message_count: 2,
            participant_count: 1,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("root_excerpt"));
    }
}
