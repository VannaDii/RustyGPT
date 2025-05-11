use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Represents a chunk of a streaming message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MessageChunk {
    /// ID of the conversation this chunk belongs to.
    pub conversation_id: Uuid,

    /// ID of the message this chunk belongs to.
    pub message_id: Uuid,

    /// The content type of this chunk.
    pub content_type: String,

    /// The content of this chunk.
    pub content: String,

    /// Whether this is the final chunk of the message.
    pub is_final: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_message_chunk_creation() {
        let conversation_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        let chunk = MessageChunk {
            conversation_id,
            message_id,
            content_type: "text".to_string(),
            content: "Hello".to_string(),
            is_final: false,
        };

        assert_eq!(chunk.conversation_id, conversation_id);
        assert_eq!(chunk.message_id, message_id);
        assert_eq!(chunk.content, "Hello");
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_message_chunk_serialization() {
        let conversation_id = Uuid::parse_str("f47ac10b-58cc-4372-a567-0e02b2c3d479").unwrap();
        let message_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let chunk = MessageChunk {
            conversation_id,
            message_id,
            content_type: "text".to_string(),
            content: "Test chunk".to_string(),
            is_final: true,
        };

        let serialized = serde_json::to_string(&chunk).unwrap();
        let deserialized: MessageChunk = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, chunk);
        assert_eq!(deserialized.conversation_id, conversation_id);
        assert_eq!(deserialized.message_id, message_id);
        assert_eq!(deserialized.content, "Test chunk");
        assert!(deserialized.is_final);
    }
}
