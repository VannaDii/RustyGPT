use std::{
    convert::Infallible,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    Json,
    extract::Extension,
    http::{HeaderMap, StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auth::session::{SessionUser, SessionValidation},
    handlers::{
        auth::{extract_session_cookie, map_session_error, metadata_from_headers},
        streaming::SharedStreamHub,
        threads::{
            ensure_reply_response, infer_usage_from_text, persist_chunk_if_needed,
            publish_delta_event, token_usage_to_breakdown,
        },
    },
    http::error::{ApiError, AppResult},
    middleware::request_context::RequestContext,
    services::{
        assistant_service::{AssistantError, AssistantStreamingSession, finish_reason_to_string},
        chat_service::{ChatService, ChatServiceError, ThreadSummaryWithConversation},
        stream_supervisor::{SharedStreamSupervisor, StreamSession, StreamStopReason},
    },
};
use chrono::Utc;
use shared::{
    config::server::Config,
    llms::ThreadContextBuilder,
    llms::types::{LLMRequest, StreamingResponse, TokenUsage},
    models::{
        ChatCompletionChoice, ChatCompletionChunk, ChatCompletionChunkChoice,
        ChatCompletionChunkDelta, ChatCompletionMessage, ChatCompletionRequest,
        ChatCompletionResponse, ConversationStreamEvent, MessageDoneEvent, MessageRole,
        MessageView, Model, ModelsResponse, ReplyMessageRequest, ReplyMessageResponse,
        StreamErrorEvent, ThreadActivityEvent, UsageBreakdown,
    },
};

const OBJECT_COMPLETION: &str = "chat.completion";
const OBJECT_CHUNK: &str = "chat.completion.chunk";

#[derive(Debug, Default)]
struct CompletionOverrides {
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
    stop_sequences: Vec<String>,
}

impl CompletionOverrides {
    const fn from_request(request: &ChatCompletionRequest, stop_sequences: Vec<String>) -> Self {
        Self {
            temperature: request.temperature,
            top_p: request.top_p,
            max_tokens: request.max_tokens,
            stop_sequences,
        }
    }
}

#[derive(Debug, Clone)]
struct RustyMetadata {
    conversation_id: Uuid,
    parent_message_id: Uuid,
}

struct StatefulContext {
    actor: SessionUser,
    service: ChatService,
    hub: SharedStreamHub,
    streams: Option<SharedStreamSupervisor>,
    parent_message_id: Uuid,
    prompt_sequence: Vec<MessageView>,
    fallback_user_message: String,
}

struct StatefulStreamController {
    service: ChatService,
    hub: SharedStreamHub,
    supervisor: Option<SharedStreamSupervisor>,
    stream_session: Option<Arc<StreamSession>>,
    actor_id: Uuid,
    parent_message_id: Uuid,
    persist_chunks: bool,
    reply_response: Option<ReplyMessageResponse>,
    resolved_conversation: Option<Uuid>,
    summary: Option<ThreadSummaryWithConversation>,
    chunk_index: i32,
    accumulated: String,
    registered: bool,
}

impl StatefulStreamController {
    #[allow(clippy::missing_const_for_fn)]
    fn new(
        service: ChatService,
        hub: SharedStreamHub,
        supervisor: Option<SharedStreamSupervisor>,
        stream_session: Option<Arc<StreamSession>>,
        actor_id: Uuid,
        parent_message_id: Uuid,
        persist_chunks: bool,
    ) -> Self {
        Self {
            service,
            hub,
            supervisor,
            stream_session,
            actor_id,
            parent_message_id,
            persist_chunks,
            reply_response: None,
            resolved_conversation: None,
            summary: None,
            chunk_index: 0,
            accumulated: String::new(),
            registered: false,
        }
    }

    async fn process_chunk(&mut self, chunk: &StreamingResponse) -> Result<(), ChatServiceError> {
        if !chunk.text_delta.is_empty() {
            self.accumulated.push_str(&chunk.text_delta);
        }

        if ensure_reply_response(
            &self.service,
            self.actor_id,
            self.parent_message_id,
            &self.accumulated,
            &mut self.reply_response,
            &mut self.resolved_conversation,
            &mut self.summary,
        )
        .await?
        {
            return Ok(());
        }

        if !self.registered && self.reply_response.is_some() {
            if let Some(message_id) = self
                .reply_response
                .as_ref()
                .map(|created| created.message_id)
            {
                if let (Some(supervisor), Some(session_handle)) = (
                    self.supervisor.as_ref(),
                    self.stream_session.as_ref().map(Arc::clone),
                ) {
                    supervisor.register(message_id, session_handle).await;
                }
                self.registered = true;
            }
        }

        let Some(created) = self.reply_response.as_ref() else {
            return Ok(());
        };

        persist_chunk_if_needed(
            &self.service,
            self.actor_id,
            created,
            self.chunk_index,
            self.persist_chunks,
            &chunk.text_delta,
        )
        .await;

        if !chunk.text_delta.is_empty() {
            let conversation = self
                .resolved_conversation
                .unwrap_or(created.conversation_id);
            publish_delta_event(
                &self.hub,
                conversation,
                created,
                &chunk.text_delta,
                self.chunk_index,
            )
            .await;
            self.chunk_index = self.chunk_index.saturating_add(1);
        }

        Ok(())
    }

    #[allow(clippy::cognitive_complexity)]
    async fn finalize(
        mut self,
        prompt_tokens: i64,
        finish_reason: Option<String>,
        usage: Option<TokenUsage>,
        mut stream_error: Option<String>,
        stop_reason: Option<StreamStopReason>,
    ) -> Result<StatefulFinalization, ChatServiceError> {
        let reply_response = if let Some(reply) = self.reply_response.clone() {
            reply
        } else {
            let fallback = match stop_reason {
                Some(StreamStopReason::Cancelled) => "Assistant response cancelled.".to_string(),
                Some(StreamStopReason::TimedOut) => {
                    "Assistant response timed out before completion.".to_string()
                }
                _ => "I'm sorry, I couldn't generate a response right now.".to_string(),
            };
            let created = self
                .service
                .reply_as_assistant(self.actor_id, self.parent_message_id, fallback.clone())
                .await?;
            self.accumulated = fallback;
            self.summary = self
                .service
                .get_thread_summary(self.actor_id, created.root_id)
                .await
                .ok();
            created
        };

        let mut warning_message: Option<String> = None;

        if stop_reason == Some(StreamStopReason::TimedOut) {
            warning_message = Some("assistant generation timed out before completion.".to_string());
            stream_error = None;
        }

        if warning_message.is_none() {
            if let Some(error) = stream_error.clone() {
                warning_message = Some(format!("assistant stream error: {error}"));
            }
        }

        if let Some(message) = warning_message.as_ref() {
            if !self.accumulated.is_empty() {
                self.accumulated.push_str("\n\n");
            }
            self.accumulated.push_str(&format!("⚠️ {}", message));
        }

        if let Err(err) = self
            .service
            .update_message_content(
                self.actor_id,
                reply_response.message_id,
                self.accumulated.clone(),
            )
            .await
        {
            warn!(error = %err, "failed to persist assistant final content");
        }

        let usage_breakdown = usage
            .as_ref()
            .map(|usage| token_usage_to_breakdown(usage, prompt_tokens, &self.accumulated))
            .unwrap_or_else(|| infer_usage_from_text(prompt_tokens, &self.accumulated));

        let default_finish = finish_reason.unwrap_or_else(|| "stop".to_string());

        let finish_reason_value = match stop_reason {
            Some(StreamStopReason::Cancelled) => "cancelled".to_string(),
            Some(StreamStopReason::TimedOut) => "timeout".to_string(),
            _ => {
                if warning_message.is_some() {
                    "error".to_string()
                } else {
                    default_finish
                }
            }
        };

        let conversation = self
            .resolved_conversation
            .unwrap_or(reply_response.conversation_id);

        self.hub
            .publish(
                conversation,
                ConversationStreamEvent::MessageDone {
                    payload: MessageDoneEvent {
                        message_id: reply_response.message_id,
                        root_id: reply_response.root_id,
                        conversation_id: reply_response.conversation_id,
                        finish_reason: Some(finish_reason_value.clone()),
                        usage: Some(usage_breakdown.clone()),
                    },
                },
            )
            .await;

        if let Some(message) = warning_message.as_ref() {
            let code = if matches!(stop_reason, Some(StreamStopReason::TimedOut)) {
                "assistant_timeout"
            } else {
                "assistant_stream_error"
            };
            self.hub
                .publish(
                    conversation,
                    ConversationStreamEvent::Error {
                        payload: StreamErrorEvent {
                            code: code.to_string(),
                            message: message.clone(),
                        },
                    },
                )
                .await;
        }

        if self.summary.is_none() {
            self.summary = self
                .service
                .get_thread_summary(self.actor_id, reply_response.root_id)
                .await
                .ok();
        }

        if let Some(summary) = self.summary.as_ref() {
            self.hub
                .publish(
                    summary.conversation_id,
                    ConversationStreamEvent::ThreadActivity {
                        payload: ThreadActivityEvent {
                            root_id: reply_response.root_id,
                            last_activity_at: summary.summary.last_activity_at.clone(),
                        },
                    },
                )
                .await;
        }

        if let Some(supervisor) = self.supervisor.as_ref() {
            supervisor.unregister(&reply_response.message_id).await;
        }

        Ok(StatefulFinalization {
            accumulated: self.accumulated,
            usage: usage_breakdown,
            finish_reason: finish_reason_value,
            warning: warning_message,
        })
    }
}

struct StatefulFinalization {
    accumulated: String,
    usage: UsageBreakdown,
    finish_reason: String,
    warning: Option<String>,
}

fn gather_warnings(request: &ChatCompletionRequest) -> Vec<String> {
    let mut warnings = Vec::new();
    if request.presence_penalty.is_some() {
        warnings.push("presence_penalty is not supported and was ignored".to_string());
    }
    if request.frequency_penalty.is_some() {
        warnings.push("frequency_penalty is not supported and was ignored".to_string());
    }
    if request
        .user
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        warnings.push("user parameter was provided but empty; ignoring".to_string());
    }
    warnings
}

fn parse_stop_sequences(value: &Option<Value>) -> AppResult<Vec<String>> {
    match value {
        None => Ok(Vec::new()),
        Some(Value::String(single)) => Ok(vec![single.clone()]),
        Some(Value::Array(items)) => {
            let mut stops = Vec::with_capacity(items.len());
            for item in items {
                let Some(text) = item.as_str() else {
                    return Err(ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "RGP.V1.INVALID_STOP",
                        "stop must be a string or array of strings",
                    ));
                };
                stops.push(text.to_string());
            }
            Ok(stops)
        }
        Some(_) => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RGP.V1.INVALID_STOP",
            "stop must be a string or array of strings",
        )),
    }
}

fn parse_rustygpt_metadata(value: &Option<Value>) -> AppResult<Option<RustyMetadata>> {
    let Some(Value::Object(root)) = value else {
        return Ok(None);
    };
    let Some(rg_value) = root.get("rustygpt") else {
        return Ok(None);
    };
    let Some(rg) = rg_value.as_object() else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "RGP.V1.INVALID_METADATA",
            "metadata.rustygpt must be an object",
        ));
    };

    let conversation_id = rg
        .get("conversation_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "RGP.V1.INVALID_METADATA",
                "metadata.rustygpt.conversation_id is required",
            )
        })
        .and_then(|value| {
            Uuid::parse_str(value).map_err(|_| {
                ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "RGP.V1.INVALID_METADATA",
                    "conversation_id must be a UUID string",
                )
            })
        })?;

    let parent_message_id = rg
        .get("parent_message_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "RGP.V1.INVALID_METADATA",
                "metadata.rustygpt.parent_message_id is required",
            )
        })
        .and_then(|value| {
            Uuid::parse_str(value).map_err(|_| {
                ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "RGP.V1.INVALID_METADATA",
                    "parent_message_id must be a UUID string",
                )
            })
        })?;

    Ok(Some(RustyMetadata {
        conversation_id,
        parent_message_id,
    }))
}

fn extract_latest_user_message(
    messages: &[ChatCompletionMessage],
) -> Option<&ChatCompletionMessage> {
    messages.iter().rev().find(|message| {
        matches!(message.role.to_lowercase().as_str(), "user" | "human")
            && !message.content.trim().is_empty()
    })
}

async fn authenticate_request(
    state: &Arc<AppState>,
    config: &Config,
    headers: &HeaderMap,
) -> AppResult<Option<SessionValidation>> {
    let Some(manager) = state.sessions.clone() else {
        return Ok(None);
    };

    let Some(token) = extract_session_cookie(headers, &config.session.session_cookie_name) else {
        return Ok(None);
    };

    let metadata = metadata_from_headers(headers);
    match manager.validate_session(&token, &metadata).await {
        Ok(Some(validation)) => Ok(Some(validation)),
        Ok(None) => Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "RGP.AUTH.INVALID_SESSION",
            "session expired",
        )),
        Err(err) => Err(map_session_error(err)),
    }
}

async fn prepare_stateful_context(
    state: &Arc<AppState>,
    hub: &SharedStreamHub,
    validation: &SessionValidation,
    metadata: RustyMetadata,
    payload: &ChatCompletionRequest,
) -> AppResult<StatefulContext> {
    let pool = state.pool.clone().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "RGP.DB.UNAVAILABLE",
            "database pool not configured",
        )
    })?;

    let service = ChatService::new(pool);
    let actor = validation.user.clone();

    service
        .ensure_membership(actor.id, metadata.conversation_id)
        .await?;

    let parent_message = service
        .get_message(actor.id, metadata.parent_message_id)
        .await?;

    if parent_message.conversation_id != metadata.conversation_id {
        return Err(ApiError::forbidden(
            "parent message does not belong to target conversation",
        ));
    }

    let user_message = extract_latest_user_message(&payload.messages).ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "RGP.V1.INVALID_MESSAGES",
            "at least one user message is required",
        )
    })?;

    service
        .reply_message(
            actor.id,
            metadata.parent_message_id,
            ReplyMessageRequest {
                content: user_message.content.clone(),
                role: Some(MessageRole::User),
            },
        )
        .await?;

    let context_chain = service
        .get_ancestor_chain(actor.id, parent_message.root_id, &parent_message.path)
        .await?;

    let builder = ThreadContextBuilder::new(context_chain);
    let prompt_sequence = builder.ancestor_chain(metadata.parent_message_id);

    Ok(StatefulContext {
        actor,
        service,
        hub: hub.clone(),
        streams: state.streams.clone(),
        parent_message_id: metadata.parent_message_id,
        prompt_sequence,
        fallback_user_message: user_message.content.clone(),
    })
}

#[instrument(skip(config))]
pub async fn get_models(Extension(config): Extension<Arc<Config>>) -> Json<ModelsResponse> {
    let mut models: Vec<Model> = config
        .llm
        .models
        .iter()
        .map(|(id, model)| Model {
            id: id.clone(),
            object: OBJECT_MODEL.to_string(),
            created: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            owned_by: model.provider.clone(),
            name: Some(model.display_name.clone()),
            model_type: Some(if model.capabilities.chat_format {
                "chat".to_string()
            } else {
                "text".to_string()
            }),
        })
        .collect();

    models.sort_by(|a, b| a.id.cmp(&b.id));

    Json(ModelsResponse { models })
}

fn build_stateless_request(
    messages: &[ChatCompletionMessage],
    default_config: &shared::llms::types::LLMConfig,
    model_name: &str,
    overrides: &CompletionOverrides,
    stream: bool,
) -> LLMRequest {
    let (prompt, system_message) = build_prompt_from_messages(messages);
    finalize_llm_request(
        prompt,
        system_message,
        default_config,
        overrides,
        model_name,
        stream,
    )
}

fn build_stateful_request(
    context: &StatefulContext,
    default_config: &shared::llms::types::LLMConfig,
    model_name: &str,
    overrides: &CompletionOverrides,
    stream: bool,
) -> LLMRequest {
    let (prompt, system_message) =
        build_prompt_from_thread(&context.prompt_sequence, &context.fallback_user_message);
    finalize_llm_request(
        prompt,
        system_message,
        default_config,
        overrides,
        model_name,
        stream,
    )
}

fn finalize_llm_request(
    prompt: String,
    system_message: Option<String>,
    default_config: &shared::llms::types::LLMConfig,
    overrides: &CompletionOverrides,
    model_name: &str,
    stream: bool,
) -> LLMRequest {
    let mut request = if stream {
        LLMRequest::new_streaming(prompt)
    } else {
        LLMRequest::new(prompt)
    };

    if let Some(system) = system_message {
        request = request.with_system_message(system);
    }

    if let Some(max_tokens) = overrides
        .max_tokens
        .or_else(|| default_config.max_tokens.map(|value| value as u32))
    {
        request = request.with_max_tokens(max_tokens);
    }

    if let Some(temperature) = overrides
        .temperature
        .or_else(|| default_config.temperature.map(|value| value as f32))
    {
        request = request.with_temperature(temperature);
    }

    if let Some(top_p) = overrides
        .top_p
        .or_else(|| default_config.top_p.map(|value| value as f32))
    {
        request = request.with_metadata("top_p", json!(top_p));
    }

    if let Some(top_k) = default_config.top_k {
        request = request.with_metadata("top_k", json!(top_k));
    }

    if let Some(repetition_penalty) = default_config.repetition_penalty {
        request = request.with_metadata("repetition_penalty", json!(repetition_penalty));
    }

    for stop in &overrides.stop_sequences {
        request = request.with_stop_sequence(stop.clone());
    }

    if overrides.stop_sequences.is_empty() {
        if let Some(stops) = default_config
            .additional_params
            .get("stop_sequences")
            .and_then(|value| value.as_array())
        {
            for stop in stops.iter().filter_map(|value| value.as_str()) {
                request = request.with_stop_sequence(stop.to_string());
            }
        }
    }

    request = request.with_metadata("model", json!(model_name));
    request
}

fn build_prompt_from_messages(messages: &[ChatCompletionMessage]) -> (String, Option<String>) {
    let mut system_segments = Vec::new();
    let mut lines = Vec::new();

    for message in messages {
        let role = message.role.to_lowercase();
        let content = message.content.trim();
        if content.is_empty() {
            continue;
        }

        match role.as_str() {
            "system" => system_segments.push(content.to_string()),
            "assistant" => lines.push(format!("Assistant: {content}")),
            "tool" => lines.push(format!("Tool: {content}")),
            _ => lines.push(format!("User: {content}")),
        }
    }

    if lines.is_empty() {
        lines.push("User:".to_string());
    }

    if !lines
        .last()
        .map(|line| line.starts_with("Assistant:"))
        .unwrap_or(false)
    {
        lines.push("Assistant:".to_string());
    }

    let system_message = if system_segments.is_empty() {
        None
    } else {
        Some(system_segments.join("\n"))
    };

    (lines.join("\n"), system_message)
}

fn build_prompt_from_thread(
    messages: &[MessageView],
    fallback_user_message: &str,
) -> (String, Option<String>) {
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
        if !fallback.is_empty() {
            lines.push(format!("User: {fallback}"));
        }
    }

    if !lines
        .last()
        .map(|line| line.starts_with("Assistant:"))
        .unwrap_or(false)
    {
        lines.push("Assistant:".to_string());
    }

    let system_message = if system_segments.is_empty() {
        None
    } else {
        Some(system_segments.join("\n"))
    };

    (lines.join("\n"), system_message)
}

fn map_assistant_error(error: AssistantError) -> ApiError {
    match error {
        AssistantError::Config(message) => {
            ApiError::new(StatusCode::BAD_REQUEST, "RGP.LLM.CONFIG", message)
        }
        AssistantError::Provider(message) => {
            ApiError::internal_server_error(format!("llm provider error: {message}"))
        }
        AssistantError::Inference(message) => {
            ApiError::internal_server_error(format!("llm inference error: {message}"))
        }
    }
}

fn apply_session_rotation(response: &mut Response, validation: Option<&SessionValidation>) {
    let Some(validation) = validation else {
        return;
    };

    if let Some(bundle) = validation.bundle.as_ref() {
        if let Ok(value) = header_value(&bundle.session_cookie.to_string()) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
        if let Ok(value) = header_value(&bundle.csrf_cookie.to_string()) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
        response.headers_mut().insert(
            header::HeaderName::from_static("x-session-rotated"),
            header::HeaderValue::from_static("1"),
        );
    } else if validation.rotated {
        response.headers_mut().insert(
            header::HeaderName::from_static("x-session-rotated"),
            header::HeaderValue::from_static("1"),
        );
    }
}

fn header_value(value: &str) -> Result<header::HeaderValue, ApiError> {
    header::HeaderValue::from_str(value)
        .map_err(|_| ApiError::internal_server_error("failed to encode cookie header".to_string()))
}

async fn complete_non_streaming(
    session: AssistantStreamingSession,
    completion_id: String,
    created: i64,
    model_name: String,
    warnings: Vec<String>,
    stateful: Option<StatefulContext>,
    persist_chunks: bool,
) -> AppResult<Response> {
    if let Some(context) = stateful {
        complete_stateful_non_streaming(
            session,
            completion_id,
            created,
            model_name,
            warnings,
            context,
            persist_chunks,
        )
        .await
    } else {
        complete_stateless_non_streaming(session, completion_id, created, model_name, warnings)
            .await
    }
}

async fn stream_completion(
    session: AssistantStreamingSession,
    completion_id: String,
    created: i64,
    model_name: String,
    warnings: Vec<String>,
    stateful: Option<StatefulContext>,
    persist_chunks: bool,
) -> AppResult<Response> {
    let (tx, rx) = mpsc::channel::<Event>(32);

    tokio::spawn(async move {
        if let Err(err) = run_streaming_session(
            session,
            completion_id,
            created,
            model_name,
            warnings,
            stateful,
            persist_chunks,
            tx,
        )
        .await
        {
            warn!(error = %err, "streaming session terminated with error");
        }
    });

    let stream = ReceiverStream::new(rx).map(|event| Ok::<Event, Infallible>(event));
    let response = Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("ping"),
        )
        .into_response();
    Ok(response)
}

#[allow(clippy::cognitive_complexity)]
async fn run_streaming_session(
    session: AssistantStreamingSession,
    completion_id: String,
    created: i64,
    model_name: String,
    mut warnings: Vec<String>,
    stateful: Option<StatefulContext>,
    persist_chunks: bool,
    tx: mpsc::Sender<Event>,
) -> Result<(), ApiError> {
    let mut stream = session.stream;
    let mut stateful_state = stateful.map(|context| {
        let stream_session = context.streams.as_ref().map(|sup| sup.create_session());
        (
            StatefulStreamController::new(
                context.service.clone(),
                context.hub.clone(),
                context.streams.clone(),
                stream_session.clone(),
                context.actor.id,
                context.parent_message_id,
                persist_chunks,
            ),
            stream_session,
        )
    });

    let mut stateless_accumulated = String::new();
    let mut finish_reason: Option<String> = None;
    let mut usage: Option<TokenUsage> = None;
    let mut stream_error: Option<String> = None;
    let mut first_chunk = true;

    'stream_loop: loop {
        let next_future = stream.next();
        tokio::pin!(next_future);

        let next_item = if let Some((_, session_handle)) = stateful_state.as_ref() {
            if let Some(token) = session_handle
                .as_ref()
                .map(|handle| handle.cancellation_token())
            {
                tokio::select! {
                    _ = token.cancelled() => {
                        break 'stream_loop;
                    }
                    item = &mut next_future => item
                }
            } else {
                next_future.await
            }
        } else {
            next_future.await
        };

        let Some(next) = next_item else {
            break;
        };

        match next {
            Ok(chunk) => {
                if let Some(reason) = &chunk.finish_reason {
                    finish_reason = Some(finish_reason_to_string(reason));
                }

                usage = Some(chunk.usage.clone());

                if let Some((controller, _)) = stateful_state.as_mut() {
                    if let Err(err) = controller.process_chunk(&chunk).await {
                        stream_error = Some(err.to_string());
                        break;
                    }
                } else if !chunk.text_delta.is_empty() {
                    stateless_accumulated.push_str(&chunk.text_delta);
                }

                let mut delta = ChatCompletionChunkDelta::default();
                if first_chunk {
                    delta.role = Some("assistant".to_string());
                }
                if !chunk.text_delta.is_empty() {
                    delta.content = Some(chunk.text_delta.clone());
                }

                let chunk_payload = ChatCompletionChunk {
                    id: completion_id.clone(),
                    object: OBJECT_CHUNK.to_string(),
                    created: chunk.timestamp.timestamp(),
                    model: model_name.clone(),
                    system_fingerprint: None,
                    usage: None,
                    choices: vec![ChatCompletionChunkChoice {
                        index: 0,
                        delta,
                        finish_reason: None,
                    }],
                    warnings: if first_chunk {
                        warnings.clone()
                    } else {
                        Vec::new()
                    },
                };

                if tx.send(chunk_event(&chunk_payload)?).await.is_err() {
                    return Ok(());
                }

                first_chunk = false;

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

    let stop_reason = stateful_state
        .as_ref()
        .and_then(|(_, session_handle)| session_handle.as_ref().map(|handle| handle.stop_reason()));

    if let Some((_, Some(session_handle))) = stateful_state.as_ref() {
        session_handle.mark_completed();
    }

    let (final_finish, final_usage) = if let Some((controller, _)) = stateful_state.take() {
        let stateful_error = stream_error.take();
        match controller
            .finalize(
                session.prompt_tokens,
                finish_reason,
                usage,
                stateful_error,
                stop_reason,
            )
            .await
        {
            Ok(result) => {
                if let Some(warning) = result.warning.as_ref() {
                    warnings.push(warning.clone());
                }
                (result.finish_reason, result.usage)
            }
            Err(err) => {
                warnings.push(format!("assistant finalization error: {err}"));
                (
                    "error".to_string(),
                    infer_usage_from_text(session.prompt_tokens, ""),
                )
            }
        }
    } else {
        let error_opt = stream_error;
        if let Some(error) = error_opt.clone() {
            warnings.push(format!("assistant stream error: {error}"));
        }
        let usage_breakdown = usage
            .as_ref()
            .map(|usage| {
                token_usage_to_breakdown(usage, session.prompt_tokens, &stateless_accumulated)
            })
            .unwrap_or_else(|| {
                infer_usage_from_text(session.prompt_tokens, &stateless_accumulated)
            });
        let finish = if error_opt.is_some() {
            "error".to_string()
        } else {
            finish_reason.unwrap_or_else(|| "stop".to_string())
        };
        (finish, usage_breakdown)
    };

    let final_chunk = ChatCompletionChunk {
        id: completion_id,
        object: OBJECT_CHUNK.to_string(),
        created,
        model: model_name,
        system_fingerprint: None,
        usage: Some(final_usage),
        choices: vec![ChatCompletionChunkChoice {
            index: 0,
            delta: ChatCompletionChunkDelta {
                role: None,
                content: None,
            },
            finish_reason: Some(final_finish),
        }],
        warnings,
    };

    if tx.send(chunk_event(&final_chunk)?).await.is_err() {
        return Ok(());
    }

    if tx.send(done_event()).await.is_err() {
        return Ok(());
    }

    Ok(())
}

fn chunk_event(chunk: &ChatCompletionChunk) -> Result<Event, ApiError> {
    let data = serde_json::to_string(chunk)
        .map_err(|err| ApiError::internal_server_error(format!("failed to encode chunk: {err}")))?;
    Ok(Event::default().data(data))
}

fn done_event() -> Event {
    Event::default().data("[DONE]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::Extension, http::StatusCode};
    use axum_test::TestServer;
    use chrono::Utc;
    use futures::stream;
    use shared::{
        config::server::{Config, Profile},
        llms::errors::LLMError,
        llms::types::{FinishReason, LLMConfig},
        models::ChatCompletionResponse,
    };
    use std::{collections::HashMap, sync::Arc};

    use crate::{
        handlers::streaming::StreamHub, middleware::request_context::RequestContext,
        routes::copilot::create_router_copilot, services::assistant_service::AssistantRuntime,
    };
    use serde_json::json;

    struct StubAssistant {
        model: String,
        chunks: Vec<StreamingResponse>,
        config: LLMConfig,
    }

    impl StubAssistant {
        fn new(model: &str, chunks: Vec<StreamingResponse>) -> Self {
            Self {
                model: model.to_string(),
                chunks,
                config: LLMConfig {
                    model_path: "stub.gguf".into(),
                    max_tokens: Some(128),
                    temperature: Some(0.7),
                    top_p: Some(1.0),
                    top_k: None,
                    repetition_penalty: None,
                    n_threads: None,
                    n_gpu_layers: None,
                    context_size: None,
                    batch_size: None,
                    additional_params: HashMap::new(),
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl AssistantRuntime for StubAssistant {
        async fn stream_reply(
            &self,
            _request: LLMRequest,
        ) -> Result<AssistantStreamingSession, AssistantError> {
            let stream = stream::iter(
                self.chunks
                    .clone()
                    .into_iter()
                    .map(|chunk| Ok::<StreamingResponse, LLMError>(chunk)),
            );

            Ok(AssistantStreamingSession::from_stream(Box::pin(stream), 4))
        }

        fn persist_stream_chunks(&self) -> bool {
            false
        }

        fn default_model_name(&self) -> &str {
            &self.model
        }

        fn default_chat_config(&self) -> Result<LLMConfig, AssistantError> {
            Ok(self.config.clone())
        }
    }

    fn stub_chunks() -> Vec<StreamingResponse> {
        vec![
            StreamingResponse {
                request_id: Uuid::new_v4(),
                text_delta: "Hello".to_string(),
                is_final: false,
                current_text: Some("Hello".to_string()),
                finish_reason: None,
                usage: TokenUsage::new(4, 1),
                timestamp: Utc::now(),
            },
            StreamingResponse {
                request_id: Uuid::new_v4(),
                text_delta: " world".to_string(),
                is_final: true,
                current_text: Some("Hello world".to_string()),
                finish_reason: Some(FinishReason::EndOfText),
                usage: TokenUsage::new(4, 2),
                timestamp: Utc::now(),
            },
        ]
    }

    fn test_app(assistant: Arc<dyn AssistantRuntime>) -> TestServer {
        let config = Arc::new(Config::default_for_profile(Profile::Test));
        let hub: SharedStreamHub = Arc::new(StreamHub::new(32, None, None));
        let context = RequestContext {
            request_id: "req".into(),
            session: None,
        };

        let app_state = Arc::new(AppState {
            assistant: Some(assistant),
            ..AppState::default()
        });

        let app = create_router_copilot()
            .layer(Extension(app_state.clone()))
            .layer(Extension(config))
            .layer(Extension(context))
            .layer(Extension(hub))
            .with_state(app_state);

        TestServer::new(app).expect("test server")
    }

    #[tokio::test]
    async fn get_models_uses_configuration() {
        let config = Arc::new(Config::default_for_profile(Profile::Test));
        let response = super::get_models(Extension(config.clone())).await;
        assert!(!response.models.is_empty());
        assert_eq!(response.models[0].object, OBJECT_MODEL);
    }

    #[tokio::test]
    async fn post_chat_completions_returns_final_message() {
        let assistant: Arc<dyn AssistantRuntime> =
            Arc::new(StubAssistant::new("stub-model", stub_chunks()));
        let server = test_app(assistant);

        let response = server
            .post("/v1/chat/completions")
            .json(&json!({
                "model": "stub-model",
                "messages": [
                    { "role": "user", "content": "Hello" }
                ]
            }))
            .await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let body: ChatCompletionResponse = response.json();
        assert_eq!(body.choices.len(), 1);
        assert_eq!(body.choices[0].message.content, "Hello world");
        assert_eq!(body.choices[0].finish_reason.as_deref(), Some("stop"));
        assert_eq!(body.usage.as_ref().unwrap().prompt_tokens, 4);
    }

    #[tokio::test]
    async fn post_chat_completions_streams_sse() {
        let assistant: Arc<dyn AssistantRuntime> =
            Arc::new(StubAssistant::new("stub-model", stub_chunks()));
        let server = test_app(assistant);

        let response = server
            .post("/v1/chat/completions")
            .json(&json!({
                "model": "stub-model",
                "stream": true,
                "messages": [
                    { "role": "user", "content": "Hello" }
                ]
            }))
            .await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let body = response.text();
        assert!(body.contains("\"content\":\"Hello\""));
        assert!(body.contains("[DONE]"));
    }
}

async fn complete_stateless_non_streaming(
    session: AssistantStreamingSession,
    completion_id: String,
    created: i64,
    model_name: String,
    mut warnings: Vec<String>,
) -> AppResult<Response> {
    let mut stream = session.stream;
    let mut accumulated = String::new();
    let mut finish_reason: Option<String> = None;
    let mut usage: Option<TokenUsage> = None;

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

                if chunk.is_final {
                    break;
                }
            }
            Err(err) => {
                warnings.push(format!("assistant stream error: {err}"));
                break;
            }
        }
    }

    let usage_breakdown = usage
        .as_ref()
        .map(|usage| token_usage_to_breakdown(usage, session.prompt_tokens, &accumulated))
        .unwrap_or_else(|| infer_usage_from_text(session.prompt_tokens, &accumulated));

    let finish_reason_value = finish_reason.unwrap_or_else(|| "stop".to_string());

    let response = ChatCompletionResponse {
        id: completion_id,
        object: OBJECT_COMPLETION.to_string(),
        created,
        model: model_name,
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatCompletionMessage {
                role: "assistant".to_string(),
                content: accumulated,
                name: None,
            },
            finish_reason: Some(finish_reason_value),
            logprobs: None,
        }],
        usage: Some(usage_breakdown),
        system_fingerprint: None,
        warnings,
    };

    Ok(Json(response).into_response())
}

async fn complete_stateful_non_streaming(
    session: AssistantStreamingSession,
    completion_id: String,
    created: i64,
    model_name: String,
    mut warnings: Vec<String>,
    context: StatefulContext,
    persist_chunks: bool,
) -> AppResult<Response> {
    let mut stream = session.stream;
    let stream_session = context.streams.as_ref().map(|sup| sup.create_session());
    let mut controller = StatefulStreamController::new(
        context.service.clone(),
        context.hub.clone(),
        context.streams.clone(),
        stream_session.clone(),
        context.actor.id,
        context.parent_message_id,
        persist_chunks,
    );

    let mut finish_reason: Option<String> = None;
    let mut usage: Option<TokenUsage> = None;
    let mut stream_error: Option<String> = None;

    while let Some(next) = stream.next().await {
        match next {
            Ok(chunk) => {
                if let Some(reason) = &chunk.finish_reason {
                    finish_reason = Some(finish_reason_to_string(reason));
                }
                usage = Some(chunk.usage.clone());

                if let Err(err) = controller.process_chunk(&chunk).await {
                    stream_error = Some(err.to_string());
                    break;
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

    let stop_reason = stream_session.as_ref().map(|handle| handle.stop_reason());

    if let Some(session_handle) = stream_session.as_ref() {
        session_handle.mark_completed();
    }

    let finalization = controller
        .finalize(
            session.prompt_tokens,
            finish_reason,
            usage,
            stream_error,
            stop_reason,
        )
        .await
        .map_err(ApiError::from)?;

    if let Some(warning) = finalization.warning.as_ref() {
        warnings.push(warning.clone());
    }

    let response = ChatCompletionResponse {
        id: completion_id,
        object: OBJECT_COMPLETION.to_string(),
        created,
        model: model_name,
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatCompletionMessage {
                role: "assistant".to_string(),
                content: finalization.accumulated,
                name: None,
            },
            finish_reason: Some(finalization.finish_reason),
            logprobs: None,
        }],
        usage: Some(finalization.usage),
        system_fingerprint: None,
        warnings,
    };

    Ok(Json(response).into_response())
}

#[instrument(skip(state, config, _context, hub, headers, payload))]
pub async fn post_chat_completions(
    Extension(state): Extension<Arc<AppState>>,
    Extension(config): Extension<Arc<Config>>,
    Extension(_context): Extension<RequestContext>,
    Extension(hub): Extension<SharedStreamHub>,
    headers: HeaderMap,
    Json(payload): Json<ChatCompletionRequest>,
) -> AppResult<Response> {
    let assistant = state.assistant.clone().ok_or_else(|| {
        ApiError::internal_server_error("assistant streaming service not configured")
    })?;

    let stream = payload.stream.unwrap_or(false);
    let stop_sequences = parse_stop_sequences(&payload.stop)?;
    let overrides = CompletionOverrides::from_request(&payload, stop_sequences);
    let warnings = gather_warnings(&payload);

    let completion_id = format!("chatcmpl-{}", Uuid::new_v4());
    let created = Utc::now().timestamp();

    let auth_session = authenticate_request(&state, &config, &headers).await?;
    let metadata = parse_rustygpt_metadata(&payload.metadata)?;

    if metadata.is_some() && auth_session.is_none() {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "RGP.AUTH.INVALID_SESSION",
            "metadata.rustygpt requires an authenticated session",
        ));
    }

    let default_config = assistant
        .default_chat_config()
        .map_err(|err| ApiError::internal_server_error(err.to_string()))?;

    let stateful_context = if let Some(meta) = metadata {
        let validation = auth_session
            .as_ref()
            .expect("metadata implies validated session");
        Some(prepare_stateful_context(&state, &hub, validation, meta, &payload).await?)
    } else {
        None
    };

    let llm_request = if let Some(context) = stateful_context.as_ref() {
        build_stateful_request(context, &default_config, &payload.model, &overrides, stream)
    } else {
        build_stateless_request(
            &payload.messages,
            &default_config,
            &payload.model,
            &overrides,
            stream,
        )
    };

    let session = assistant
        .stream_reply(llm_request)
        .await
        .map_err(map_assistant_error)?;

    let persist_chunks = assistant.persist_stream_chunks();

    let mut response = if stream {
        stream_completion(
            session,
            completion_id,
            created,
            payload.model.clone(),
            warnings,
            stateful_context,
            persist_chunks,
        )
        .await?
    } else {
        complete_non_streaming(
            session,
            completion_id,
            created,
            payload.model.clone(),
            warnings,
            stateful_context,
            persist_chunks,
        )
        .await?
    };

    apply_session_rotation(&mut response, auth_session.as_ref());
    Ok(response)
}

const OBJECT_MODEL: &str = "model";
