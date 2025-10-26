use std::{net::IpAddr, sync::Arc};

use axum::{
    Json,
    extract::Extension,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use tracing::instrument;

use crate::{
    app_state::AppState,
    auth::session::{SessionError, SessionManager, SessionMetadata},
    http::error::{ApiError, AppResult},
    middleware::request_context::RequestContext,
};
use serde_json::json;
use shared::{
    config::server::{Config, CookieSameSite},
    models::{
        AuthenticatedUser, LoginRequest, LoginResponse, MeResponse, SessionSummary, Timestamp,
    },
};
use time::{Duration as TimeDuration, OffsetDateTime};

pub fn map_session_error(error: SessionError) -> ApiError {
    match error {
        SessionError::InvalidCredentials => ApiError::new(
            StatusCode::UNAUTHORIZED,
            "RGP.AUTH.INVALID_CREDENTIALS",
            "invalid credentials",
        ),
        SessionError::SessionExpired | SessionError::AbsoluteExpired => ApiError::new(
            StatusCode::UNAUTHORIZED,
            "RGP.AUTH.EXPIRED",
            "session expired",
        ),
        SessionError::DisabledUser => ApiError::new(
            StatusCode::LOCKED,
            "RGP.AUTH.DISABLED",
            "user account disabled",
        ),
        SessionError::RotationRequired => ApiError::new(
            StatusCode::CONFLICT,
            "RGP.AUTH.ROTATION_REQUIRED",
            "session rotation failed",
        ),
        SessionError::SuspiciousActivity => ApiError::new(
            StatusCode::UNAUTHORIZED,
            "RGP.AUTH.SUSPICIOUS",
            "session requires refresh",
        )
        .with_header(
            header::WWW_AUTHENTICATE,
            HeaderValue::from_static("refresh"),
        ),
        other => ApiError::internal_server_error(other.to_string()),
    }
}

pub fn metadata_from_headers(headers: &HeaderMap) -> SessionMetadata {
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

    let ip_source = forwarded_ip.or(real_ip);

    let mut metadata = SessionMetadata::default()
        .with_user_agent(user_agent)
        .with_fingerprint(fingerprint);

    if let Some(ip_value) = ip_source {
        match ip_value.parse::<IpAddr>() {
            Ok(parsed) => {
                metadata = metadata.with_ip(Some(parsed));
            }
            Err(_) => {
                metadata = metadata.with_ip_str(Some(ip_value));
            }
        }
    }

    metadata
}

pub fn extract_session_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
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

fn session_service(state: &Arc<AppState>) -> Result<Arc<dyn SessionManager>, ApiError> {
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

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::session::{
            SessionBundle, SessionManager, SessionMetadata, SessionUser, SessionValidation,
        },
        middleware::{auth::auth_middleware, csrf},
        server,
        services::chat_service::ChatServiceError,
    };
    use async_trait::async_trait;
    use axum::{
        Extension, Json, Router,
        body::Body,
        http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
        response::IntoResponse,
        routing::post,
    };
    use axum_test::TestServer;
    use chrono::{Duration as ChronoDuration, Utc};
    use cookie::{Cookie, SameSite};
    use serde_json::json;
    use shared::config::server::{Config, Profile};
    use sqlx::Error as SqlxError;
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;

    fn sample_session_bundle() -> SessionBundle {
        SessionBundle {
            session_cookie: Cookie::build(("session", "token")).path("/").build(),
            csrf_token: "csrf-token".into(),
            csrf_cookie: Cookie::build(("csrf", "token")).path("/").build(),
            session_id: Uuid::new_v4(),
            issued_at: Utc::now(),
            expires_at: Utc::now(),
            absolute_expires_at: Utc::now(),
        }
    }

    fn sample_session_user() -> SessionUser {
        SessionUser {
            id: Uuid::new_v4(),
            email: "user@example.com".into(),
            username: "user".into(),
            display_name: Some("User".into()),
            roles: vec![shared::models::UserRole::Member],
            session_id: Uuid::new_v4(),
            issued_at: Utc::now(),
            expires_at: Utc::now(),
            absolute_expires_at: Utc::now(),
        }
    }

    #[test]
    fn map_session_error_maps_status_codes() {
        let unauthorized = map_session_error(SessionError::InvalidCredentials)
            .into_response()
            .status();
        assert_eq!(unauthorized, StatusCode::UNAUTHORIZED);

        let locked = map_session_error(SessionError::DisabledUser)
            .into_response()
            .status();
        assert_eq!(locked, StatusCode::LOCKED);

        let conflict = map_session_error(SessionError::RotationRequired)
            .into_response()
            .status();
        assert_eq!(conflict, StatusCode::CONFLICT);

        let suspicious = map_session_error(SessionError::SuspiciousActivity).into_response();
        assert_eq!(suspicious.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            suspicious.headers().get(header::WWW_AUTHENTICATE).unwrap(),
            "refresh"
        );
    }

    #[test]
    fn metadata_from_headers_extracts_values() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static("integration-test"),
        );
        headers.insert(
            HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static("10.0.0.2, proxy"),
        );
        headers.insert(
            HeaderName::from_static("x-client-fingerprint"),
            HeaderValue::from_static("abc123"),
        );

        let metadata = metadata_from_headers(&headers);
        assert_eq!(metadata.user_agent, Some("integration-test".into()));
        assert_eq!(metadata.ip, Some("10.0.0.2".into()));
        assert_eq!(metadata.fingerprint, Some("abc123".into()));
    }

    #[test]
    fn build_authenticated_user_clones_fields() {
        let source = sample_session_user();
        let auth_user = build_authenticated_user(&source);
        assert_eq!(auth_user.id, source.id);
        assert_eq!(auth_user.email, source.email);
        assert_eq!(auth_user.roles, source.roles);
    }

    #[test]
    fn build_session_summary_wraps_timestamps() {
        let bundle = sample_session_bundle();
        let summary = build_session_summary(&bundle);
        assert_eq!(summary.id, bundle.session_id);
        assert_eq!(summary.issued_at.0, bundle.issued_at);
        assert_eq!(summary.absolute_expires_at.0, bundle.absolute_expires_at);
    }

    #[test]
    fn login_sets_cookie_and_csrf_exact_values() {
        let mut response = Response::new(Body::empty());
        let bundle = sample_session_bundle();
        let cookies = [bundle.session_cookie.clone(), bundle.csrf_cookie.clone()];
        apply_cookies(&mut response, &cookies);
        let headers = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert_eq!(headers.len(), 2);
        assert!(
            headers
                .iter()
                .any(|value| value.starts_with("session=") && value.contains("Path=/"))
        );
        assert!(
            headers
                .iter()
                .any(|value| value.starts_with("csrf=") && value.contains("Path=/"))
        );
    }

    #[test]
    fn refresh_rotates_cookie_and_sets_x_session_rotated() {
        let bundle = sample_session_bundle();
        let response_body = LoginResponse {
            user: build_authenticated_user(&sample_session_user()),
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

        assert_eq!(
            response
                .headers()
                .get(header::HeaderName::from_static("x-session-rotated"))
                .map(|value| value.to_str().unwrap()),
            Some("1")
        );
    }

    #[test]
    fn session_service_errors_without_configured_service() {
        let state = Arc::new(AppState::default());
        let status = session_service(&state)
            .err()
            .expect("expected missing session service")
            .into_response()
            .status();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn clear_cookie_matches_configuration() {
        let mut config = Config::default_for_profile(Profile::Dev);
        config.security.cookie.secure = true;
        config.security.cookie.domain = Some("example.com".into());
        let cookie = clear_cookie(&config, "session", true, cookie::SameSite::Strict);
        assert_eq!(cookie.name(), "session");
        assert!(cookie.http_only().unwrap());
        assert_eq!(cookie.domain(), Some("example.com"));
        assert_eq!(cookie.same_site(), Some(cookie::SameSite::Strict));
    }

    #[test]
    fn extract_session_cookie_reads_specific_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("foo=bar; session=token123"),
        );
        let value = extract_session_cookie(&headers, "session");
        assert_eq!(value.as_deref(), Some("token123"));
    }

    #[test]
    fn chat_service_errors_map_to_matching_status_codes() {
        let validation = ApiError::from(ChatServiceError::Validation("bad".into()))
            .into_response()
            .status();
        assert_eq!(validation, StatusCode::BAD_REQUEST);

        let not_found = ApiError::from(ChatServiceError::NotFound("missing".into()))
            .into_response()
            .status();
        assert_eq!(not_found, StatusCode::NOT_FOUND);

        let forbidden = ApiError::from(ChatServiceError::Forbidden("nope".into()))
            .into_response()
            .status();
        assert_eq!(forbidden, StatusCode::FORBIDDEN);

        let limited = ApiError::from(ChatServiceError::RateLimited("slow".into()))
            .into_response()
            .status();
        assert_eq!(limited, StatusCode::TOO_MANY_REQUESTS);

        let db = ApiError::from(ChatServiceError::Database(SqlxError::PoolTimedOut))
            .into_response()
            .status();
        assert_eq!(db, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[derive(Default)]
    struct StubSessionManager {
        authenticate: Mutex<VecDeque<Result<(SessionUser, SessionBundle), SessionError>>>,
        validate: Mutex<VecDeque<Result<Option<SessionValidation>, SessionError>>>,
        refresh: Mutex<VecDeque<Result<Option<(SessionUser, SessionBundle)>, SessionError>>>,
    }

    impl StubSessionManager {
        fn enqueue_auth(&self, response: Result<(SessionUser, SessionBundle), SessionError>) {
            self.authenticate.lock().unwrap().push_back(response);
        }

        fn enqueue_validate(&self, response: Result<Option<SessionValidation>, SessionError>) {
            self.validate.lock().unwrap().push_back(response);
        }

        fn enqueue_refresh(
            &self,
            response: Result<Option<(SessionUser, SessionBundle)>, SessionError>,
        ) {
            self.refresh.lock().unwrap().push_back(response);
        }
    }

    #[async_trait]
    impl SessionManager for StubSessionManager {
        async fn authenticate(
            &self,
            _identifier: &str,
            _password: &str,
            _metadata: &SessionMetadata,
        ) -> Result<(SessionUser, SessionBundle), SessionError> {
            self.authenticate
                .lock()
                .unwrap()
                .pop_front()
                .expect("missing authenticate response")
        }

        async fn validate_session(
            &self,
            _token: &str,
            _metadata: &SessionMetadata,
        ) -> Result<Option<SessionValidation>, SessionError> {
            self.validate
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(None))
        }

        async fn refresh_session(
            &self,
            _token: &str,
            _metadata: &SessionMetadata,
        ) -> Result<Option<(SessionUser, SessionBundle)>, SessionError> {
            self.refresh.lock().unwrap().pop_front().unwrap_or(Ok(None))
        }

        async fn revoke_session_by_id(
            &self,
            _session_id: Uuid,
            _reason: Option<&str>,
        ) -> Result<(), SessionError> {
            Ok(())
        }

        async fn mark_user_for_rotation(
            &self,
            _user_id: Uuid,
            _reason: &str,
        ) -> Result<i64, SessionError> {
            Ok(0)
        }
    }

    fn test_config() -> Arc<Config> {
        let mut config = Config::default_for_profile(Profile::Test);
        config.features.auth_v1 = true;
        config.security.cookie.secure = false;
        config.security.cookie.same_site = CookieSameSite::Lax;
        Arc::new(config)
    }

    fn build_session_artifacts() -> (SessionUser, SessionBundle) {
        let session_id = Uuid::new_v4();
        let issued_at = Utc::now();
        let expires_at = issued_at + ChronoDuration::hours(1);
        let absolute_expires_at = issued_at + ChronoDuration::hours(4);

        let session_cookie = Cookie::build(("SESSION_ID", "session-token"))
            .http_only(true)
            .path("/")
            .same_site(SameSite::Lax)
            .build();
        let csrf_cookie = Cookie::build(("CSRF-TOKEN", "csrf-token"))
            .http_only(false)
            .path("/")
            .same_site(SameSite::Strict)
            .build();

        let bundle = SessionBundle {
            session_cookie,
            csrf_token: "csrf-token".into(),
            csrf_cookie,
            session_id,
            issued_at,
            expires_at,
            absolute_expires_at,
        };

        let user = SessionUser {
            id: Uuid::new_v4(),
            email: "integration@example.com".into(),
            username: "integration".into(),
            display_name: Some("Integration".into()),
            roles: vec![shared::models::UserRole::Member],
            session_id,
            issued_at,
            expires_at,
            absolute_expires_at,
        };

        (user, bundle)
    }

    #[tokio::test]
    async fn login_handler_sets_cookies_and_returns_payload() {
        let stub = Arc::new(StubSessionManager::default());
        let (user, bundle) = build_session_artifacts();
        stub.enqueue_auth(Ok((user.clone(), bundle.clone())));

        let session_manager: Arc<dyn SessionManager> = stub.clone();
        let state = server::create_app_state(None, None, None, Some(session_manager), None, None);

        let app = Router::new()
            .route("/api/auth/login", post(login))
            .layer(Extension(state));

        let server = TestServer::new(app).expect("test server");
        let response = server
            .post("/api/auth/login")
            .json(&LoginRequest {
                email: "integration@example.com".into(),
                password: "secret".into(),
            })
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let cookies = response.cookies();
        let body: LoginResponse = response.json();

        assert_eq!(body.user.id, user.id);
        assert_eq!(body.session.id, bundle.session_id);
        assert_eq!(body.csrf_token, bundle.csrf_token);

        let session = cookies
            .iter()
            .find(|cookie| cookie.name() == "SESSION_ID")
            .expect("session cookie");
        assert_eq!(session.value(), "session-token");
        let csrf = cookies
            .iter()
            .find(|cookie| cookie.name() == "CSRF-TOKEN")
            .expect("csrf cookie");
        assert_eq!(csrf.value(), "csrf-token");
    }

    #[tokio::test]
    async fn refresh_returns_rotated_session_and_header() {
        let config = test_config();
        let stub = Arc::new(StubSessionManager::default());
        let (user, bundle) = build_session_artifacts();
        stub.enqueue_refresh(Ok(Some((user.clone(), bundle.clone()))));

        let session_manager: Arc<dyn SessionManager> = stub.clone();
        let state = server::create_app_state(None, None, None, Some(session_manager), None, None);

        let app = Router::new()
            .route("/api/auth/refresh", post(refresh))
            .layer(Extension(config.clone()))
            .layer(Extension(state));

        let server = TestServer::new(app).expect("test server");
        let response = server
            .post("/api/auth/refresh")
            .add_header(
                header::COOKIE,
                "SESSION_ID=existing-session; CSRF-TOKEN=token",
            )
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("x-session-rotated")
                .expect("rotation header"),
            "1"
        );

        let body: LoginResponse = response.json();
        assert_eq!(body.session.id, bundle.session_id);
        assert_eq!(body.csrf_token, bundle.csrf_token);
    }

    async fn messages_handler() -> impl IntoResponse {
        Json(json!({ "ok": true }))
    }

    #[tokio::test]
    async fn login_then_mutation_with_csrf_succeeds() {
        let config = test_config();
        let stub = Arc::new(StubSessionManager::default());
        let (user, bundle) = build_session_artifacts();
        let validation = SessionValidation {
            user: user.clone(),
            bundle: None,
            rotated: false,
        };

        stub.enqueue_auth(Ok((user.clone(), bundle.clone())));
        stub.enqueue_validate(Ok(Some(validation)));

        let session_manager: Arc<dyn SessionManager> = stub.clone();
        let state = server::create_app_state(None, None, None, Some(session_manager), None, None);

        let csrf_state = csrf::CsrfState::from_config(&config);

        let protected = Router::new()
            .route("/api/messages", post(messages_handler))
            .layer(axum::middleware::from_fn_with_state(
                csrf_state.clone(),
                csrf::enforce_csrf,
            ))
            .route_layer(axum::middleware::from_fn(auth_middleware));

        let app = Router::new()
            .route("/api/auth/login", post(login))
            .merge(protected)
            .layer(Extension(config.clone()))
            .layer(Extension(state));

        let server = TestServer::new(app).expect("test server");
        let login_response = server
            .post("/api/auth/login")
            .json(&LoginRequest {
                email: "integration@example.com".into(),
                password: "secret".into(),
            })
            .await;

        assert_eq!(login_response.status_code(), StatusCode::OK);

        let cookies = login_response.cookies();
        let body: LoginResponse = login_response.json();

        let session_cookie = cookies
            .iter()
            .find(|cookie| cookie.name() == "SESSION_ID")
            .expect("session cookie")
            .value()
            .to_string();
        let csrf_cookie = cookies
            .iter()
            .find(|cookie| cookie.name() == "CSRF-TOKEN")
            .expect("csrf cookie")
            .value()
            .to_string();
        let cookie_header = format!("SESSION_ID={}; CSRF-TOKEN={}", session_cookie, csrf_cookie);

        let response = server
            .post("/api/messages")
            .add_header(header::COOKIE, cookie_header)
            .add_header(
                config.security.csrf.header_name.to_string(),
                body.csrf_token.clone(),
            )
            .json(&json!({ "message": "hello" }))
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn mutation_without_csrf_is_rejected() {
        let config = test_config();
        let stub = Arc::new(StubSessionManager::default());
        let (user, bundle) = build_session_artifacts();
        let validation = SessionValidation {
            user: user.clone(),
            bundle: None,
            rotated: false,
        };

        stub.enqueue_auth(Ok((user.clone(), bundle.clone())));
        stub.enqueue_validate(Ok(Some(validation)));

        let session_manager: Arc<dyn SessionManager> = stub.clone();
        let state = server::create_app_state(None, None, None, Some(session_manager), None, None);

        let csrf_state = csrf::CsrfState::from_config(&config);

        let protected = Router::new()
            .route("/api/messages", post(messages_handler))
            .layer(axum::middleware::from_fn_with_state(
                csrf_state,
                csrf::enforce_csrf,
            ))
            .route_layer(axum::middleware::from_fn(auth_middleware));

        let app = Router::new()
            .route("/api/auth/login", post(login))
            .merge(protected)
            .layer(Extension(config.clone()))
            .layer(Extension(state));

        let server = TestServer::new(app).expect("test server");

        let login_response = server
            .post("/api/auth/login")
            .json(&LoginRequest {
                email: "integration@example.com".into(),
                password: "secret".into(),
            })
            .await;

        let cookies = login_response.cookies();
        let session_cookie = cookies
            .iter()
            .find(|cookie| cookie.name() == "SESSION_ID")
            .expect("session cookie")
            .value()
            .to_string();

        let response = server
            .post("/api/messages")
            .add_header(
                header::COOKIE,
                format!("SESSION_ID={}; CSRF-TOKEN=csrf-token", session_cookie),
            )
            .json(&json!({ "message": "hello" }))
            .await;

        assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
        let payload: serde_json::Value = response.json();
        assert_eq!(payload["code"], "RGP.AUTH.CSRF");
    }
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
            "RGP.AUTH.MISSING_SESSION",
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
                "RGP.AUTH.INVALID_SESSION",
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
            "RGP.AUTH.MISSING_SESSION",
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
                "RGP.AUTH.INVALID_SESSION",
                "session expired",
            )
        })?;

    service
        .revoke_session_by_id(validation.user.session_id, Some("logout"))
        .await
        .map_err(map_session_error)?;

    let mut response = Json(json!({ "ok": true })).into_response();
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
    *response.status_mut() = StatusCode::OK;

    Ok(response)
}
