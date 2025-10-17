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
use tracing::{instrument, warn};

use crate::{
    app_state::AppState,
    auth::session::{SessionMetadata, SessionValidation},
    middleware::request_context::RequestContext,
};

// Middleware to check if a user is authenticated
#[instrument(skip(next, req))]
pub async fn auth_middleware(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let config = match req.extensions().get::<Arc<Config>>().cloned() {
        Some(config) => config,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    if !config.features.auth_v1 {
        return Ok(next.run(req).await);
    }

    let state = req
        .extensions()
        .get::<Arc<AppState>>()
        .cloned()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let session_service = state
        .sessions
        .clone()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_cookie_name = &config.session.session_cookie_name;
    let token = extract_session_cookie(req.headers(), session_cookie_name)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let forwarded_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string());

    let real_ip = req
        .headers()
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let metadata = SessionMetadata::default()
        .with_user_agent(user_agent)
        .with_ip_str(forwarded_ip.or(real_ip));

    let validation = session_service
        .validate_session(&token, &metadata)
        .await
        .map_err(|err| {
            warn!(error = %err, "session validation failed");
            StatusCode::UNAUTHORIZED
        })?;

    let SessionValidation {
        user,
        refresh_cookie,
    } = validation.ok_or(StatusCode::UNAUTHORIZED)?;

    let request_id = req
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.request_id.clone())
        .or_else(|| {
            let header = HeaderName::from_str(&config.server.request_id_header)
                .unwrap_or_else(|_| HeaderName::from_static("x-request-id"));
            req.headers()
                .get(&header)
                .and_then(|value| value.to_str().ok())
                .map(|value| value.to_string())
        })
        .unwrap_or_default();

    if let Some(context) = req.extensions_mut().get_mut::<RequestContext>() {
        context.session = Some(user.clone());
    } else {
        req.extensions_mut().insert(RequestContext {
            request_id,
            session: Some(user.clone()),
        });
    }

    let mut response = next.run(req).await;
    if let Some(cookie) = refresh_cookie {
        if let Ok(value) = http::HeaderValue::from_str(&cookie.to_string()) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }

    Ok(response)
}

fn extract_session_cookie(headers: &http::HeaderMap, name: &str) -> Option<String> {
    let value = headers.get(header::COOKIE)?.to_str().ok()?;
    Cookie::split_parse(value)
        .flatten()
        .find(|cookie| cookie.name() == name)
        .map(|cookie| cookie.value().to_string())
}
