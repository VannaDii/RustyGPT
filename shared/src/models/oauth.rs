use serde::{Deserialize, Serialize};

/// Represents a request to authenticate using OAuth.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OAuthRequest {
    /// The authorization code received from the OAuth provider.
    pub auth_code: String,
}

/// Query parameters for the OAuth callback
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OAuthCallback {
    /// The authorization code received from the OAuth provider
    pub code: String,
    /// Optional state parameter for CSRF protection
    pub state: Option<String>,
}

/// Response for the OAuth initialization
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OAuthInitResponse {
    /// The authorization URL to redirect the user to
    pub auth_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_oauth_request_serialization() {
        let request = OAuthRequest {
            auth_code: "test_auth_code".to_string(),
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let expected = r#"{"auth_code":"test_auth_code"}"#;

        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_oauth_request_deserialization() {
        let json = r#"{"auth_code":"test_auth_code"}"#;
        let deserialized: OAuthRequest = serde_json::from_str(json).unwrap();

        assert_eq!(deserialized.auth_code, "test_auth_code");
    }

    #[test]
    fn test_oauth_callback_serialization() {
        let callback = OAuthCallback {
            code: "test_code".to_string(),
            state: Some("test_state".to_string()),
        };

        let serialized = serde_json::to_string(&callback).unwrap();
        let expected = r#"{"code":"test_code","state":"test_state"}"#;

        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_oauth_callback_deserialization() {
        let json = r#"{"code":"test_code","state":"test_state"}"#;
        let deserialized: OAuthCallback = serde_json::from_str(json).unwrap();

        assert_eq!(deserialized.code, "test_code");
        assert_eq!(deserialized.state, Some("test_state".to_string()));
    }

    #[test]
    fn test_oauth_callback_without_state() {
        let callback = OAuthCallback {
            code: "test_code".to_string(),
            state: None,
        };

        let serialized = serde_json::to_string(&callback).unwrap();
        let expected = r#"{"code":"test_code","state":null}"#;

        assert_eq!(serialized, expected);

        let deserialized: OAuthCallback = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.state, None);
    }

    #[test]
    fn test_oauth_init_response_serialization() {
        let response = OAuthInitResponse {
            auth_url: "https://example.com/auth".to_string(),
        };

        let serialized = serde_json::to_string(&response).unwrap();
        let expected = r#"{"auth_url":"https://example.com/auth"}"#;

        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_oauth_init_response_deserialization() {
        let json = r#"{"auth_url":"https://example.com/auth"}"#;
        let deserialized: OAuthInitResponse = serde_json::from_str(json).unwrap();

        assert_eq!(deserialized.auth_url, "https://example.com/auth");
    }
}
