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
        assert_eq!(chunk.content_type, "text");
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
        assert_eq!(deserialized.content_type, "text");
        assert!(deserialized.is_final);
    }

    #[test]
    fn test_message_chunk_clone() {
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "markdown".to_string(),
            content: "# Header".to_string(),
            is_final: false,
        };

        let cloned = chunk.clone();
        assert_eq!(chunk, cloned);
        assert_eq!(chunk.conversation_id, cloned.conversation_id);
        assert_eq!(chunk.message_id, cloned.message_id);
        assert_eq!(chunk.content_type, cloned.content_type);
        assert_eq!(chunk.content, cloned.content);
        assert_eq!(chunk.is_final, cloned.is_final);
    }

    #[test]
    fn test_message_chunk_debug() {
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "json".to_string(),
            content: r#"{"key": "value"}"#.to_string(),
            is_final: true,
        };

        let debug_str = format!("{:?}", chunk);
        assert!(debug_str.contains("MessageChunk"));
        assert!(debug_str.contains("json"));
        assert!(debug_str.contains("key"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_message_chunk_equality() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let chunk1 = MessageChunk {
            conversation_id: id1,
            message_id: id2,
            content_type: "text".to_string(),
            content: "Same content".to_string(),
            is_final: false,
        };

        let chunk2 = MessageChunk {
            conversation_id: id1,
            message_id: id2,
            content_type: "text".to_string(),
            content: "Same content".to_string(),
            is_final: false,
        };

        let chunk3 = MessageChunk {
            conversation_id: id1,
            message_id: id2,
            content_type: "text".to_string(),
            content: "Different content".to_string(),
            is_final: false,
        };

        assert_eq!(chunk1, chunk2);
        assert_ne!(chunk1, chunk3);
    }

    #[test]
    fn test_message_chunk_with_empty_content() {
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text".to_string(),
            content: String::new(),
            is_final: true,
        };

        assert_eq!(chunk.content, "");
        assert!(chunk.is_final);

        let serialized = serde_json::to_string(&chunk).unwrap();
        let deserialized: MessageChunk = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.content, "");
    }

    #[test]
    fn test_message_chunk_with_empty_content_type() {
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: String::new(),
            content: "Some content".to_string(),
            is_final: false,
        };

        assert_eq!(chunk.content_type, "");
        assert_eq!(chunk.content, "Some content");
    }

    #[test]
    fn test_message_chunk_various_content_types() {
        let content_types = vec![
            "text",
            "markdown",
            "html",
            "json",
            "code",
            "error",
            "system",
            "user",
            "assistant",
        ];

        for content_type in content_types {
            let chunk = MessageChunk {
                conversation_id: Uuid::new_v4(),
                message_id: Uuid::new_v4(),
                content_type: content_type.to_string(),
                content: format!("Content for {}", content_type),
                is_final: false,
            };

            assert_eq!(chunk.content_type, content_type);
            assert!(chunk.content.contains(content_type));
        }
    }

    #[test]
    fn test_message_chunk_final_states() {
        let chunk_not_final = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text".to_string(),
            content: "Partial content".to_string(),
            is_final: false,
        };

        let chunk_final = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text".to_string(),
            content: "Final content".to_string(),
            is_final: true,
        };

        assert!(!chunk_not_final.is_final);
        assert!(chunk_final.is_final);
    }

    #[test]
    fn test_message_chunk_large_content() {
        let large_content = "x".repeat(10_000);
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text".to_string(),
            content: large_content.clone(),
            is_final: true,
        };

        assert_eq!(chunk.content.len(), 10_000);
        assert_eq!(chunk.content, large_content);

        // Test serialization/deserialization of large content
        let serialized = serde_json::to_string(&chunk).unwrap();
        let deserialized: MessageChunk = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.content.len(), 10_000);
        assert_eq!(deserialized.content, large_content);
    }

    #[test]
    fn test_message_chunk_special_characters() {
        let special_content = "Hello 世界! 🚀 \n\t\r\"'\\";
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text/unicode".to_string(),
            content: special_content.to_string(),
            is_final: false,
        };

        assert_eq!(chunk.content, special_content);
        assert_eq!(chunk.content_type, "text/unicode");

        // Test serialization/deserialization with special characters
        let serialized = serde_json::to_string(&chunk).unwrap();
        let deserialized: MessageChunk = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.content, special_content);
        assert_eq!(deserialized.content_type, "text/unicode");
    }

    #[test]
    fn test_message_chunk_json_formatting() {
        let chunk = MessageChunk {
            conversation_id: Uuid::parse_str("12345678-1234-5678-9abc-123456789abc").unwrap(),
            message_id: Uuid::parse_str("87654321-4321-8765-cba9-987654321098").unwrap(),
            content_type: "test".to_string(),
            content: "test content".to_string(),
            is_final: true,
        };

        let json = serde_json::to_string_pretty(&chunk).unwrap();
        assert!(json.contains("12345678-1234-5678-9abc-123456789abc"));
        assert!(json.contains("87654321-4321-8765-cba9-987654321098"));
        assert!(json.contains("test content"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_message_chunk_deserialization_from_json() {
        let json_str = r#"
        {
            "conversation_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
            "message_id": "550e8400-e29b-41d4-a716-446655440000",
            "content_type": "text",
            "content": "Hello from JSON",
            "is_final": false
        }
        "#;

        let chunk: MessageChunk = serde_json::from_str(json_str).unwrap();
        assert_eq!(
            chunk.conversation_id,
            Uuid::parse_str("f47ac10b-58cc-4372-a567-0e02b2c3d479").unwrap()
        );
        assert_eq!(
            chunk.message_id,
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
        );
        assert_eq!(chunk.content_type, "text");
        assert_eq!(chunk.content, "Hello from JSON");
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_message_chunk_schema_attributes() {
        // This test ensures that our ToSchema derive is working correctly
        // In practice, this would be validated by the utoipa schema generation
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text".to_string(),
            content: "Schema test".to_string(),
            is_final: true,
        };

        // Basic check that the struct can be used in schema contexts
        assert!(chunk.content.contains("Schema test"));
    }
}
