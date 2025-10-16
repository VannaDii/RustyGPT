pub mod chat;
pub mod errors;
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
use serde::{Deserialize, Serialize};
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
pub use user::User;

/// Represents a model available for chat completions.
#[derive(Debug, Serialize, Deserialize)]
pub struct Model {
    /// The unique identifier of the model.
    pub id: String,
    /// The name of the model.
    pub name: String,
    /// The type of the model (e.g., "chat", "completion").
    pub model_type: String,
}

/// Response schema for `/v1/models` endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelsResponse {
    /// List of available models.
    pub models: Vec<Model>,
}

/// Request schema for `/v1/chat/completions`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// The model to use for the completion.
    pub model: String,
    /// The input messages for the chat.
    pub messages: Vec<ChatMessage>,
    /// Optional temperature for randomness in responses.
    pub temperature: Option<f32>,
}

/// A single chat message in the conversation.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message sender (e.g., "user", "assistant").
    pub role: String,
    /// The content of the message.
    pub content: String,
}

/// Response schema for `/v1/chat/completions`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// The generated responses.
    pub choices: Vec<ChatChoice>,
}

/// A single choice in the chat completion response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatChoice {
    /// The index of the choice.
    pub index: usize,
    /// The message content.
    pub message: ChatMessage,
}
