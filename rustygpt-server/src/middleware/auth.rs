use axum::{
    body::Body,
    extract::Request,
    http::{self, HeaderName, header},
    middleware::Next,
    response::Response,
};
use cookie::Cookie;
use http::StatusCode;
use shared::config::server::Config;
use std::{str::FromStr, sync::Arc};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::middleware::request_context::RequestContext;

// Middleware to check if a user is authenticated
#[instrument(skip(next))]
pub async fn auth_middleware(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let config = req.extensions().get::<Arc<Config>>().cloned();

    if let Some(config) = config {
        if config.features.auth_v1 {
            let session_cookie_name = &config.session.session_cookie_name;
            let session_id = extract_session_cookie(req.headers(), session_cookie_name)
                .ok_or(StatusCode::UNAUTHORIZED)?;

            let user_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, session_id.as_bytes());

            let request_header = HeaderName::from_str(&config.server.request_id_header)
                .unwrap_or_else(|_| HeaderName::from_static("x-request-id"));
            let request_id = req
                .headers()
                .get(&request_header)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();

            if let Some(context) = req.extensions_mut().get_mut::<RequestContext>() {
                context.user_id = Some(user_id);
            } else {
                req.extensions_mut().insert(RequestContext {
                    request_id,
                    user_id: Some(user_id),
                });
            }
        }
    }

    info!(
        "Auth middleware processing request to: {}",
        req.uri().path()
    );
    Ok(next.run(req).await)
}

fn extract_session_cookie(headers: &http::HeaderMap, name: &str) -> Option<String> {
    let value = headers.get(header::COOKIE)?.to_str().ok()?;
    Cookie::split_parse(value)
        .flatten()
        .find(|cookie| cookie.name() == name)
        .map(|cookie| cookie.value().to_string())
}
