use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    handlers::streaming::SharedStreamHub,
    http::error::{ApiError, AppResult},
    middleware::request_context::RequestContext,
    services::{
        assistant_service::{AssistantService, finish_reason_to_string},
        chat_service::{ChatService, ChatServiceError},
    },
};
use futures::StreamExt;
use serde_json::json;
use shared::{
    llms::{
        ThreadContextBuilder,
        types::{LLMConfig, LLMRequest, TokenUsage},
    },
    models::{
        ChatDelta, ChatDeltaChoice, ChatDeltaChunk, ConversationStreamEvent, MarkThreadReadRequest,
        MessageChunk, MessageDeleteRequest, MessageDoneEvent, MessageEditRequest, MessageRole,
        MessageView, PostRootMessageRequest, PresenceHeartbeatRequest, ReplyMessageRequest,
        ReplyMessageResponse, StreamErrorEvent, ThreadActivityEvent, ThreadNewEvent,
        ThreadTreeResponse, Timestamp, TypingRequest, UnreadUpdateEvent, UsageBreakdown,
    },
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/threads/{root_id}/tree", get(thread_tree))
        .route("/api/threads/{conversation_id}/root", post(post_root))
        .route("/api/messages/{parent_id}/reply", post(reply_message))
        .route("/api/messages/{message_id}/chunks", get(message_chunks))
        .route("/api/threads/{root_id}/read", post(mark_thread_read))
        .route("/api/messages/{message_id}/delete", post(delete_message))
        .route("/api/messages/{message_id}/restore", post(restore_message))
        .route("/api/messages/{message_id}/edit", post(edit_message))
        .route("/api/typing", post(set_typing))
        .route("/api/presence/heartbeat", post(presence_heartbeat))
}

#[derive(Debug, Deserialize, Default)]
struct ThreadTreeQuery {
    cursor_path: Option<String>,
    limit: Option<i32>,
}

#[derive(Debug, Deserialize, Default)]
struct ChunkQuery {
    from: Option<i32>,
    limit: Option<i32>,
}

#[instrument(skip(app_state, context, query))]
async fn thread_tree(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(root_id): Path<Uuid>,
    Query(query): Query<ThreadTreeQuery>,
) -> AppResult<Json<ThreadTreeResponse>> {
    let user_id = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool.clone());

    let response = service
        .get_thread_subtree(user_id, root_id, query.cursor_path, query.limit)
        .await?;

    Ok(Json(response))
}

#[instrument(skip(app_state, context, payload))]
async fn post_root(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Extension(hub): Extension<SharedStreamHub>,
    Path(conversation_id): Path<Uuid>,
    Json(payload): Json<PostRootMessageRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool.clone());
    let assistant = require_assistant(&app_state)?;

    let PostRootMessageRequest { content, role } = payload;
    let request = PostRootMessageRequest {
        content: content.clone(),
        role,
    };

    let response = service
        .post_root_message(user_id, conversation_id, request)
        .await?;

    let summary = service
        .get_thread_summary(user_id, response.root_id)
        .await?;

    if summary.conversation_id == conversation_id {
        let thread_new = ConversationStreamEvent::ThreadNew {
            payload: ThreadNewEvent {
                conversation_id,
                root_id: response.root_id,
                summary: summary.summary.clone(),
            },
        };
        hub.publish(conversation_id, thread_new).await;

        let activity = ConversationStreamEvent::ThreadActivity {
            payload: ThreadActivityEvent {
                root_id: response.root_id,
                last_activity_at: summary.summary.last_activity_at.clone(),
            },
        };
        hub.publish(conversation_id, activity).await;
    }

    if should_spawn_assistant(role) {
        spawn_assistant_reply(
            pool,
            hub.clone(),
            assistant.clone(),
            user_id,
            response.message_id,
            content,
        );
    }

    Ok((StatusCode::CREATED, Json(response)))
}

#[instrument(skip(app_state, context, payload))]
async fn reply_message(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Extension(hub): Extension<SharedStreamHub>,
    Path(parent_id): Path<Uuid>,
    Json(payload): Json<ReplyMessageRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool.clone());
    let assistant = require_assistant(&app_state)?;

    let ReplyMessageRequest { content, role } = payload;
    let request = ReplyMessageRequest {
        content: content.clone(),
        role,
    };

    let response = service.reply_message(user_id, parent_id, request).await?;

    let summary = service
        .get_thread_summary(user_id, response.root_id)
        .await
        .ok();

    if let Some(summary_ref) = summary.as_ref() {
        let activity = ConversationStreamEvent::ThreadActivity {
            payload: ThreadActivityEvent {
                root_id: response.root_id,
                last_activity_at: summary_ref.summary.last_activity_at.clone(),
            },
        };

        hub.publish(summary_ref.conversation_id, activity).await;
    }

    if should_spawn_assistant(role) {
        spawn_assistant_reply(
            pool,
            hub.clone(),
            assistant.clone(),
            user_id,
            response.message_id,
            content,
        );
    }

    Ok((StatusCode::CREATED, Json(response)))
}

#[instrument(skip(app_state, context, query))]
async fn message_chunks(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(message_id): Path<Uuid>,
    Query(query): Query<ChunkQuery>,
) -> AppResult<Json<Vec<MessageChunk>>> {
    let user_id = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    let chunks = service
        .list_chunks(user_id, message_id, query.from, query.limit)
        .await?;

    Ok(Json(chunks))
}

#[instrument(skip(app_state, context, hub, payload))]
async fn mark_thread_read(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Extension(hub): Extension<SharedStreamHub>,
    Path(root_id): Path<Uuid>,
    Json(payload): Json<MarkThreadReadRequest>,
) -> AppResult<impl IntoResponse> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool.clone());

    let root_message = service.get_message(actor, root_id).await?;
    service
        .mark_thread_read(
            actor,
            root_message.conversation_id,
            root_id,
            payload.path.as_deref(),
        )
        .await?;

    let summaries = service
        .unread_summary(actor, root_message.conversation_id)
        .await?;
    let unread = summaries
        .into_iter()
        .find(|item| item.root_id == root_id)
        .map(|item| item.unread)
        .unwrap_or(0);

    let event = ConversationStreamEvent::UnreadUpdate {
        payload: UnreadUpdateEvent { root_id, unread },
    };
    hub.publish(root_message.conversation_id, event).await;

    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(app_state, context, payload))]
async fn delete_message(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(message_id): Path<Uuid>,
    Json(payload): Json<MessageDeleteRequest>,
) -> AppResult<impl IntoResponse> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    service
        .soft_delete_message(actor, message_id, payload.reason.clone())
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(app_state, context))]
async fn restore_message(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(message_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    service.restore_message(actor, message_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(app_state, context, payload))]
async fn edit_message(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(message_id): Path<Uuid>,
    Json(payload): Json<MessageEditRequest>,
) -> AppResult<impl IntoResponse> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    service
        .edit_message(
            actor,
            message_id,
            payload.content.clone(),
            payload.reason.clone(),
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(app_state, context, hub, payload))]
async fn set_typing(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Extension(hub): Extension<SharedStreamHub>,
    Json(payload): Json<TypingRequest>,
) -> AppResult<impl IntoResponse> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    service
        .set_typing(
            actor,
            payload.conversation_id,
            payload.root_id,
            payload.seconds,
        )
        .await?;

    if payload.seconds > 0 {
        let expires_at = Timestamp(Utc::now() + Duration::seconds(payload.seconds as i64));
        let event = ConversationStreamEvent::TypingUpdate {
            payload: shared::models::TypingUpdate {
                conversation_id: payload.conversation_id,
                root_id: payload.root_id,
                user_id: actor,
                expires_at,
            },
        };
        hub.publish(payload.conversation_id, event).await;
    }

    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(app_state, context, hub, payload))]
async fn presence_heartbeat(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Extension(hub): Extension<SharedStreamHub>,
    Json(payload): Json<PresenceHeartbeatRequest>,
) -> AppResult<impl IntoResponse> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    let status = payload
        .status
        .unwrap_or(shared::models::PresenceStatus::Online);
    service.heartbeat(actor, Some(status)).await?;

    let last_seen = Timestamp(Utc::now());
    let conversations = service.active_conversations(actor).await?;
    let event = ConversationStreamEvent::PresenceUpdate {
        payload: shared::models::PresenceUpdate {
            user_id: actor,
            status,
            last_seen_at: last_seen.clone(),
        },
    };

    for conversation_id in conversations {
        hub.publish(conversation_id, event.clone()).await;
    }

    Ok(StatusCode::NO_CONTENT)
}

fn should_spawn_assistant(role: Option<MessageRole>) -> bool {
    matches!(role, None | Some(MessageRole::User))
}

fn spawn_assistant_reply(
    pool: PgPool,
    hub: SharedStreamHub,
    assistant: Arc<AssistantService>,
    actor: Uuid,
    parent_message_id: Uuid,
    user_message: String,
) {
    tokio::spawn(async move {
        if let Err(err) =
            run_assistant_reply(pool, hub, assistant, actor, parent_message_id, user_message).await
        {
            warn!(error = %err, "assistant reply generation failed");
        }
    });
}

async fn run_assistant_reply(
    pool: PgPool,
    hub: SharedStreamHub,
    assistant: Arc<AssistantService>,
    actor: Uuid,
    parent_message_id: Uuid,
    user_message: String,
) -> Result<(), ChatServiceError> {
    let service = ChatService::new(pool);

    let default_config = assistant
        .default_chat_config()
        .map_err(|err| ChatServiceError::Validation(err.to_string()))?;

    let parent_message = service.get_message(actor, parent_message_id).await?;
    let context_chain = service
        .get_ancestor_chain(actor, parent_message.root_id, &parent_message.path)
        .await?;
    let builder = ThreadContextBuilder::new(context_chain.clone());
    let prompt_sequence = builder.ancestor_chain(parent_message_id);

    let request = build_stream_request(
        &prompt_sequence,
        &default_config,
        assistant.default_model_name(),
        &user_message,
    );

    let session = assistant
        .stream_reply(request)
        .await
        .map_err(|err| ChatServiceError::Validation(err.to_string()))?;

    let mut stream = session.stream;
    let mut accumulated = String::new();
    let mut reply_response: Option<ReplyMessageResponse> = None;
    let mut summary = None;
    let mut resolved_conversation = None;
    let mut finish_reason: Option<String> = None;
    let mut usage: Option<TokenUsage> = None;
    let mut chunk_index: i32 = 0;
    let mut stream_error: Option<String> = None;
    let persist_chunks = assistant.persist_stream_chunks();

    while let Some(next) = stream.next().await {
        match next {
            Ok(chunk) => {
                if let Some(reason) = &chunk.finish_reason {
                    finish_reason = Some(finish_reason_to_string(reason));
                }

                usage = Some(chunk.usage.clone());

                if !chunk.text_delta.is_empty() {
                    accumulated.push_str(&chunk.text_delta);
                }

                if reply_response.is_none() {
                    if accumulated.is_empty() {
                        continue;
                    }

                    let created = service
                        .reply_as_assistant(actor, parent_message_id, accumulated.clone())
                        .await?;

                    resolved_conversation = Some(created.conversation_id);
                    summary = service
                        .get_thread_summary(actor, created.root_id)
                        .await
                        .ok();

                    reply_response = Some(created);
                } else if let Some(created) = reply_response.as_ref() {
                    if let Err(err) = service
                        .update_message_content(actor, created.message_id, accumulated.clone())
                        .await
                    {
                        warn!(error = %err, "failed to update assistant message content");
                    }
                }

                let Some(created) = reply_response.as_ref() else {
                    continue;
                };

                if persist_chunks && !chunk.text_delta.is_empty() {
                    if let Err(err) = service
                        .append_chunk(
                            actor,
                            created.message_id,
                            chunk_index,
                            chunk.text_delta.clone(),
                        )
                        .await
                    {
                        warn!(error = %err, "failed to store assistant chunk");
                    }
                }

                if !chunk.text_delta.is_empty() {
                    let delta = ConversationStreamEvent::MessageDelta {
                        payload: ChatDeltaChunk {
                            id: format!("{}:{}", created.message_id, chunk_index),
                            object: "chat.completion.chunk".to_string(),
                            root_id: created.root_id,
                            message_id: created.message_id,
                            conversation_id: created.conversation_id,
                            parent_id: created.parent_id,
                            depth: Some(created.depth),
                            choices: vec![ChatDeltaChoice {
                                index: 0,
                                delta: ChatDelta {
                                    role: if chunk_index == 0 {
                                        Some(MessageRole::Assistant)
                                    } else {
                                        None
                                    },
                                    content: Some(chunk.text_delta.clone()),
                                },
                                finish_reason: None,
                            }],
                        },
                    };

                    let conversation = resolved_conversation.unwrap_or(created.conversation_id);
                    hub.publish(conversation, delta).await;
                    chunk_index = chunk_index.saturating_add(1);
                }

                if chunk.is_final {
                    break;
                }
            }
            Err(err) => {
                stream_error = Some(err.to_string());
                break;
            }
        }
    }

    let reply_response = if let Some(reply) = reply_response {
        reply
    } else {
        let fallback = "I'm sorry, I couldn't generate a response right now.".to_string();
        let created = service
            .reply_as_assistant(actor, parent_message_id, fallback.clone())
            .await?;
        resolved_conversation = Some(created.conversation_id);
        accumulated = fallback;
        summary = service
            .get_thread_summary(actor, created.root_id)
            .await
            .ok();
        created
    };

    if let Some(error) = stream_error.as_ref() {
        if !accumulated.is_empty() {
            accumulated.push_str("\n\n");
        }
        let warning = format!("⚠️ Assistant stream interrupted: {error}");
        accumulated.push_str(&warning);
        finish_reason = Some("error".to_string());
        let conversation = resolved_conversation.unwrap_or(reply_response.conversation_id);
        hub.publish(
            conversation,
            ConversationStreamEvent::Error {
                payload: StreamErrorEvent {
                    code: "assistant_stream_error".to_string(),
                    message: warning,
                },
            },
        )
        .await;
    }

    let conversation = resolved_conversation.unwrap_or(reply_response.conversation_id);

    if let Err(err) = service
        .update_message_content(actor, reply_response.message_id, accumulated.clone())
        .await
    {
        warn!(error = %err, "failed to persist final assistant message content");
    }

    let usage_breakdown = usage
        .map(|usage| token_usage_to_breakdown(&usage, session.prompt_tokens, &accumulated))
        .unwrap_or_else(|| infer_usage_from_text(session.prompt_tokens, &accumulated));

    let finish_reason_value = stream_error
        .map(|_| "error".to_string())
        .or(finish_reason.clone())
        .unwrap_or_else(|| "stop".to_string());

    let done = ConversationStreamEvent::MessageDone {
        payload: MessageDoneEvent {
            message_id: reply_response.message_id,
            root_id: reply_response.root_id,
            conversation_id: reply_response.conversation_id,
            finish_reason: Some(finish_reason_value),
            usage: Some(usage_breakdown),
        },
    };
    hub.publish(conversation, done).await;

    if summary.is_none() {
        summary = service
            .get_thread_summary(actor, reply_response.root_id)
            .await
            .ok();
    }

    if let Some(summary) = summary {
        let activity = ConversationStreamEvent::ThreadActivity {
            payload: ThreadActivityEvent {
                root_id: reply_response.root_id,
                last_activity_at: summary.summary.last_activity_at.clone(),
            },
        };
        hub.publish(summary.conversation_id, activity).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_guard_allows_only_user_role() {
        assert!(should_spawn_assistant(None));
        assert!(should_spawn_assistant(Some(MessageRole::User)));
        assert!(!should_spawn_assistant(Some(MessageRole::Assistant)));
        assert!(!should_spawn_assistant(Some(MessageRole::System)));
        assert!(!should_spawn_assistant(Some(MessageRole::Tool)));
    }
}

fn require_user(context: &RequestContext) -> AppResult<Uuid> {
    context
        .user_id
        .ok_or_else(|| ApiError::forbidden("authentication required"))
}

fn require_pool(state: &AppState) -> AppResult<PgPool> {
    state.pool.clone().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
            "database pool not configured",
        )
    })
}

fn require_assistant(state: &AppState) -> AppResult<Arc<AssistantService>> {
    state.assistant.clone().ok_or_else(|| {
        ApiError::internal_server_error("assistant streaming service not configured")
    })
}

fn token_usage_to_breakdown(
    usage: &TokenUsage,
    prompt_fallback: i64,
    content: &str,
) -> UsageBreakdown {
    let prompt_tokens = if usage.prompt_tokens > 0 {
        usage.prompt_tokens as i64
    } else {
        prompt_fallback
    };

    let completion_tokens = if usage.completion_tokens > 0 {
        usage.completion_tokens as i64
    } else {
        approximate_text_tokens(content)
    };

    let total_tokens = if usage.total_tokens > 0 {
        usage.total_tokens as i64
    } else {
        prompt_tokens + completion_tokens
    };

    UsageBreakdown {
        prompt_tokens,
        completion_tokens,
        total_tokens,
    }
}

fn infer_usage_from_text(prompt_tokens: i64, content: &str) -> UsageBreakdown {
    let completion_tokens = approximate_text_tokens(content);
    UsageBreakdown {
        prompt_tokens,
        completion_tokens,
        total_tokens: prompt_tokens + completion_tokens,
    }
}

fn approximate_text_tokens(text: &str) -> i64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        0
    } else {
        trimmed.split_whitespace().count() as i64
    }
}

fn build_stream_request(
    messages: &[MessageView],
    default_config: &LLMConfig,
    model_name: &str,
    fallback_user_message: &str,
) -> LLMRequest {
    let mut system_segments = Vec::new();
    let mut lines = Vec::new();

    for message in messages {
        let content = message.content.trim();
        if content.is_empty() {
            continue;
        }
        match message.role {
            MessageRole::System => system_segments.push(content.to_string()),
            MessageRole::User => lines.push(format!("User: {content}")),
            MessageRole::Assistant => lines.push(format!("Assistant: {content}")),
            MessageRole::Tool => lines.push(format!("Tool: {content}")),
        }
    }

    if lines.is_empty() {
        let fallback = fallback_user_message.trim();
        if fallback.is_empty() {
            lines.push("Assistant:".to_string());
        } else {
            lines.push(format!("User: {fallback}"));
            lines.push("Assistant:".to_string());
        }
    } else if !lines
        .last()
        .map(|line| line.starts_with("Assistant:"))
        .unwrap_or(false)
    {
        lines.push("Assistant:".to_string());
    }

    let mut request = LLMRequest::new_streaming(lines.join("\n"));

    if !system_segments.is_empty() {
        request = request.with_system_message(system_segments.join("\n"));
    }

    if let Some(max_tokens) = default_config.max_tokens {
        request = request.with_max_tokens(max_tokens);
    }
    if let Some(temperature) = default_config.temperature {
        request = request.with_temperature(temperature);
    }
    if let Some(top_p) = default_config.top_p {
        request = request.with_metadata("top_p", json!(top_p));
    }
    if let Some(top_k) = default_config.top_k {
        request = request.with_metadata("top_k", json!(top_k));
    }
    if let Some(repetition_penalty) = default_config.repetition_penalty {
        request = request.with_metadata("repetition_penalty", json!(repetition_penalty));
    }
    if let Some(stop_sequences) = default_config
        .additional_params
        .get("stop_sequences")
        .and_then(|value| value.as_array())
    {
        for sequence in stop_sequences.iter().filter_map(|value| value.as_str()) {
            request = request.with_stop_sequence(sequence.to_string());
        }
    }

    request = request.with_metadata("model", json!(model_name));
    request
}
