use serde::{Deserialize, Serialize};

/// Represents a request for first-time setup.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SetupRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// Represents a response for first-time setup check.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SetupResponse {
    pub is_setup: bool,
}
