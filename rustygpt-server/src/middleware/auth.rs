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
    auth::session::{SessionError, SessionMetadata, SessionValidation},
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
    let token = match extract_session_cookie(req.headers(), session_cookie_name) {
        Some(token) => token,
        None => return Ok(unauthorized_response()),
    };

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

    let fingerprint = req
        .headers()
        .get("x-client-fingerprint")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let metadata = SessionMetadata::default()
        .with_user_agent(user_agent)
        .with_ip_str(forwarded_ip.or(real_ip))
        .with_fingerprint(fingerprint);

    let validation = match session_service.validate_session(&token, &metadata).await {
        Ok(Some(value)) => value,
        Ok(None) => return Ok(unauthorized_response()),
        Err(err) => {
            warn!(error = %err, "session validation failed");
            return Ok(map_session_error(err));
        }
    };

    let SessionValidation {
        user,
        bundle,
        rotated,
    } = validation;

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
    let rotation_header_set = bundle.as_ref().map_or(false, |bundle| {
        if let Ok(value) = http::HeaderValue::from_str(&bundle.session_cookie.to_string()) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
        if let Ok(value) = http::HeaderValue::from_str(&bundle.csrf_cookie.to_string()) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
        response.headers_mut().insert(
            header::HeaderName::from_static("x-session-rotated"),
            http::HeaderValue::from_static("1"),
        );
        true
    });

    if rotated && !rotation_header_set {
        response.headers_mut().insert(
            header::HeaderName::from_static("x-session-rotated"),
            http::HeaderValue::from_static("1"),
        );
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

fn unauthorized_response() -> Response {
    http::Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::WWW_AUTHENTICATE, "session")
        .body(Body::empty())
        .unwrap()
        .into()
}

fn conflict_response() -> Response {
    http::Response::builder()
        .status(StatusCode::CONFLICT)
        .body(Body::empty())
        .unwrap()
        .into()
}

fn locked_response() -> Response {
    http::Response::builder()
        .status(StatusCode::LOCKED)
        .body(Body::empty())
        .unwrap()
        .into()
}

fn map_session_error(error: SessionError) -> Response {
    match error {
        SessionError::DisabledUser => locked_response(),
        SessionError::RotationRequired => conflict_response(),
        SessionError::SessionExpired
        | SessionError::AbsoluteExpired
        | SessionError::InvalidCredentials => unauthorized_response(),
        other => {
            warn!(error = %other, "unexpected session error");
            unauthorized_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_response_sets_header() {
        let response = unauthorized_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response
                .headers()
                .get(header::WWW_AUTHENTICATE)
                .map(|value| value.to_str().unwrap()),
            Some("session")
        );
    }

    #[test]
    fn map_session_error_handles_disabled_user() {
        let response = map_session_error(SessionError::DisabledUser);
        assert_eq!(response.status(), StatusCode::LOCKED);
    }

    #[test]
    fn map_session_error_handles_expired() {
        let response = map_session_error(SessionError::SessionExpired);
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
