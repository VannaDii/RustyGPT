use std::sync::Arc;

use axum::{
    Json,
    extract::Extension,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use tracing::instrument;

use crate::{
    app_state::AppState,
    auth::session::{SessionError, SessionMetadata, SessionService},
    http::error::{ApiError, AppResult},
    middleware::request_context::RequestContext,
};
use shared::{
    config::server::{Config, CookieSameSite},
    models::{
        AuthenticatedUser, LoginRequest, LoginResponse, MeResponse, SessionSummary, Timestamp,
    },
};
use time::{Duration as TimeDuration, OffsetDateTime};

fn map_session_error(error: SessionError) -> ApiError {
    match error {
        SessionError::InvalidCredentials => ApiError::new(
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "invalid credentials",
        ),
        SessionError::SessionExpired | SessionError::AbsoluteExpired => ApiError::new(
            StatusCode::UNAUTHORIZED,
            "session_expired",
            "session expired",
        ),
        SessionError::DisabledUser => {
            ApiError::new(StatusCode::LOCKED, "user_disabled", "user account disabled")
        }
        SessionError::RotationRequired => ApiError::new(
            StatusCode::CONFLICT,
            "session_conflict",
            "session rotation failed",
        ),
        other => ApiError::internal_server_error(other.to_string()),
    }
}

fn metadata_from_headers(headers: &HeaderMap) -> SessionMetadata {
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|ua| ua.to_string());
    let forwarded_ip = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string());
    let real_ip = headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let fingerprint = headers
        .get("x-client-fingerprint")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    SessionMetadata::default()
        .with_user_agent(user_agent)
        .with_ip_str(forwarded_ip.or(real_ip))
        .with_fingerprint(fingerprint)
}

fn extract_session_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| {
            cookie::Cookie::split_parse(raw)
                .flatten()
                .find(|cookie| cookie.name() == name)
                .map(|cookie| cookie.value().to_string())
        })
}

fn build_authenticated_user(user: &crate::auth::session::SessionUser) -> AuthenticatedUser {
    AuthenticatedUser {
        id: user.id,
        email: user.email.clone(),
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        roles: user.roles.clone(),
    }
}

const fn build_session_summary(bundle: &crate::auth::session::SessionBundle) -> SessionSummary {
    SessionSummary {
        id: bundle.session_id,
        issued_at: Timestamp(bundle.issued_at),
        expires_at: Timestamp(bundle.expires_at),
        absolute_expires_at: Timestamp(bundle.absolute_expires_at),
    }
}

fn apply_cookies(response: &mut Response, cookies: &[cookie::Cookie<'static>]) {
    for cookie in cookies {
        if let Ok(value) = HeaderValue::from_str(&cookie.to_string()) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }
}

fn session_service(state: &Arc<AppState>) -> Result<Arc<SessionService>, ApiError> {
    state
        .sessions
        .clone()
        .ok_or_else(|| ApiError::internal_server_error("session service unavailable"))
}

fn clear_cookie(
    config: &Config,
    name: &str,
    http_only: bool,
    same_site: cookie::SameSite,
) -> cookie::Cookie<'static> {
    let mut builder = cookie::Cookie::build((name.to_string(), String::new()))
        .path("/")
        .http_only(http_only)
        .secure(config.security.cookie.secure)
        .same_site(same_site)
        .max_age(TimeDuration::seconds(0))
        .expires(OffsetDateTime::UNIX_EPOCH);

    if let Some(domain) = &config.security.cookie.domain {
        builder = builder.domain(domain.clone());
    }

    builder.build()
}

#[instrument(skip(state, headers, payload))]
pub async fn login(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Response> {
    let service = session_service(&state)?;
    let metadata = metadata_from_headers(&headers);

    if payload.email.trim().is_empty() || payload.password.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_request",
            "email and password are required",
        ));
    }

    let (user, bundle) = service
        .authenticate(payload.email.trim(), &payload.password, &metadata)
        .await
        .map_err(map_session_error)?;

    let response_body = LoginResponse {
        user: build_authenticated_user(&user),
        session: build_session_summary(&bundle),
        csrf_token: bundle.csrf_token.clone(),
    };

    let mut response = Json(response_body).into_response();
    apply_cookies(
        &mut response,
        &[bundle.session_cookie.clone(), bundle.csrf_cookie.clone()],
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-session-rotated"),
        HeaderValue::from_static("1"),
    );

    Ok(response)
}

#[instrument(skip(state, headers))]
pub async fn refresh(
    Extension(state): Extension<Arc<AppState>>,
    Extension(config): Extension<Arc<Config>>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let service = session_service(&state)?;
    let metadata = metadata_from_headers(&headers);

    let session_cookie_name = config.session.session_cookie_name.clone();
    let token = extract_session_cookie(&headers, &session_cookie_name).ok_or_else(|| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "missing_session",
            "session cookie missing",
        )
    })?;

    let (user, bundle) = service
        .refresh_session(&token, &metadata)
        .await
        .map_err(map_session_error)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "invalid_session",
                "session expired",
            )
        })?;

    let response_body = LoginResponse {
        user: build_authenticated_user(&user),
        session: build_session_summary(&bundle),
        csrf_token: bundle.csrf_token.clone(),
    };

    let mut response = Json(response_body).into_response();
    apply_cookies(
        &mut response,
        &[bundle.session_cookie.clone(), bundle.csrf_cookie.clone()],
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-session-rotated"),
        HeaderValue::from_static("1"),
    );

    Ok(response)
}

#[instrument(skip(context))]
pub async fn me(Extension(context): Extension<RequestContext>) -> AppResult<Json<MeResponse>> {
    let session = context.session.ok_or_else(|| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "authentication required",
        )
    })?;

    let user = build_authenticated_user(&session);
    let summary = SessionSummary {
        id: session.session_id,
        issued_at: Timestamp(session.issued_at),
        expires_at: Timestamp(session.expires_at),
        absolute_expires_at: Timestamp(session.absolute_expires_at),
    };

    Ok(Json(MeResponse {
        user,
        session: summary,
    }))
}

#[instrument(skip(state, headers))]
pub async fn logout(
    Extension(state): Extension<Arc<AppState>>,
    Extension(config): Extension<Arc<Config>>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let service = session_service(&state)?;
    let metadata = metadata_from_headers(&headers);
    let session_cookie_name = config.session.session_cookie_name.clone();

    let token = extract_session_cookie(&headers, &session_cookie_name).ok_or_else(|| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "missing_session",
            "session cookie missing",
        )
    })?;

    let validation = service
        .validate_session(&token, &metadata)
        .await
        .map_err(map_session_error)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "invalid_session",
                "session expired",
            )
        })?;

    service
        .revoke_session_by_id(validation.user.session_id, Some("logout"))
        .await
        .map_err(map_session_error)?;

    let mut response = Response::new(axum::body::Body::empty());
    if let Ok(value) = HeaderValue::from_str(
        &clear_cookie(
            &config,
            &session_cookie_name,
            true,
            match config.security.cookie.same_site {
                CookieSameSite::Lax => cookie::SameSite::Lax,
                CookieSameSite::Strict => cookie::SameSite::Strict,
                CookieSameSite::None => cookie::SameSite::None,
            },
        )
        .to_string(),
    ) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
    if let Ok(value) = HeaderValue::from_str(
        &clear_cookie(
            &config,
            &config.session.csrf_cookie_name,
            false,
            cookie::SameSite::Strict,
        )
        .to_string(),
    ) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
    response.headers_mut().insert(
        header::HeaderName::from_static("x-session-rotated"),
        HeaderValue::from_static("1"),
    );
    *response.status_mut() = StatusCode::NO_CONTENT;

    Ok(response)
}
