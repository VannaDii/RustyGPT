use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{Message, Timestamp};

/// Represents a conversation between multiple users.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct Conversation {
    /// The title of the conversation.
    pub title: String,

    /// Unique identifier for the conversation.
    pub id: Uuid,

    /// The users participating in this conversation.
    pub participant_ids: Vec<Uuid>,

    /// The messages in this conversation.
    pub messages: Vec<Message>,

    /// Timestamp of the last message in the conversation.
    pub last_updated: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct SendMessageRequest {
    pub content: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct SendMessageResponse {
    /// The assistant's response message echoing the user's input.
    pub message: Message,
}

/// Request structure for creating a new conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct CreateConversationRequest {
    /// The title of the conversation.
    pub title: String,

    /// The UUID of the user creating the conversation.
    pub creator_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MessageType;
    use chrono::{TimeZone, Utc};
    use serde_json;
    use uuid::Uuid;

    #[test]
    fn test_conversation_creation() {
        let conversation = Conversation {
            id: Uuid::new_v4(),
            title: "Sample Chat".into(),
            participant_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
            messages: vec![],
            last_updated: Timestamp(Utc::now()),
        };

        assert_eq!(conversation.participant_ids.len(), 2);
        assert!(!conversation.id.is_nil());
        assert_eq!(conversation.title, "Sample Chat");
        assert!(conversation.messages.is_empty());
    }

    #[test]
    fn test_conversation_empty_participants() {
        let conversation = Conversation {
            id: Uuid::new_v4(),
            title: "Sample Chat".into(),
            participant_ids: vec![],
            messages: vec![],
            last_updated: Timestamp(Utc::now()),
        };

        assert!(conversation.participant_ids.is_empty());
    }

    #[test]
    fn test_conversation_with_messages() {
        let user_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let dt = Utc.with_ymd_and_hms(2025, 3, 8, 14, 30, 0).unwrap();
        let timestamp = Timestamp(dt);

        let message1 = Message {
            id: Uuid::new_v4(),
            sender_id: user_id,
            conversation_id,
            content: "Hello".to_string(),
            message_type: MessageType::User,
            timestamp: timestamp.clone(),
        };

        let message2 = Message {
            id: Uuid::new_v4(),
            sender_id: user_id,
            conversation_id,
            content: "World".to_string(),
            message_type: MessageType::Assistant,
            timestamp: timestamp.clone(),
        };

        let conversation = Conversation {
            id: conversation_id,
            title: "Test Conversation".into(),
            participant_ids: vec![user_id],
            messages: vec![message1.clone(), message2.clone()],
            last_updated: timestamp.clone(),
        };

        assert_eq!(conversation.messages.len(), 2);
        assert_eq!(conversation.messages[0], message1);
        assert_eq!(conversation.messages[1], message2);
        assert_eq!(conversation.participant_ids.len(), 1);
        assert_eq!(conversation.participant_ids[0], user_id);
    }

    #[test]
    fn test_conversation_equality() {
        let id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let dt = Utc.with_ymd_and_hms(2025, 3, 8, 14, 30, 0).unwrap();
        let timestamp = Timestamp(dt);

        let conversation1 = Conversation {
            id,
            title: "Test".into(),
            participant_ids: vec![user_id],
            messages: vec![],
            last_updated: timestamp.clone(),
        };

        let conversation2 = Conversation {
            id,
            title: "Test".into(),
            participant_ids: vec![user_id],
            messages: vec![],
            last_updated: timestamp.clone(),
        };

        let conversation3 = Conversation {
            id: Uuid::new_v4(), // Different ID
            title: "Test".into(),
            participant_ids: vec![user_id],
            messages: vec![],
            last_updated: timestamp.clone(),
        };

        assert_eq!(conversation1, conversation2);
        assert_ne!(conversation1, conversation3);
    }

    #[test]
    fn test_conversation_serialization() {
        let id = Uuid::parse_str("f47ac10b-58cc-4372-a567-0e02b2c3d479").unwrap();
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let dt = Utc.with_ymd_and_hms(2025, 3, 8, 14, 30, 0).unwrap();

        let conversation = Conversation {
            id,
            title: "Test Conversation".into(),
            participant_ids: vec![user_id],
            messages: vec![],
            last_updated: Timestamp(dt),
        };

        let serialized = serde_json::to_string(&conversation).unwrap();
        let deserialized: Conversation = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, conversation);
        assert_eq!(deserialized.id, id);
        assert_eq!(deserialized.title, "Test Conversation");
        assert_eq!(deserialized.participant_ids, vec![user_id]);
        assert!(deserialized.messages.is_empty());
        assert_eq!(deserialized.last_updated.0, dt);
    }

    #[test]
    fn test_conversation_with_empty_title() {
        let conversation = Conversation {
            id: Uuid::new_v4(),
            title: "".into(),
            participant_ids: vec![Uuid::new_v4()],
            messages: vec![],
            last_updated: Timestamp(Utc::now()),
        };

        assert_eq!(conversation.title, "");
        assert!(conversation.title.is_empty());
    }
}
