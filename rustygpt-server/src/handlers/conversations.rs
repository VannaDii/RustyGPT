use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    http::error::{ApiError, AppResult},
    middleware::request_context::RequestContext,
    services::chat_service::ChatService,
};
use shared::models::{
    AcceptInviteRequest, AddParticipantRequest, ConversationCreateRequest, CreateInviteRequest,
    CreateInviteResponse, ThreadListResponse, UnreadSummaryResponse,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/conversations", post(create_conversation))
        .route(
            "/api/conversations/{conversation_id}/participants",
            post(add_participant),
        )
        .route(
            "/api/conversations/{conversation_id}/participants/{user_id}",
            axum::routing::delete(remove_participant),
        )
        .route(
            "/api/conversations/{conversation_id}/invites",
            post(create_invite),
        )
        .route(
            "/api/conversations/{conversation_id}/threads",
            get(list_threads),
        )
        .route(
            "/api/conversations/{conversation_id}/unread",
            get(unread_summary),
        )
        .route("/api/invites/accept", post(accept_invite))
        .route("/api/invites/{token}/revoke", post(revoke_invite))
}

#[derive(Deserialize, Default)]
struct ThreadListQuery {
    after: Option<DateTime<Utc>>,
    limit: Option<i32>,
}

#[instrument(skip(app_state, context, payload))]
async fn create_conversation(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Json(payload): Json<ConversationCreateRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    let created = service.create_conversation(user_id, payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[instrument(skip(app_state, context, payload))]
async fn add_participant(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(conversation_id): Path<Uuid>,
    Json(payload): Json<AddParticipantRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    service
        .add_participant(user_id, conversation_id, payload)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(app_state, context))]
async fn remove_participant(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path((conversation_id, user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<impl IntoResponse> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    service
        .remove_participant(actor, conversation_id, user_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(app_state, context, payload))]
async fn create_invite(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(conversation_id): Path<Uuid>,
    Json(payload): Json<CreateInviteRequest>,
) -> AppResult<Json<CreateInviteResponse>> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    let ttl = payload.ttl_seconds.unwrap_or(86_400);
    let response = service
        .create_invite(
            actor,
            conversation_id,
            &payload.email,
            payload.role,
            Some(ttl),
        )
        .await?;

    Ok(Json(response))
}

#[instrument(skip(app_state, context, payload))]
async fn accept_invite(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Json(payload): Json<AcceptInviteRequest>,
) -> AppResult<impl IntoResponse> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    let conversation_id = service.accept_invite(actor, &payload.token).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"conversation_id": conversation_id})),
    ))
}

#[instrument(skip(app_state, context))]
async fn revoke_invite(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(token): Path<String>,
) -> AppResult<impl IntoResponse> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    service.revoke_invite(actor, &token).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(app_state, context, query))]
async fn list_threads(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(conversation_id): Path<Uuid>,
    Query(query): Query<ThreadListQuery>,
) -> AppResult<Json<ThreadListResponse>> {
    let user_id = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    let response = service
        .list_threads(user_id, conversation_id, query.after, query.limit)
        .await?;

    Ok(Json(response))
}

#[instrument(skip(app_state, context))]
async fn unread_summary(
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(conversation_id): Path<Uuid>,
) -> AppResult<Json<UnreadSummaryResponse>> {
    let actor = require_user(&context)?;
    let pool = require_pool(&app_state)?;
    let service = ChatService::new(pool);

    let threads = service.unread_summary(actor, conversation_id).await?;
    Ok(Json(UnreadSummaryResponse { threads }))
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
