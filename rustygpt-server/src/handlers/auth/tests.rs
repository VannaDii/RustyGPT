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

type RefreshResult = Result<Option<(SessionUser, SessionBundle)>, SessionError>;

#[derive(Default)]
struct StubSessionManager {
    authenticate: Mutex<VecDeque<Result<(SessionUser, SessionBundle), SessionError>>>,
    validate: Mutex<VecDeque<Result<Option<SessionValidation>, SessionError>>>,
    refresh: Mutex<VecDeque<RefreshResult>>,
}

impl StubSessionManager {
    fn enqueue_auth(&self, response: Result<(SessionUser, SessionBundle), SessionError>) {
        self.authenticate.lock().unwrap().push_back(response);
    }

    fn enqueue_validate(&self, response: Result<Option<SessionValidation>, SessionError>) {
        self.validate.lock().unwrap().push_back(response);
    }

    fn enqueue_refresh(&self, response: RefreshResult) {
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

    async fn refresh_session(&self, _token: &str, _metadata: &SessionMetadata) -> RefreshResult {
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
    let cookie_header = format!("SESSION_ID={session_cookie}; CSRF-TOKEN={csrf_cookie}");

    let response = server
        .post("/api/messages")
        .add_header(header::COOKIE, cookie_header)
        .add_header(
            config.security.csrf.header_name.clone(),
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
            format!("SESSION_ID={session_cookie}; CSRF-TOKEN=csrf-token"),
        )
        .json(&json!({ "message": "hello" }))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    let payload: serde_json::Value = response.json();
    assert_eq!(payload["code"], "RGP.AUTH.CSRF");
}
