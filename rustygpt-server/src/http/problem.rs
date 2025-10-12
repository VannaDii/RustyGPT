use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use http::header::{CONTENT_TYPE, HeaderValue};
use serde::Serialize;
use serde_json::Value;

/// RFC 7807 compliant error response body used throughout the API.
#[derive(Debug, Serialize)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub title: String,
    pub status: u16,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl ProblemDetails {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        let title = status.canonical_reason().unwrap_or("Error").to_string();
        Self {
            problem_type: format!("https://rustygpt.dev/problems/{code}"),
            title,
            status: status.as_u16(),
            code: code.to_string(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response {
        let mut response = axum::Json(self).into_response();
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response.headers_mut().insert(
            http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        );
        response
    }
}
