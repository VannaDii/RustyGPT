use crate::http::error::AppResult;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderValue, Request, header},
    middleware::Next,
    response::Response,
};
use shared::config::server::Config;

#[derive(Clone)]
pub struct SecurityHeadersState {
    header_value: HeaderValue,
    include_hsts: bool,
    content_security_policy: HeaderValue,
}

impl SecurityHeadersState {
    pub fn from_config(config: &Config) -> Self {
        let hsts_enabled = config.security.hsts.enabled;
        let hsts_value = if hsts_enabled {
            let mut directives = vec![format!("max-age={}", config.security.hsts.max_age_seconds)];
            if config.security.hsts.include_subdomains {
                directives.push("includeSubDomains".into());
            }
            if config.security.hsts.preload {
                directives.push("preload".into());
            }
            HeaderValue::from_str(&directives.join("; "))
                .unwrap_or_else(|_| HeaderValue::from_static("max-age=63072000"))
        } else {
            HeaderValue::from_static("")
        };

        let csp = HeaderValue::from_static(
            "default-src 'self'; frame-ancestors 'none'; object-src 'none'; base-uri 'self'",
        );

        Self {
            header_value: hsts_value,
            include_hsts: hsts_enabled,
            content_security_policy: csp,
        }
    }
}

pub async fn apply_security_headers(
    State(state): State<SecurityHeadersState>,
    request: Request<Body>,
    next: Next,
) -> AppResult<Response> {
    let mut response = next.run(request).await;

    if state.include_hsts {
        response.headers_mut().insert(
            header::STRICT_TRANSPORT_SECURITY,
            state.header_value.clone(),
        );
    }

    response
        .headers_mut()
        .entry(header::X_CONTENT_TYPE_OPTIONS)
        .or_insert_with(|| HeaderValue::from_static("nosniff"));

    response
        .headers_mut()
        .entry(header::CONTENT_SECURITY_POLICY)
        .or_insert_with(|| state.content_security_policy.clone());

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::{Router, routing::get};
    use shared::config::server::Profile;
    use tower::ServiceExt;

    #[test]
    fn from_config_builds_expected_hsts_directive() {
        let mut config = Config::default_for_profile(Profile::Prod);
        config.security.hsts.enabled = true;
        config.security.hsts.max_age_seconds = 123;
        config.security.hsts.include_subdomains = true;
        config.security.hsts.preload = true;

        let state = SecurityHeadersState::from_config(&config);
        assert!(state.include_hsts);
        assert_eq!(
            state.header_value.to_str().unwrap(),
            "max-age=123; includeSubDomains; preload"
        );
    }

    #[tokio::test]
    async fn middleware_adds_security_headers_when_enabled() {
        let config = Config::default_for_profile(Profile::Prod);
        let state = SecurityHeadersState::from_config(&config);
        let app = Router::new()
            .route("/", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                apply_security_headers,
            ))
            .with_state(state.clone());

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(
            response
                .headers()
                .get(header::STRICT_TRANSPORT_SECURITY)
                .and_then(|value| value.to_str().ok()),
            Some(state.header_value.to_str().unwrap())
        );
        assert_eq!(
            response
                .headers()
                .get(header::X_CONTENT_TYPE_OPTIONS)
                .and_then(|value| value.to_str().ok()),
            Some("nosniff")
        );
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_SECURITY_POLICY)
                .and_then(|value| value.to_str().ok()),
            Some(state.content_security_policy.to_str().unwrap())
        );
    }

    #[tokio::test]
    async fn hsts_header_omitted_when_disabled() {
        let mut config = Config::default_for_profile(Profile::Prod);
        config.security.hsts.enabled = false;
        let state = SecurityHeadersState::from_config(&config);
        let app = Router::new()
            .route("/", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                apply_security_headers,
            ))
            .with_state(state.clone());

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(
            !response
                .headers()
                .contains_key(header::STRICT_TRANSPORT_SECURITY)
        );
    }
}
