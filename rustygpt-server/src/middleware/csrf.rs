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

fn is_method_idempotent(method: &Method) -> bool {
    matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS)
}

fn is_stream_endpoint(path: &str) -> bool {
    path.starts_with("/api/stream")
}

fn extract_cookie(request: &Request<Body>, cookie_name: &str) -> Option<String> {
    let cookie_header = request.headers().get(header::COOKIE)?.to_str().ok()?;

    Cookie::split_parse(cookie_header)
        .flatten()
        .find(|cookie| cookie.name() == cookie_name)
        .map(|cookie| cookie.value().to_string())
}
