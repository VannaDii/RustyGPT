use std::str::FromStr;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{
    auth::session::SessionUser,
    http::error::{ApiError, AppResult},
};
use shared::config::server::Config;

#[derive(Clone, Debug, Default)]
pub struct RequestContext {
    pub request_id: String,
    pub session: Option<SessionUser>,
}

impl RequestContext {
    pub fn user_id(&self) -> Option<Uuid> {
        self.session.as_ref().map(|session| session.id)
    }
}

#[derive(Clone)]
pub struct RequestIdState {
    header: HeaderName,
}

impl RequestIdState {
    pub fn from_config(config: &Config) -> Self {
        let header = HeaderName::from_str(&config.server.request_id_header)
            .unwrap_or_else(|_| HeaderName::from_static("x-request-id"));
        Self { header }
    }
}

pub async fn assign_request_id(
    State(state): State<RequestIdState>,
    mut request: Request<Body>,
    next: Next,
) -> AppResult<Response> {
    let header_name = state.header.clone();
    let current = extract_request_id(request.headers(), &header_name);

    let request_id = current.unwrap_or_else(|| Uuid::new_v4().to_string());

    request.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
        session: None,
    });

    request.headers_mut().insert(
        header_name.clone(),
        HeaderValue::from_str(&request_id)
            .map_err(|_| ApiError::internal_server_error("failed to encode request id"))?,
    );

    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header_name,
        HeaderValue::from_str(&request_id)
            .map_err(|_| ApiError::internal_server_error("failed to encode request id"))?,
    );

    Ok(response)
}

fn extract_request_id(headers: &HeaderMap, header: &HeaderName) -> Option<String> {
    headers
        .get(header)
        .and_then(|value| value.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
