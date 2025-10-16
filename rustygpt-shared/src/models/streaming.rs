use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::timestamp::Timestamp;

/// Represents a persisted streaming chunk for SSE replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct MessageChunk {
    /// Message identifier the chunk belongs to.
    pub message_id: Uuid,
    /// Sequential chunk index starting at 0.
    pub idx: i32,
    /// Chunk content emitted by the provider.
    pub content: String,
    /// Timestamp for when the chunk was stored.
    pub created_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn message_chunk_round_trip() {
        let chunk = MessageChunk {
            message_id: Uuid::new_v4(),
            idx: 0,
            content: "hello".into(),
            created_at: Timestamp(Utc.with_ymd_and_hms(2024, 5, 1, 12, 0, 0).unwrap()),
        };

        let serialized = serde_json::to_string(&chunk).unwrap();
        let deserialized: MessageChunk = serde_json::from_str(&serialized).unwrap();

        assert_eq!(chunk, deserialized);
    }
}
