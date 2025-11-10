pub mod chat;
pub mod errors;
pub mod limits;
pub mod oauth;
pub mod setup;
pub mod streaming;
pub mod threads;
pub mod timestamp;
pub mod user;

pub use chat::{
    AddParticipantRequest, ChatDelta, ChatDeltaChoice, ChatDeltaChunk, ConversationCreateRequest,
    ConversationCreateResponse, ConversationRole, ConversationStreamEvent, MessageChunkPayload,
    MessageDoneEvent, MessageRole, MessageView, PostRootMessageRequest, PostRootMessageResponse,
    ReplyMessageRequest, ReplyMessageResponse, StreamErrorEvent, ThreadActivityEvent,
    ThreadListResponse, ThreadNewEvent, ThreadSummary, ThreadTreeResponse, UsageBreakdown,
};
pub use errors::ErrorResponse;
pub use limits::{
    AssignRateLimitRequest, CreateRateLimitProfileRequest, RateLimitAssignment, RateLimitProfile,
    UpdateRateLimitProfileRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use setup::SetupRequest;
pub use setup::SetupResponse;
pub use streaming::MessageChunk;
pub use threads::{
    AcceptInviteRequest, CreateInviteRequest, CreateInviteResponse, MarkThreadReadRequest,
    MembershipChangeAction, MembershipChangedEvent, MessageDeleteRequest, MessageEditRequest,
    PresenceHeartbeatRequest, PresenceStatus, PresenceUpdate, TypingRequest, TypingUpdate,
    UnreadSummaryResponse, UnreadThreadSummary, UnreadUpdateEvent,
};
pub use timestamp::Timestamp;
pub use user::{
    AuthenticatedUser, LoginRequest, LoginResponse, MeResponse, SessionSummary, User, UserRole,
};

fn default_model_object() -> String {
    "model".to_string()
}

/// Represents a model available for chat completions.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Model {
    /// The unique identifier of the model.
    pub id: String,
    /// Object type, fixed to "model" for `OpenAI` compatibility.
    #[serde(default = "default_model_object")]
    pub object: String,
    /// Unix timestamp when the model metadata was created.
    pub created: i64,
    /// Owner identifier for the model.
    pub owned_by: String,
    /// Human-friendly display name (non-standard extension).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Model category (non-standard extension).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_type: Option<String>,
}

/// Response schema for `/v1/models` endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelsResponse {
    /// List of available models.
    pub models: Vec<Model>,
}

/// Request schema for `/v1/chat/completions`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionRequest {
    /// The model to use for the completion.
    pub model: String,
    /// The input messages for the chat.
    pub messages: Vec<ChatCompletionMessage>,
    /// Optional temperature for randomness in responses.
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Optional nucleus sampling parameter.
    #[serde(default)]
    pub top_p: Option<f32>,
    /// Optional upper bound on generated tokens.
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Optional stop sequences (string or array of strings).
    #[serde(default)]
    pub stop: Option<Value>,
    /// Optional presence penalty (ignored but reported).
    #[serde(default)]
    pub presence_penalty: Option<f32>,
    /// Optional frequency penalty (ignored but reported).
    #[serde(default)]
    pub frequency_penalty: Option<f32>,
    /// Optional end-user identifier.
    #[serde(default)]
    pub user: Option<String>,
    /// Whether to stream deltas. Defaults to false.
    #[serde(default)]
    pub stream: Option<bool>,
    /// Arbitrary metadata; `RustyGPT` extensions expect `metadata.rustygpt`.
    #[serde(default)]
    pub metadata: Option<Value>,
}

/// A single chat message supplied in the completion request or returned in a response.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionMessage {
    /// The role of the message sender (e.g., "user", "assistant").
    pub role: String,
    /// The content of the message.
    pub content: String,
    /// Optional name for tool/function messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Response schema for `/v1/chat/completions`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionResponse {
    /// Unique identifier for this completion.
    pub id: String,
    /// Object type, fixed to "chat.completion".
    pub object: String,
    /// Unix timestamp when the completion was created.
    pub created: i64,
    /// The model that generated the completion.
    pub model: String,
    /// The generated responses.
    pub choices: Vec<ChatCompletionChoice>,
    /// Token usage details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageBreakdown>,
    /// Optional fingerprint for model diagnostics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Warnings about ignored or adjusted parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// A single choice in the chat completion response.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionChoice {
    /// The index of the choice.
    pub index: usize,
    /// The message content.
    pub message: ChatCompletionMessage,
    /// Finish reason (e.g., "stop", "length").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// Log probability data (currently unsupported).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Value>,
}

/// Streaming chunk response for `/v1/chat/completions?stream=true`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionChunk {
    /// Identifier matching the completion id.
    pub id: String,
    /// Object type, fixed to "chat.completion.chunk".
    pub object: String,
    /// Unix timestamp when the chunk was generated.
    pub created: i64,
    /// The model producing the chunk.
    pub model: String,
    /// Optional fingerprint for diagnostics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Token usage summary (present on terminal chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageBreakdown>,
    /// Chunk choices.
    pub choices: Vec<ChatCompletionChunkChoice>,
    /// Warnings propagated to the stream.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Choice entry within a streaming chunk.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionChunkChoice {
    /// Choice index.
    pub index: usize,
    /// Delta describing incremental tokens.
    pub delta: ChatCompletionChunkDelta,
    /// Finish reason (set when `delta` is terminal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Delta payload for streaming chunks.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ChatCompletionChunkDelta {
    /// Optional role (present on first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Partial content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn model_defaults_object_field() {
        let payload = json!({
            "id": "test",
            "created": 1,
            "owned_by": "unit",
        });
        let model: Model = serde_json::from_value(payload).expect("deserialize");
        assert_eq!(model.object, "model");
        assert_eq!(model.id, "test");
    }

    #[test]
    fn chunk_warnings_default_empty() {
        let chunk = ChatCompletionChunk {
            id: "chunk".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 42,
            model: "gpt-test".to_string(),
            system_fingerprint: None,
            usage: None,
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: ChatCompletionChunkDelta {
                    role: Some("assistant".into()),
                    content: Some("Hello".into()),
                },
                finish_reason: None,
            }],
            warnings: Vec::new(),
        };

        let serialized = serde_json::to_string(&chunk).expect("serialize");
        assert!(
            !serialized.contains("warnings"),
            "empty warnings should be omitted"
        );
    }
}
