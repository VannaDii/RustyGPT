//! Conversation management service layer.
//!
//! This module provides high-level conversation operations that interact with the database
//! through stored procedures. It includes methods for creating conversations, adding participants,
//! and managing conversation metadata.

use anyhow::Result;
use shared::models::conversation::{Conversation, CreateConversationRequest};
use sqlx::PgPool;
use uuid::Uuid;

/// Service for managing conversation operations.
///
/// This service provides methods for creating and managing conversations,
/// including participant management and conversation metadata updates.
#[derive(Debug, Clone)]
pub struct ConversationService {
    /// Database connection pool for executing queries.
    pool: PgPool,
}

impl ConversationService {
    /// Creates a new conversation service with the given database pool.
    ///
    /// # Arguments
    /// * `pool` - A PostgreSQL connection pool for database operations
    ///
    /// # Returns
    /// A new [`ConversationService`] instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates a new conversation.
    ///
    /// # Arguments
    /// * `request` - The conversation creation request with title and initial participant
    ///
    /// # Returns
    /// The UUID of the newly created conversation.
    ///
    /// # Errors
    /// Returns an error if the conversation creation fails or the database query fails.
    pub async fn create_conversation(&self, request: CreateConversationRequest) -> Result<Uuid> {
        let conversation_id =
            sqlx::query_scalar::<_, Option<Uuid>>("SELECT create_conversation($1, $2)")
                .bind(request.title)
                .bind(request.creator_id)
                .fetch_one(&self.pool)
                .await?;

        conversation_id.ok_or_else(|| anyhow::anyhow!("Failed to create conversation"))
    }

    /// Adds a participant to an existing conversation.
    ///
    /// # Arguments
    /// * `conversation_id` - The UUID of the conversation
    /// * `user_id` - The UUID of the user to add as a participant
    ///
    /// # Returns
    /// `Ok(())` if the participant was successfully added.
    ///
    /// # Errors
    /// Returns an error if the participant addition fails or the database query fails.
    pub async fn add_participant(&self, conversation_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query("SELECT add_conversation_participant($1, $2)")
            .bind(conversation_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Retrieves a conversation by its ID.
    ///
    /// # Arguments
    /// * `conversation_id` - The UUID of the conversation to retrieve
    /// * `user_id` - The UUID of the user requesting the conversation (for access control)
    ///
    /// # Returns
    /// The [`Conversation`] if found and accessible to the user.
    ///
    /// # Errors
    /// Returns an error if the conversation is not found, not accessible, or the database query fails.
    pub async fn get_conversation(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<Conversation> {
        #[derive(sqlx::FromRow)]
        struct ConversationRow {
            id: Uuid,
            title: String,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let result = sqlx::query_as::<_, ConversationRow>(
            "SELECT id, title, created_at FROM get_conversation($1, $2)",
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let row =
            result.ok_or_else(|| anyhow::anyhow!("Conversation not found or access denied"))?;

        // Note: This is a minimal conversation representation for basic operations
        // Full conversation loading with messages and participants would require additional queries
        Ok(Conversation {
            id: row.id,
            title: row.title,
            participant_ids: vec![], // TODO: Load participants in separate query
            messages: vec![],        // TODO: Load messages in separate query
            last_updated: shared::models::timestamp::Timestamp(row.created_at),
        })
    }

    /// Lists all conversations for a user.
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user whose conversations to retrieve
    ///
    /// # Returns
    /// A vector of [`Conversation`] objects accessible to the user.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn list_user_conversations(&self, user_id: Uuid) -> Result<Vec<Conversation>> {
        #[derive(sqlx::FromRow)]
        struct ConversationRow {
            id: Uuid,
            title: String,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let rows = sqlx::query_as::<_, ConversationRow>(
            "SELECT id, title, created_at FROM list_user_conversations($1)",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let conversations = rows
            .into_iter()
            .map(|row| Conversation {
                id: row.id,
                title: row.title,
                participant_ids: vec![], // TODO: Load participants in separate query
                messages: vec![],        // TODO: Load messages in separate query
                last_updated: shared::models::timestamp::Timestamp(row.created_at),
            })
            .collect();

        Ok(conversations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_conversation_service_creation() {
        // Create service without database pool for testing
        let pool = sqlx::PgPool::connect_lazy("postgresql://test:test@localhost/test")
            .expect("Failed to create test pool");

        let _service = ConversationService::new(pool);
        // Service creation test - if we reach here without panicking, the test passes
    }
}
