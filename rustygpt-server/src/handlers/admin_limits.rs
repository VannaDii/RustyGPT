use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::Value;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auth::session::SessionUser,
    http::error::{ApiError, AppResult},
    middleware::request_context::RequestContext,
};
use shared::models::Timestamp;
use shared::models::{
    AssignRateLimitRequest, CreateRateLimitProfileRequest, RateLimitAssignment, RateLimitProfile,
    UpdateRateLimitProfileRequest, UserRole,
};

fn require_pool(state: &Arc<AppState>) -> AppResult<PgPool> {
    state.pool.clone().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
            "database pool not configured",
        )
    })
}

fn require_admin_context(context: &RequestContext) -> AppResult<&SessionUser> {
    let session = context.session.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "authentication required",
        )
    })?;

    if session
        .roles
        .iter()
        .any(|role| matches!(role, UserRole::Admin))
    {
        Ok(session)
    } else {
        Err(ApiError::forbidden("admin privileges required"))
    }
}

fn apply_profile(row: DbProfileRow) -> RateLimitProfile {
    RateLimitProfile {
        id: row.profile_id,
        name: row.name,
        algorithm: row.algorithm,
        params: row.params,
        description: row.description,
        created_at: Timestamp(row.created_at),
        updated_at: Timestamp(row.updated_at),
    }
}

fn apply_assignment(row: DbAssignmentRow) -> RateLimitAssignment {
    RateLimitAssignment {
        id: row.assignment_id,
        profile_id: row.profile_id,
        profile_name: row.profile_name,
        method: row.method,
        path_pattern: row.path_pattern,
        created_at: Timestamp(row.created_at),
        updated_at: Timestamp(row.updated_at),
    }
}

fn normalize_method(method: &str) -> String {
    method.trim().to_uppercase()
}

fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        return "/".to_string();
    }
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

async fn reload_limits(state: &Arc<AppState>) {
    if let Some(rate_limits) = state.rate_limits.clone()
        && let Err(err) = rate_limits.reload_from_db().await
    {
        warn!(error = %err, "failed to reload rate limit configuration after admin change");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;
    use serde_json::json;
    use shared::models::CreateRateLimitProfileRequest;
    use uuid::Uuid;

    fn make_context_with_roles(roles: Vec<UserRole>) -> RequestContext {
        let now = chrono::Utc::now();
        RequestContext {
            request_id: "test".into(),
            session: Some(SessionUser {
                id: Uuid::new_v4(),
                email: "user@example.com".into(),
                username: "user".into(),
                display_name: None,
                roles,
                session_id: Uuid::new_v4(),
                issued_at: now,
                expires_at: now,
                absolute_expires_at: now,
            }),
        }
    }

    #[tokio::test]
    async fn list_profiles_requires_admin_role() {
        let state = Arc::new(AppState::default());
        let context = make_context_with_roles(vec![UserRole::Member]);

        let status = match list_profiles(Extension(state), Extension(context)).await {
            Ok(_) => panic!("expected forbidden"),
            Err(err) => err.into_response().status(),
        };

        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn list_profiles_requires_session() {
        let state = Arc::new(AppState::default());
        let context = RequestContext {
            request_id: "test".into(),
            session: None,
        };

        let status = match list_profiles(Extension(state), Extension(context)).await {
            Ok(_) => panic!("expected unauthorized"),
            Err(err) => err.into_response().status(),
        };

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn create_profile_without_pool_returns_service_unavailable() {
        let state = Arc::new(AppState::default());
        let context = make_context_with_roles(vec![UserRole::Admin]);
        let payload = CreateRateLimitProfileRequest {
            name: "burst".into(),
            algorithm: "gcra".into(),
            params: json!({ "requests_per_second": 5 }),
            description: None,
        };

        let status = match create_profile(Extension(state), Extension(context), Json(payload)).await
        {
            Ok(_) => panic!("expected service unavailable"),
            Err(err) => err.into_response().status(),
        };

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn normalize_method_uppercases_input() {
        assert_eq!(normalize_method("get"), "GET");
        assert_eq!(normalize_method("PoSt"), "POST");
    }

    #[test]
    fn normalize_path_adds_leading_slash() {
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("admin/limits"), "/admin/limits");
        assert_eq!(normalize_path("/already"), "/already");
    }
}
#[derive(sqlx::FromRow)]
struct DbProfileRow {
    profile_id: Uuid,
    name: String,
    algorithm: String,
    params: Value,
    description: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct DbAssignmentRow {
    assignment_id: Uuid,
    profile_id: Uuid,
    profile_name: String,
    method: String,
    path_pattern: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_profiles(
    Extension(state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Vec<RateLimitProfile>>> {
    require_admin_context(&context)?;
    let pool = require_pool(&state)?;

    let rows = sqlx::query_as::<_, DbProfileRow>(
        "SELECT profile_id, name, algorithm, params, description, created_at, updated_at FROM rustygpt.sp_limits_list_profiles()",
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows.into_iter().map(apply_profile).collect()))
}

pub async fn create_profile(
    Extension(state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Json(payload): Json<CreateRateLimitProfileRequest>,
) -> AppResult<Response> {
    require_admin_context(&context)?;
    if payload.name.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_profile",
            "profile name must not be empty",
        ));
    }

    let pool = require_pool(&state)?;

    let row = sqlx::query_as::<_, DbProfileRow>(
        "SELECT * FROM rustygpt.sp_limits_create_profile($1, $2, $3, $4)",
    )
    .bind(payload.name.trim())
    .bind(payload.algorithm.trim())
    .bind(payload.params)
    .bind(payload.description.as_deref())
    .fetch_one(&pool)
    .await?;

    reload_limits(&state).await;

    let response = apply_profile(row);
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

pub async fn update_profile(
    Extension(state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(profile_id): Path<Uuid>,
    Json(payload): Json<UpdateRateLimitProfileRequest>,
) -> AppResult<Json<RateLimitProfile>> {
    require_admin_context(&context)?;
    let pool = require_pool(&state)?;

    let row = sqlx::query_as::<_, DbProfileRow>(
        "SELECT * FROM rustygpt.sp_limits_update_profile($1, $2, $3)",
    )
    .bind(profile_id)
    .bind(payload.params)
    .bind(payload.description.as_deref())
    .fetch_one(&pool)
    .await?;

    reload_limits(&state).await;

    Ok(Json(apply_profile(row)))
}

pub async fn delete_profile(
    Extension(state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(profile_id): Path<Uuid>,
) -> AppResult<Response> {
    require_admin_context(&context)?;
    let pool = require_pool(&state)?;

    let deleted = sqlx::query_scalar::<_, bool>("SELECT rustygpt.sp_limits_delete_profile($1)")
        .bind(profile_id)
        .fetch_one(&pool)
        .await?;

    if !deleted {
        return Err(ApiError::not_found("profile not found"));
    }

    reload_limits(&state).await;

    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn list_assignments(
    Extension(state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
) -> AppResult<Json<Vec<RateLimitAssignment>>> {
    require_admin_context(&context)?;
    let pool = require_pool(&state)?;

    let rows = sqlx::query_as::<_, DbAssignmentRow>(
        "SELECT assignment_id, profile_id, profile_name, method, path_pattern, created_at, updated_at FROM rustygpt.sp_limits_list_assignments()",
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows.into_iter().map(apply_assignment).collect()))
}

pub async fn assign_route(
    Extension(state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Json(payload): Json<AssignRateLimitRequest>,
) -> AppResult<Response> {
    require_admin_context(&context)?;
    let pool = require_pool(&state)?;

    if payload.method.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_assignment",
            "HTTP method is required",
        ));
    }

    let method = normalize_method(&payload.method);
    let path = normalize_path(&payload.path);

    let row = sqlx::query_as::<_, DbAssignmentRow>(
        "SELECT * FROM rustygpt.sp_limits_assign_route($1, $2, $3)",
    )
    .bind(payload.profile_id)
    .bind(method)
    .bind(path)
    .fetch_one(&pool)
    .await?;

    reload_limits(&state).await;

    Ok((StatusCode::CREATED, Json(apply_assignment(row))).into_response())
}

pub async fn delete_assignment(
    Extension(state): Extension<Arc<AppState>>,
    Extension(context): Extension<RequestContext>,
    Path(assignment_id): Path<Uuid>,
) -> AppResult<Response> {
    require_admin_context(&context)?;
    let pool = require_pool(&state)?;

    let deleted = sqlx::query_scalar::<_, bool>("SELECT rustygpt.sp_limits_delete_assignment($1)")
        .bind(assignment_id)
        .fetch_one(&pool)
        .await?;

    if !deleted {
        return Err(ApiError::not_found("assignment not found"));
    }

    reload_limits(&state).await;

    Ok(StatusCode::NO_CONTENT.into_response())
}
