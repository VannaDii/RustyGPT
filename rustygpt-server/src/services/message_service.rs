//! Message management service layer.
//!
//! This module provides high-level message operations that interact with the database
//! through stored procedures. It includes methods for creating messages, retrieving
//! conversation messages, and managing message metadata.

use anyhow::Result;
use shared::models::message::{CreateMessageRequest, Message, MessageType};
use sqlx::PgPool;
use uuid::Uuid;

/// Service for managing message operations.
///
/// This service provides methods for creating and retrieving messages
/// within conversations, with proper access control.
#[derive(Debug, Clone)]
pub struct MessageService {
    /// Database connection pool for executing queries.
    pool: PgPool,
}

impl MessageService {
    /// Creates a new message service with the given database pool.
    ///
    /// # Arguments
    /// * `pool` - A PostgreSQL connection pool for database operations
    ///
    /// # Returns
    /// A new [`MessageService`] instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates a new message in a conversation.
    ///
    /// # Arguments
    /// * `request` - The message creation request with conversation, content, and metadata
    ///
    /// # Returns
    /// The UUID of the newly created message.
    ///
    /// # Errors
    /// Returns an error if the message creation fails, user doesn't have access to the conversation,
    /// or the database query fails.
    pub async fn create_message(&self, request: CreateMessageRequest) -> Result<Uuid> {
        let message_id =
            sqlx::query_scalar::<_, Option<Uuid>>("SELECT create_message($1, $2, $3, $4)")
                .bind(request.conversation_id)
                .bind(request.sender_id)
                .bind(request.content)
                .bind(request.message_type.to_string())
                .fetch_one(&self.pool)
                .await?;

        message_id.ok_or_else(|| anyhow::anyhow!("Failed to create message"))
    }

    /// Retrieves all messages for a conversation.
    ///
    /// # Arguments
    /// * `conversation_id` - The UUID of the conversation
    /// * `user_id` - The UUID of the user requesting the messages (for access control)
    ///
    /// # Returns
    /// A vector of [`Message`] objects in the conversation.
    ///
    /// # Errors
    /// Returns an error if the user doesn't have access to the conversation or the database query fails.
    pub async fn get_conversation_messages(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Message>> {
        #[derive(sqlx::FromRow)]
        struct MessageRow {
            id: Uuid,
            conversation_id: Uuid,
            sender_id: Uuid,
            content: String,
            message_type: String,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, conversation_id, sender_id, content, message_type, created_at FROM get_conversation_messages($1, $2)"
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let messages = rows
            .into_iter()
            .map(|row| {
                let message_type = match row.message_type.as_str() {
                    "user" => MessageType::User,
                    "assistant" => MessageType::Assistant,
                    "system" => MessageType::System,
                    _ => MessageType::User, // Default fallback
                };

                Message {
                    id: row.id,
                    conversation_id: row.conversation_id,
                    sender_id: row.sender_id,
                    content: row.content,
                    message_type,
                    timestamp: shared::models::timestamp::Timestamp(row.created_at),
                }
            })
            .collect();

        Ok(messages)
    }

    /// Retrieves a specific message by ID.
    ///
    /// # Arguments
    /// * `message_id` - The UUID of the message to retrieve
    /// * `user_id` - The UUID of the user requesting the message (for access control)
    ///
    /// # Returns
    /// The [`Message`] if found and accessible to the user.
    ///
    /// # Errors
    /// Returns an error if the message is not found, not accessible, or the database query fails.
    pub async fn get_message(&self, message_id: Uuid, user_id: Uuid) -> Result<Message> {
        #[derive(sqlx::FromRow)]
        struct MessageRow {
            id: Uuid,
            conversation_id: Uuid,
            sender_id: Uuid,
            content: String,
            message_type: String,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let result = sqlx::query_as::<_, MessageRow>(
            "SELECT id, conversation_id, sender_id, content, message_type, created_at FROM get_message($1, $2)"
        )
        .bind(message_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let row = result.ok_or_else(|| anyhow::anyhow!("Message not found or access denied"))?;

        let message_type = match row.message_type.as_str() {
            "user" => MessageType::User,
            "assistant" => MessageType::Assistant,
            "system" => MessageType::System,
            _ => MessageType::User, // Default fallback
        };

        Ok(Message {
            id: row.id,
            conversation_id: row.conversation_id,
            sender_id: row.sender_id,
            content: row.content,
            message_type,
            timestamp: shared::models::timestamp::Timestamp(row.created_at),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_service_creation() {
        // Create service without database pool for testing
        let pool = sqlx::PgPool::connect_lazy("postgresql://test:test@localhost/test")
            .expect("Failed to create test pool");

        let _service = MessageService::new(pool);
        // Simply test that the service was created without panicking
        assert!(true);
    }
}
