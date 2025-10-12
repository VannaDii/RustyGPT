use axum::{http::StatusCode, response::IntoResponse};
use serde_json::json;
use thiserror::Error;

use super::problem::ProblemDetails;

pub type AppResult<T> = Result<T, ApiError>;

#[derive(Debug, Error)]
#[error("{code}: {message}")]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limit_exceeded",
            message,
        )
    }

    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let details = self.details;

        let mut problem = ProblemDetails::new(self.status, self.code, self.message);
        if let Some(details) = details {
            problem = problem.with_details(details);
        }

        problem.into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self::internal_server_error(value.to_string())
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_err) = &err {
            let code = db_err
                .code()
                .unwrap_or_else(|| std::borrow::Cow::Borrowed("unknown"));
            let message = format!("database error {code}");
            return Self::internal_server_error(message)
                .with_details(json!({ "sqlstate": code, "message": db_err.message() }));
        }

        Self::internal_server_error(err.to_string())
    }
}

impl From<http::Error> for ApiError {
    fn from(err: http::Error) -> Self {
        Self::internal_server_error(err.to_string())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::not_found(err.to_string()),
            std::io::ErrorKind::PermissionDenied => Self::forbidden(err.to_string()),
            _ => Self::internal_server_error(err.to_string()),
        }
    }
}
