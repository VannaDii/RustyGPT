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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, extract::Extension, routing::get};
    use shared::config::server::Profile;
    use tower::ServiceExt;

    #[test]
    fn user_id_proxies_session_identifier() {
        let context = RequestContext {
            request_id: "req".into(),
            session: Some(SessionUser {
                id: Uuid::new_v4(),
                email: "user@example.com".into(),
                username: "user".into(),
                display_name: None,
                roles: vec![],
                session_id: Uuid::new_v4(),
                issued_at: chrono::Utc::now(),
                expires_at: chrono::Utc::now(),
                absolute_expires_at: chrono::Utc::now(),
            }),
        };

        assert!(context.user_id().is_some());
        assert!(
            RequestContext {
                request_id: "noop".into(),
                session: None
            }
            .user_id()
            .is_none()
        );
    }

    #[test]
    fn request_id_state_uses_configured_header_or_fallback() {
        let mut config = Config::default_for_profile(Profile::Dev);
        config.server.request_id_header = "x-custom-request".into();
        let state = RequestIdState::from_config(&config);
        assert_eq!(state.header, HeaderName::from_static("x-custom-request"));

        config.server.request_id_header = "invalid header value".into();
        let fallback = RequestIdState::from_config(&config);
        assert_eq!(fallback.header, HeaderName::from_static("x-request-id"));
    }

    #[test]
    fn extract_request_id_trims_and_filters_values() {
        let mut headers = HeaderMap::new();
        let header = HeaderName::from_static("x-request-id");
        assert!(extract_request_id(&headers, &header).is_none());

        headers.insert(&header, HeaderValue::from_static("   "));
        assert!(extract_request_id(&headers, &header).is_none());

        headers.insert(&header, HeaderValue::from_static(" 12345 "));
        assert_eq!(
            extract_request_id(&headers, &header).as_deref(),
            Some("12345")
        );
    }

    async fn request_id_handler(Extension(context): Extension<RequestContext>) -> String {
        context.request_id
    }

    #[tokio::test]
    async fn assign_request_id_generates_and_propagates_new_id() {
        let state = RequestIdState::from_config(&Config::default_for_profile(Profile::Test));
        let app = Router::new()
            .route("/", get(request_id_handler))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                assign_request_id,
            ))
            .with_state(state.clone());

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let headers = response.headers().clone();
        let id = headers
            .get(&state.header)
            .and_then(|value| value.to_str().ok())
            .unwrap()
            .to_string();
        assert!(!id.is_empty());

        let (_, body) = response.into_parts();
        let body = axum::body::to_bytes(body, 1024).await.unwrap();
        assert_eq!(std::str::from_utf8(&body).unwrap(), id);
    }

    #[tokio::test]
    async fn assign_request_id_respects_existing_header() {
        let state = RequestIdState::from_config(&Config::default_for_profile(Profile::Dev));
        let existing_id = "existing-12345";
        let app = Router::new()
            .route("/", get(request_id_handler))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                assign_request_id,
            ))
            .with_state(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(&state.header, format!("  {existing_id}  "))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let header_value = response
            .headers()
            .get(&state.header)
            .and_then(|value| value.to_str().ok())
            .unwrap();
        assert_eq!(header_value, existing_id);
    }
}
