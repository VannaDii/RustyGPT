use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Timestamp;

/// Represents a single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    /// Unique identifier for the message.
    pub id: Uuid,

    /// ID of the user who sent the message.
    pub sender_id: Uuid,

    /// ID of the conversation this message belongs to.
    pub conversation_id: Uuid,

    /// The message content.
    pub content: String,

    /// Timestamp when the message was sent.
    pub timestamp: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use serde_json;
    use uuid::Uuid;

    #[test]
    fn test_message_creation() {
        let message = Message {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            content: "Hello, world!".to_string(),
            timestamp: Timestamp(Utc::now()),
        };

        assert_eq!(message.content, "Hello, world!");
        assert!(!message.id.is_nil());
        assert!(!message.sender_id.is_nil());
        assert!(!message.conversation_id.is_nil());
    }

    #[test]
    fn test_message_equality() {
        let id = Uuid::new_v4();
        let sender_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let dt = Utc.with_ymd_and_hms(2025, 3, 8, 14, 30, 0).unwrap();
        let timestamp = Timestamp(dt);

        let message1 = Message {
            id,
            sender_id,
            conversation_id,
            content: "Hello, world!".to_string(),
            timestamp: timestamp.clone(),
        };

        let message2 = Message {
            id,
            sender_id,
            conversation_id,
            content: "Hello, world!".to_string(),
            timestamp: timestamp.clone(),
        };

        let message3 = Message {
            id: Uuid::new_v4(), // Different ID
            sender_id,
            conversation_id,
            content: "Hello, world!".to_string(),
            timestamp: timestamp.clone(),
        };

        assert_eq!(message1, message2);
        assert_ne!(message1, message3);
    }

    #[test]
    fn test_message_serialization() {
        let id = Uuid::parse_str("f47ac10b-58cc-4372-a567-0e02b2c3d479").unwrap();
        let sender_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let conversation_id = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
        let dt = Utc.with_ymd_and_hms(2025, 3, 8, 14, 30, 0).unwrap();

        let message = Message {
            id,
            sender_id,
            conversation_id,
            content: "Test message".to_string(),
            timestamp: Timestamp(dt),
        };

        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, message);
        assert_eq!(deserialized.id, id);
        assert_eq!(deserialized.sender_id, sender_id);
        assert_eq!(deserialized.conversation_id, conversation_id);
        assert_eq!(deserialized.content, "Test message");
        assert_eq!(deserialized.timestamp.0, dt);
    }

    #[test]
    fn test_message_with_empty_content() {
        let message = Message {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            content: "".to_string(),
            timestamp: Timestamp(Utc::now()),
        };

        assert_eq!(message.content, "");
        assert!(message.content.is_empty());
    }
}
