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
    use chrono::Utc;
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
    }
}
