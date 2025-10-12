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
