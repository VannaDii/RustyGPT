use axum::{http::StatusCode, response::IntoResponse};
use serde_json::json;
use thiserror::Error;

use super::problem::ProblemDetails;
use crate::services::chat_service::ChatServiceError;

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

impl From<ChatServiceError> for ApiError {
    fn from(err: ChatServiceError) -> Self {
        match err {
            ChatServiceError::Validation(message) => {
                Self::new(StatusCode::BAD_REQUEST, "validation_failed", message)
            }
            ChatServiceError::NotFound(message) => Self::not_found(message),
            ChatServiceError::Forbidden(message) => Self::forbidden(message),
            ChatServiceError::RateLimited(message) => Self::too_many_requests(message),
            ChatServiceError::Database(db_err) => Self::from(db_err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use http::header::CONTENT_TYPE;
    use serde_json::Value;

    #[test]
    fn new_sets_fields_and_allows_details() {
        let error = ApiError::forbidden("nope").with_details(json!({ "reason": "policy" }));
        assert_eq!(error.status, StatusCode::FORBIDDEN);
        assert_eq!(error.code, "forbidden");
        assert!(
            error
                .details
                .as_ref()
                .is_some_and(|details| details["reason"] == Value::from("policy"))
        );
    }

    #[tokio::test]
    async fn into_response_serializes_problem_details() {
        let response = ApiError::not_found("missing resource")
            .with_details(json!({ "resource": "thing" }))
            .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "application/problem+json"
        );

        let bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .expect("body to bytes");
        let json: Value =
            serde_json::from_slice(&bytes).expect("problem details deserializes to json");
        assert_eq!(json["code"], "not_found");
        assert_eq!(json["message"], "missing resource");
        assert_eq!(json["details"]["resource"], "thing");
    }

    #[test]
    fn io_errors_map_to_expected_status_codes() {
        let not_found =
            ApiError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
        assert_eq!(not_found.status, StatusCode::NOT_FOUND);

        let forbidden = ApiError::from(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

        let other = ApiError::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            "something else",
        ));
        assert_eq!(other.status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn chat_service_errors_map_to_matching_status_codes() {
        let validation = ApiError::from(ChatServiceError::Validation("bad".into()));
        assert_eq!(validation.status, StatusCode::BAD_REQUEST);

        let not_found = ApiError::from(ChatServiceError::NotFound("missing".into()));
        assert_eq!(not_found.status, StatusCode::NOT_FOUND);

        let forbidden = ApiError::from(ChatServiceError::Forbidden("nope".into()));
        assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

        let limited = ApiError::from(ChatServiceError::RateLimited("slow down".into()));
        assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);

        let db = ApiError::from(ChatServiceError::Database(sqlx::Error::PoolTimedOut));
        assert_eq!(db.status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
