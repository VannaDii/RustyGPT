use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Represents an error response.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
    pub details: Option<String>,
}
