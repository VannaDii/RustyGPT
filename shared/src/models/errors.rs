use serde::{Deserialize, Serialize};

/// Represents a response for first-time setup check.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ErrorResponse {
    pub message: String,
    pub details: Option<String>,
}
