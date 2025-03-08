use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Message, Timestamp};

/// Represents a conversation between multiple users.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
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
}
