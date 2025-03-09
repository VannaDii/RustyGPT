use serde::{Deserialize, Serialize};

/// Represents a request to authenticate using OAuth.
#[derive(Debug, Deserialize, Serialize)]
pub struct OAuthRequest {
    /// The authorization code received from the OAuth provider.
    pub auth_code: String,
}

/// Query parameters for the OAuth callback
#[derive(Debug, Deserialize, Serialize)]
pub struct OAuthCallback {
    /// The authorization code received from the OAuth provider
    pub code: String,
    /// Optional state parameter for CSRF protection
    pub state: Option<String>,
}

/// Response for the OAuth initialization
#[derive(Debug, Deserialize, Serialize)]
pub struct OAuthInitResponse {
    /// The authorization URL to redirect the user to
    pub auth_url: String,
}
