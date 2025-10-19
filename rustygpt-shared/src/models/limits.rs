use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::Timestamp;

/// A rate-limit profile describing algorithm parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct RateLimitProfile {
    pub id: Uuid,
    pub name: String,
    pub algorithm: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Request payload to create a new rate-limit profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct CreateRateLimitProfileRequest {
    pub name: String,
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_algorithm() -> String {
    "gcra".to_string()
}

/// Request payload to update an existing rate-limit profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct UpdateRateLimitProfileRequest {
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// An assignment mapping an HTTP route to a rate-limit profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct RateLimitAssignment {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub profile_name: String,
    pub method: String,
    pub path_pattern: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Request payload to attach a profile to an HTTP route.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct AssignRateLimitRequest {
    pub profile_id: Uuid,
    pub method: String,
    pub path: String,
}
