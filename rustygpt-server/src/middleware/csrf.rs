use axum::{
    body::Body,
    extract::State,
    http::{HeaderName, Method, Request, header},
    middleware::Next,
    response::Response,
};
use cookie::Cookie;

use crate::http::error::{ApiError, AppResult};
use shared::config::server::Config;

#[derive(Clone)]
pub struct CsrfState {
    enabled: bool,
    header_name: HeaderName,
    cookie_name: String,
}

impl CsrfState {
    pub fn from_config(config: &Config) -> Self {
        let header_name = HeaderName::from_bytes(config.security.csrf.header_name.as_bytes())
            .unwrap_or(header::HeaderName::from_static("x-csrf-token"));

        Self {
            enabled: config.security.csrf.enabled,
            header_name,
            cookie_name: config.security.csrf.cookie_name.clone(),
        }
    }
}

pub async fn enforce_csrf(
    State(state): State<CsrfState>,
    request: Request<Body>,
    next: Next,
) -> AppResult<Response> {
    if !state.enabled {
        return Ok(next.run(request).await);
    }

    if is_method_idempotent(request.method()) {
        return Ok(next.run(request).await);
    }

    if is_stream_endpoint(request.uri().path()) {
        return Ok(next.run(request).await);
    }

    if is_auth_endpoint(request.uri().path()) {
        return Ok(next.run(request).await);
    }

    let header_token = request
        .headers()
        .get(&state.header_name)
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string());

    let cookie_token = extract_cookie(&request, &state.cookie_name);

    match (header_token, cookie_token) {
        (Some(header), Some(cookie)) if header == cookie => Ok(next.run(request).await),
        (None, _) => Err(ApiError::forbidden("missing CSRF header token")),
        (_, None) => Err(ApiError::forbidden("missing CSRF cookie token")),
        (Some(_), Some(_)) => Err(ApiError::forbidden("CSRF token mismatch")),
    }
}

const fn is_method_idempotent(method: &Method) -> bool {
    matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS)
}

fn is_stream_endpoint(path: &str) -> bool {
    path.starts_with("/api/stream")
}

fn is_auth_endpoint(path: &str) -> bool {
    path.starts_with("/api/auth/")
}

fn extract_cookie(request: &Request<Body>, cookie_name: &str) -> Option<String> {
    let cookie_header = request.headers().get(header::COOKIE)?.to_str().ok()?;

    Cookie::split_parse(cookie_header)
        .flatten()
        .find(|cookie| cookie.name() == cookie_name)
        .map(|cookie| cookie.value().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        routing::{get, post},
    };
    use shared::config::server::Profile;
    use tower::ServiceExt;

    fn csrf_state(enabled: bool) -> CsrfState {
        let mut config = Config::default_for_profile(Profile::Test);
        config.security.csrf.enabled = enabled;
        CsrfState::from_config(&config)
    }

    async fn call(state: CsrfState, request: Request<Body>) -> Response {
        async fn ok_handler() -> Response {
            Response::new(Body::empty())
        }

        let router = Router::new()
            .route("/api/messages", get(ok_handler).post(ok_handler))
            .route("/api/conversations", get(ok_handler))
            .route("/api/auth/login", post(ok_handler))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                enforce_csrf,
            ))
            .with_state(state);

        router.oneshot(request).await.unwrap()
    }

    #[tokio::test]
    async fn allows_idempotent_methods_without_tokens() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/conversations")
            .body(Body::empty())
            .unwrap();

        let response = call(csrf_state(true), request).await;
        assert_eq!(response.status(), http::StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_missing_header_token() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/messages")
            .body(Body::empty())
            .unwrap();

        let response = call(csrf_state(true), request).await;
        assert_eq!(response.status(), http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rejects_mismatched_tokens() {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/messages")
            .header("X-CSRF-Token", "header-token")
            .body(Body::empty())
            .unwrap();
        request.headers_mut().insert(
            header::COOKIE,
            header::HeaderValue::from_static("CSRF-TOKEN=cookie-token"),
        );

        let response = call(csrf_state(true), request).await;
        assert_eq!(response.status(), http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn accepts_matching_tokens() {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/messages")
            .header("X-CSRF-Token", "token")
            .body(Body::empty())
            .unwrap();
        request.headers_mut().insert(
            header::COOKIE,
            header::HeaderValue::from_static("CSRF-TOKEN=token"),
        );

        let response = call(csrf_state(true), request).await;
        assert_eq!(response.status(), http::StatusCode::OK);
    }

    #[tokio::test]
    async fn bypasses_auth_endpoints() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/login")
            .body(Body::empty())
            .unwrap();

        let response = call(csrf_state(true), request).await;
        assert_eq!(response.status(), http::StatusCode::OK);
    }
}
