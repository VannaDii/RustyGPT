use std::fmt;

use chrono::{DateTime, Utc};
use shared::models::timestamp::Timestamp;
use shared::models::{
    AddParticipantRequest, ConversationCreateRequest, ConversationCreateResponse, ConversationRole,
    CreateInviteResponse, MessageChunk, MessageRole, MessageView, PostRootMessageRequest,
    PostRootMessageResponse, PresenceStatus, ReplyMessageRequest, ReplyMessageResponse,
    ThreadListResponse, ThreadSummary, ThreadTreeResponse, UnreadThreadSummary,
};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ChatServiceError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("rate limited: {0}")]
    RateLimited(String),
}

impl ChatServiceError {
    fn from_db_error(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db) = &err {
            let message = db.message();
            if message.contains("RGP.401") {
                return ChatServiceError::Forbidden(message.to_string());
            }
            if message.contains("RGP.403") {
                return ChatServiceError::Forbidden(message.to_string());
            }
            if message.contains("RGP.404") {
                return ChatServiceError::NotFound(message.to_string());
            }
            if message.contains("RGP.VALIDATION") {
                return ChatServiceError::Validation(message.to_string());
            }
            if message.contains("RGP.429") {
                return ChatServiceError::RateLimited(message.to_string());
            }
        }
        ChatServiceError::Database(err)
    }
}

pub type ChatServiceResult<T> = Result<T, ChatServiceError>;

#[derive(Clone)]
pub struct ChatService {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct ThreadSummaryWithConversation {
    pub conversation_id: Uuid,
    pub summary: ThreadSummary,
}

impl fmt::Debug for ChatService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChatService").finish()
    }
}

#[derive(Debug, Clone)]
pub struct AcceptInviteResult {
    pub conversation_id: Uuid,
    pub role: ConversationRole,
}

impl ChatService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn begin_for(&self, user_id: Uuid) -> ChatServiceResult<Transaction<'_, Postgres>> {
        let mut tx = self.pool.begin().await.map_err(ChatServiceError::from)?;
        sqlx::query("SET LOCAL app.current_user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(ChatServiceError::from)?;
        Ok(tx)
    }

    #[instrument(name = "chat.create_conversation", skip(self), err)]
    pub async fn create_conversation(
        &self,
        actor: Uuid,
        request: ConversationCreateRequest,
    ) -> ChatServiceResult<ConversationCreateResponse> {
        let mut tx = self.begin_for(actor).await?;

        let ConversationCreateRequest { title, is_group } = request;

        let conversation_id: Uuid =
            sqlx::query_scalar("SELECT rustygpt.sp_create_conversation($1, $2, $3)")
                .bind(&title)
                .bind(is_group)
                .bind(actor)
                .fetch_one(&mut *tx)
                .await
                .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        Ok(ConversationCreateResponse { conversation_id })
    }

    #[instrument(name = "chat.add_participant", skip(self, request), err)]
    pub async fn add_participant(
        &self,
        actor: Uuid,
        conversation_id: Uuid,
        request: AddParticipantRequest,
    ) -> ChatServiceResult<ConversationRole> {
        let mut tx = self.begin_for(actor).await?;
        let role: String = sqlx::query_scalar(
            r#"SELECT rustygpt.sp_add_participant($1, $2, $3::rustygpt.conversation_role)::TEXT"#,
        )
        .bind(conversation_id)
        .bind(request.user_id)
        .bind(request.role.as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;
        let role = ConversationRole::try_from(role.as_str()).unwrap_or(ConversationRole::Member);
        Ok(role)
    }

    #[instrument(name = "chat.remove_participant", skip(self), err)]
    pub async fn remove_participant(
        &self,
        actor: Uuid,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> ChatServiceResult<ConversationRole> {
        let mut tx = self.begin_for(actor).await?;
        let role: String =
            sqlx::query_scalar("SELECT rustygpt.sp_remove_participant($1, $2)::TEXT")
                .bind(conversation_id)
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(ChatServiceError::from_db_error)?;
        tx.commit().await.map_err(ChatServiceError::from)?;
        let role = ConversationRole::try_from(role.as_str()).unwrap_or(ConversationRole::Member);
        Ok(role)
    }

    #[instrument(name = "chat.create_invite", skip(self), err)]
    pub async fn create_invite(
        &self,
        actor: Uuid,
        conversation_id: Uuid,
        email: &str,
        role: ConversationRole,
        ttl_seconds: Option<i32>,
    ) -> ChatServiceResult<CreateInviteResponse> {
        let mut tx = self.begin_for(actor).await?;

        #[derive(sqlx::FromRow)]
        struct InviteRow {
            token: String,
            expires_at: DateTime<Utc>,
        }

        let row = sqlx::query_as::<_, InviteRow>(
            "SELECT token, expires_at FROM rustygpt.sp_create_invite($1, $2, $3, $4)",
        )
        .bind(conversation_id)
        .bind(email)
        .bind(role.as_str())
        .bind(ttl_seconds.unwrap_or(86_400))
        .fetch_one(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        Ok(CreateInviteResponse {
            token: row.token,
            expires_at: Timestamp(row.expires_at),
        })
    }

    #[instrument(name = "chat.accept_invite", skip(self), err)]
    pub async fn accept_invite(
        &self,
        actor: Uuid,
        token: &str,
    ) -> ChatServiceResult<AcceptInviteResult> {
        let mut tx = self.begin_for(actor).await?;

        #[derive(sqlx::FromRow)]
        struct InviteRow {
            conversation_id: Uuid,
            role: String,
        }

        let row = sqlx::query_as::<_, InviteRow>(
            "SELECT conversation_id, role::TEXT AS role FROM rustygpt.sp_accept_invite($1, $2)",
        )
        .bind(token)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        let role =
            ConversationRole::try_from(row.role.as_str()).unwrap_or(ConversationRole::Member);
        Ok(AcceptInviteResult {
            conversation_id: row.conversation_id,
            role,
        })
    }

    #[instrument(name = "chat.revoke_invite", skip(self), err)]
    pub async fn revoke_invite(&self, actor: Uuid, token: &str) -> ChatServiceResult<()> {
        let mut tx = self.begin_for(actor).await?;
        sqlx::query("SELECT rustygpt.sp_revoke_invite($1)")
            .bind(token)
            .execute(&mut *tx)
            .await
            .map_err(ChatServiceError::from_db_error)?;
        tx.commit().await.map_err(ChatServiceError::from)?;
        Ok(())
    }

    #[instrument(name = "chat.list_threads", skip(self), err)]
    pub async fn list_threads(
        &self,
        actor: Uuid,
        conversation_id: Uuid,
        after: Option<DateTime<Utc>>,
        limit: Option<i32>,
    ) -> ChatServiceResult<ThreadListResponse> {
        let mut tx = self.begin_for(actor).await?;

        #[derive(sqlx::FromRow)]
        struct ThreadRow {
            root_id: Uuid,
            root_author: Option<Uuid>,
            root_excerpt: Option<String>,
            created_at: DateTime<Utc>,
            last_activity_at: DateTime<Utc>,
            message_count: i64,
            participant_count: i64,
        }

        let rows = sqlx::query_as::<_, ThreadRow>(
            "SELECT root_id, root_author, root_excerpt, created_at, last_activity_at, message_count, participant_count
             FROM rustygpt.sp_list_threads($1, $2, $3)"
        )
        .bind(conversation_id)
        .bind(after)
        .bind(limit.unwrap_or(50))
        .fetch_all(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        let threads: Vec<ThreadSummary> = rows
            .into_iter()
            .map(|row| ThreadSummary {
                root_id: row.root_id,
                root_excerpt: row.root_excerpt.unwrap_or_default(),
                root_author: row.root_author,
                created_at: Timestamp(row.created_at),
                last_activity_at: Timestamp(row.last_activity_at),
                message_count: row.message_count,
                participant_count: row.participant_count,
            })
            .collect();

        let next_after = threads.last().map(|item| item.last_activity_at.clone());

        Ok(ThreadListResponse {
            threads,
            next_after,
        })
    }

    #[instrument(name = "chat.thread_summary", skip(self), err)]
    pub async fn get_thread_summary(
        &self,
        actor: Uuid,
        root_id: Uuid,
    ) -> ChatServiceResult<ThreadSummaryWithConversation> {
        let mut tx = self.begin_for(actor).await?;

        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            conversation_id: Uuid,
            root_id: Uuid,
            root_author: Option<Uuid>,
            root_excerpt: Option<String>,
            created_at: DateTime<Utc>,
            last_activity_at: DateTime<Utc>,
            message_count: i64,
            participant_count: i64,
        }

        let row = sqlx::query_as::<_, SummaryRow>(
            "SELECT conversation_id, root_id, root_author, root_excerpt, created_at, last_activity_at, message_count, participant_count
             FROM rustygpt.sp_get_thread_summary($1)"
        )
        .bind(root_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        let row =
            row.ok_or_else(|| ChatServiceError::NotFound("thread summary not found".into()))?;

        let summary = ThreadSummary {
            root_id: row.root_id,
            root_excerpt: row.root_excerpt.unwrap_or_default(),
            root_author: row.root_author,
            created_at: Timestamp(row.created_at),
            last_activity_at: Timestamp(row.last_activity_at),
            message_count: row.message_count,
            participant_count: row.participant_count,
        };

        Ok(ThreadSummaryWithConversation {
            conversation_id: row.conversation_id,
            summary,
        })
    }

    #[instrument(name = "chat.thread_subtree", skip(self), err)]
    pub async fn get_thread_subtree(
        &self,
        actor: Uuid,
        root_id: Uuid,
        cursor_path: Option<String>,
        limit: Option<i32>,
    ) -> ChatServiceResult<ThreadTreeResponse> {
        let mut tx = self.begin_for(actor).await?;

        #[derive(sqlx::FromRow)]
        struct MessageRow {
            id: Uuid,
            root_id: Uuid,
            parent_id: Option<Uuid>,
            conversation_id: Uuid,
            author_user_id: Option<Uuid>,
            role: String,
            content: String,
            path: String,
            depth: i32,
            created_at: DateTime<Utc>,
        }

        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, root_id, parent_id, conversation_id, author_user_id, role::TEXT AS role, content, path, depth, created_at
             FROM rustygpt.sp_get_thread_subtree($1, $2, $3)"
        )
        .bind(root_id)
        .bind(cursor_path.clone())
        .bind(limit.unwrap_or(200))
        .fetch_all(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        let messages: Vec<MessageView> = rows
            .into_iter()
            .map(|row| {
                let role = MessageRole::try_from(row.role.as_str()).unwrap_or(MessageRole::User);
                MessageView {
                    id: row.id,
                    root_id: row.root_id,
                    parent_id: row.parent_id,
                    conversation_id: row.conversation_id,
                    author_user_id: row.author_user_id,
                    role,
                    content: row.content,
                    path: row.path,
                    depth: row.depth,
                    created_at: Timestamp(row.created_at),
                }
            })
            .collect();

        let next_cursor = messages.last().map(|m| m.path.clone());

        Ok(ThreadTreeResponse {
            root_id,
            messages,
            next_cursor,
        })
    }

    #[instrument(name = "chat.get_message", skip(self), err)]
    pub async fn get_message(
        &self,
        actor: Uuid,
        message_id: Uuid,
    ) -> ChatServiceResult<MessageView> {
        let mut tx = self.begin_for(actor).await?;

        #[derive(sqlx::FromRow)]
        struct MessageRow {
            id: Uuid,
            root_id: Uuid,
            parent_id: Option<Uuid>,
            conversation_id: Uuid,
            author_user_id: Option<Uuid>,
            role: String,
            content: String,
            path: String,
            depth: i32,
            created_at: DateTime<Utc>,
        }

        let row = sqlx::query_as::<_, MessageRow>(
            "SELECT id,
                    root_message_id AS root_id,
                    parent_message_id AS parent_id,
                    conversation_id,
                    author_user_id,
                    role::TEXT AS role,
                    content,
                    path::TEXT AS path,
                    depth,
                    created_at
             FROM rustygpt.messages
             WHERE id = $1",
        )
        .bind(message_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        let row = row
            .ok_or_else(|| ChatServiceError::NotFound(format!("message {message_id} not found")))?;

        let role = MessageRole::try_from(row.role.as_str()).unwrap_or(MessageRole::User);
        Ok(MessageView {
            id: row.id,
            root_id: row.root_id,
            parent_id: row.parent_id,
            conversation_id: row.conversation_id,
            author_user_id: row.author_user_id,
            role,
            content: row.content,
            path: row.path,
            depth: row.depth,
            created_at: Timestamp(row.created_at),
        })
    }

    #[instrument(name = "chat.thread_ancestors", skip(self), err)]
    pub async fn get_ancestor_chain(
        &self,
        actor: Uuid,
        root_id: Uuid,
        target_path: &str,
    ) -> ChatServiceResult<Vec<MessageView>> {
        let mut tx = self.begin_for(actor).await?;

        #[derive(sqlx::FromRow)]
        struct AncestorRow {
            id: Uuid,
            root_id: Uuid,
            parent_id: Option<Uuid>,
            conversation_id: Uuid,
            author_user_id: Option<Uuid>,
            role: String,
            content: String,
            path: String,
            depth: i32,
            created_at: DateTime<Utc>,
        }

        let rows = sqlx::query_as::<_, AncestorRow>(
            "SELECT id,
                    root_message_id AS root_id,
                    parent_message_id AS parent_id,
                    conversation_id,
                    author_user_id,
                    role::TEXT AS role,
                    content,
                    path::TEXT AS path,
                    depth,
                    created_at
             FROM rustygpt.messages
             WHERE root_message_id = $1
               AND path @> $2::ltree
             ORDER BY path",
        )
        .bind(root_id)
        .bind(target_path)
        .fetch_all(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        let messages = rows
            .into_iter()
            .map(|row| {
                let role = MessageRole::try_from(row.role.as_str()).unwrap_or(MessageRole::User);
                MessageView {
                    id: row.id,
                    root_id: row.root_id,
                    parent_id: row.parent_id,
                    conversation_id: row.conversation_id,
                    author_user_id: row.author_user_id,
                    role,
                    content: row.content,
                    path: row.path,
                    depth: row.depth,
                    created_at: Timestamp(row.created_at),
                }
            })
            .collect();

        Ok(messages)
    }

    #[instrument(name = "chat.post_root", skip(self, request), err)]
    pub async fn post_root_message(
        &self,
        actor: Uuid,
        conversation_id: Uuid,
        request: PostRootMessageRequest,
    ) -> ChatServiceResult<PostRootMessageResponse> {
        let mut tx = self.begin_for(actor).await?;
        #[derive(sqlx::FromRow)]
        struct ResponseRow {
            message_id: Uuid,
            root_id: Uuid,
            conversation_id: Uuid,
            depth: i32,
        }

        let role = request.role.unwrap_or(MessageRole::User);

        let row = sqlx::query_as::<_, ResponseRow>(
            "SELECT message_id, root_id, conversation_id, depth FROM rustygpt.sp_post_root_message($1, $2, $3::rustygpt.message_role, $4)"
        )
        .bind(conversation_id)
        .bind(actor)
        .bind(role.as_str())
        .bind(request.content)
        .fetch_one(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        Ok(PostRootMessageResponse {
            message_id: row.message_id,
            root_id: row.root_id,
            conversation_id: row.conversation_id,
            depth: row.depth,
        })
    }

    #[instrument(name = "chat.reply", skip(self, request), err)]
    pub async fn reply_message(
        &self,
        actor: Uuid,
        parent_message: Uuid,
        request: ReplyMessageRequest,
    ) -> ChatServiceResult<ReplyMessageResponse> {
        self.reply_with_author(
            actor,
            parent_message,
            Some(actor),
            request.role.unwrap_or(MessageRole::User),
            request.content,
        )
        .await
    }

    #[instrument(name = "chat.reply.assistant", skip(self, content), err)]
    pub async fn reply_as_assistant(
        &self,
        actor: Uuid,
        parent_message: Uuid,
        content: String,
    ) -> ChatServiceResult<ReplyMessageResponse> {
        self.reply_with_author(actor, parent_message, None, MessageRole::Assistant, content)
            .await
    }

    async fn reply_with_author(
        &self,
        actor: Uuid,
        parent_message: Uuid,
        author: Option<Uuid>,
        role: MessageRole,
        content: String,
    ) -> ChatServiceResult<ReplyMessageResponse> {
        let mut tx = self.begin_for(actor).await?;
        #[derive(sqlx::FromRow)]
        struct ResponseRow {
            message_id: Uuid,
            root_id: Uuid,
            conversation_id: Uuid,
            parent_id: Option<Uuid>,
            depth: i32,
        }

        let row = sqlx::query_as::<_, ResponseRow>(
            "SELECT message_id, root_id, conversation_id, parent_id, depth FROM rustygpt.sp_reply_message($1, $2, $3::rustygpt.message_role, $4)"
        )
        .bind(parent_message)
        .bind(author)
        .bind(role.as_str())
        .bind(content)
        .fetch_one(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        Ok(ReplyMessageResponse {
            message_id: row.message_id,
            root_id: row.root_id,
            conversation_id: row.conversation_id,
            parent_id: row.parent_id,
            depth: row.depth,
        })
    }

    #[instrument(name = "chat.append_chunk", skip(self), err)]
    pub async fn append_chunk(
        &self,
        actor: Uuid,
        message_id: Uuid,
        idx: i32,
        content: String,
    ) -> ChatServiceResult<()> {
        let mut tx = self.begin_for(actor).await?;
        sqlx::query("SELECT rustygpt.sp_append_message_chunk($1, $2, $3)")
            .bind(message_id)
            .bind(idx)
            .bind(content)
            .execute(&mut *tx)
            .await
            .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;
        Ok(())
    }

    #[instrument(name = "chat.list_chunks", skip(self), err)]
    pub async fn list_chunks(
        &self,
        actor: Uuid,
        message_id: Uuid,
        from_idx: Option<i32>,
        limit: Option<i32>,
    ) -> ChatServiceResult<Vec<MessageChunk>> {
        let mut tx = self.begin_for(actor).await?;
        #[derive(sqlx::FromRow)]
        struct ChunkRow {
            message_id: Uuid,
            idx: i32,
            content: String,
            created_at: DateTime<Utc>,
        }

        let rows = sqlx::query_as::<_, ChunkRow>(
            "SELECT message_id, idx, content, created_at FROM rustygpt.sp_list_message_chunks($1, $2, $3)"
        )
        .bind(message_id)
        .bind(from_idx.unwrap_or(0))
        .bind(limit.unwrap_or(500))
        .fetch_all(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        Ok(rows
            .into_iter()
            .map(|row| MessageChunk {
                message_id: row.message_id,
                idx: row.idx,
                content: row.content,
                created_at: Timestamp(row.created_at),
            })
            .collect())
    }

    #[instrument(name = "chat.update_message", skip(self, content), err)]
    pub async fn update_message_content(
        &self,
        actor: Uuid,
        message_id: Uuid,
        content: String,
    ) -> ChatServiceResult<()> {
        let mut tx = self.begin_for(actor).await?;
        sqlx::query("SELECT rustygpt.sp_update_message_content($1, $2)")
            .bind(message_id)
            .bind(content)
            .execute(&mut *tx)
            .await
            .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;
        Ok(())
    }

    #[instrument(name = "chat.mark_thread_read", skip(self), err)]
    pub async fn mark_thread_read(
        &self,
        actor: Uuid,
        conversation_id: Uuid,
        root_id: Uuid,
        path: Option<&str>,
    ) -> ChatServiceResult<()> {
        let mut tx = self.begin_for(actor).await?;
        sqlx::query("SELECT rustygpt.sp_mark_thread_read($1, $2, $3, $4)")
            .bind(conversation_id)
            .bind(root_id)
            .bind(actor)
            .bind(path)
            .execute(&mut *tx)
            .await
            .map_err(ChatServiceError::from_db_error)?;
        tx.commit().await.map_err(ChatServiceError::from)?;
        Ok(())
    }

    #[instrument(name = "chat.unread_summary", skip(self), err)]
    pub async fn unread_summary(
        &self,
        actor: Uuid,
        conversation_id: Uuid,
    ) -> ChatServiceResult<Vec<UnreadThreadSummary>> {
        let mut tx = self.begin_for(actor).await?;

        #[derive(sqlx::FromRow)]
        struct UnreadRow {
            root_id: Uuid,
            unread: i32,
        }

        let rows = sqlx::query_as::<_, UnreadRow>(
            "SELECT root_id, unread FROM rustygpt.sp_get_unread_summary($1, $2)",
        )
        .bind(conversation_id)
        .bind(actor)
        .fetch_all(&mut *tx)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        Ok(rows
            .into_iter()
            .map(|row| UnreadThreadSummary {
                root_id: row.root_id,
                unread: row.unread as i64,
            })
            .collect())
    }

    #[instrument(name = "chat.set_typing", skip(self), err)]
    pub async fn set_typing(
        &self,
        actor: Uuid,
        conversation_id: Uuid,
        root_id: Uuid,
        seconds: i32,
    ) -> ChatServiceResult<()> {
        let mut tx = self.begin_for(actor).await?;
        sqlx::query("SELECT rustygpt.sp_set_typing($1, $2, $3, $4)")
            .bind(conversation_id)
            .bind(root_id)
            .bind(actor)
            .bind(seconds)
            .execute(&mut *tx)
            .await
            .map_err(ChatServiceError::from_db_error)?;
        tx.commit().await.map_err(ChatServiceError::from)?;
        Ok(())
    }

    #[instrument(name = "chat.heartbeat", skip(self), err)]
    pub async fn heartbeat(
        &self,
        actor: Uuid,
        status: Option<PresenceStatus>,
    ) -> ChatServiceResult<()> {
        let mut tx = self.begin_for(actor).await?;
        let status_str = status.map(|s| match s {
            PresenceStatus::Online => "online",
            PresenceStatus::Away => "away",
            PresenceStatus::Offline => "offline",
        });

        sqlx::query("SELECT rustygpt.sp_heartbeat($1, $2)")
            .bind(actor)
            .bind(status_str)
            .execute(&mut *tx)
            .await
            .map_err(ChatServiceError::from_db_error)?;
        tx.commit().await.map_err(ChatServiceError::from)?;
        Ok(())
    }

    #[instrument(name = "chat.soft_delete_message", skip(self), err)]
    pub async fn soft_delete_message(
        &self,
        actor: Uuid,
        message_id: Uuid,
        reason: Option<String>,
    ) -> ChatServiceResult<()> {
        let mut tx = self.begin_for(actor).await?;
        sqlx::query("SELECT rustygpt.sp_soft_delete_message($1, $2, $3)")
            .bind(message_id)
            .bind(actor)
            .bind(reason)
            .execute(&mut *tx)
            .await
            .map_err(ChatServiceError::from_db_error)?;
        tx.commit().await.map_err(ChatServiceError::from)?;
        Ok(())
    }

    #[instrument(name = "chat.restore_message", skip(self), err)]
    pub async fn restore_message(&self, actor: Uuid, message_id: Uuid) -> ChatServiceResult<()> {
        let mut tx = self.begin_for(actor).await?;
        sqlx::query("SELECT rustygpt.sp_restore_message($1, $2)")
            .bind(message_id)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(ChatServiceError::from_db_error)?;
        tx.commit().await.map_err(ChatServiceError::from)?;
        Ok(())
    }

    #[instrument(name = "chat.edit_message", skip(self), err)]
    pub async fn edit_message(
        &self,
        actor: Uuid,
        message_id: Uuid,
        content: String,
        reason: Option<String>,
    ) -> ChatServiceResult<()> {
        let mut tx = self.begin_for(actor).await?;
        sqlx::query("SELECT rustygpt.sp_edit_message($1, $2, $3, $4)")
            .bind(message_id)
            .bind(actor)
            .bind(&content)
            .bind(reason)
            .execute(&mut *tx)
            .await
            .map_err(ChatServiceError::from_db_error)?;
        tx.commit().await.map_err(ChatServiceError::from)?;
        Ok(())
    }

    pub async fn active_conversations(&self, actor: Uuid) -> ChatServiceResult<Vec<Uuid>> {
        let conversations = sqlx::query_scalar::<_, Uuid>(
            "SELECT conversation_id FROM rustygpt.conversation_participants WHERE user_id = $1 AND left_at IS NULL",
        )
        .bind(actor)
        .fetch_all(&self.pool)
        .await
        .map_err(ChatServiceError::from_db_error)?;

        Ok(conversations)
    }

    pub async fn ensure_membership(
        &self,
        actor: Uuid,
        conversation_id: Uuid,
    ) -> ChatServiceResult<()> {
        let mut tx = self.begin_for(actor).await?;
        let allowed: bool = sqlx::query_scalar("SELECT rustygpt.sp_user_can_access($1, $2)")
            .bind(actor)
            .bind(conversation_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(ChatServiceError::from_db_error)?;

        tx.commit().await.map_err(ChatServiceError::from)?;

        if allowed {
            Ok(())
        } else {
            Err(ChatServiceError::Forbidden(
                "user is not a participant in conversation".to_string(),
            ))
        }
    }
}
