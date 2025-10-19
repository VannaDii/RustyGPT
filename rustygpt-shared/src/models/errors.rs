use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Represents an error response.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub struct ErrorResponse {
    /// The main error message
    pub message: String,
    /// Optional additional details about the error
    pub details: Option<String>,
}

impl ErrorResponse {
    /// Creates a new error response with just a message.
    ///
    /// # Arguments
    /// * `message` - The error message
    ///
    /// # Returns
    /// A new [`ErrorResponse`] with the provided message and no details.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            details: None,
        }
    }

    /// Creates a new error response with message and details.
    ///
    /// # Arguments
    /// * `message` - The error message
    /// * `details` - Additional error details
    ///
    /// # Returns
    /// A new [`ErrorResponse`] with the provided message and details.
    pub fn with_details(message: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            details: Some(details.into()),
        }
    }

    /// Checks if this error response has details.
    ///
    /// # Returns
    /// `true` if details are present, `false` otherwise.
    pub const fn has_details(&self) -> bool {
        self.details.is_some()
    }
}

impl std::fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.details {
            Some(details) => write!(f, "{}: {}", self.message, details),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ErrorResponse {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test ErrorResponse creation with new()
    #[test]
    fn test_error_response_new() {
        let error = ErrorResponse::new("Test error");
        assert_eq!(error.message, "Test error");
        assert_eq!(error.details, None);
        assert!(!error.has_details());
    }

    /// Test ErrorResponse creation with with_details()
    #[test]
    fn test_error_response_with_details() {
        let error = ErrorResponse::with_details("Test error", "Additional details");
        assert_eq!(error.message, "Test error");
        assert_eq!(error.details, Some("Additional details".to_string()));
        assert!(error.has_details());
    }

    /// Test ErrorResponse creation from String
    #[test]
    fn test_error_response_from_string() {
        let message = "Dynamic error".to_string();
        let details = "Dynamic details".to_string();
        let error = ErrorResponse::with_details(message, details);
        assert_eq!(error.message, "Dynamic error");
        assert_eq!(error.details, Some("Dynamic details".to_string()));
    }

    /// Test ErrorResponse equality
    #[test]
    fn test_error_response_equality() {
        let error1 = ErrorResponse::new("Same message");
        let error2 = ErrorResponse::new("Same message");
        let error3 = ErrorResponse::new("Different message");

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    /// Test ErrorResponse with details equality
    #[test]
    fn test_error_response_with_details_equality() {
        let error1 = ErrorResponse::with_details("Message", "Details");
        let error2 = ErrorResponse::with_details("Message", "Details");
        let error3 = ErrorResponse::with_details("Message", "Different details");

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    /// Test ErrorResponse serialization
    #[test]
    fn test_error_response_serialization() {
        let error = ErrorResponse::new("Test error");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Test error"));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"details\":null"));
    }

    /// Test ErrorResponse with details serialization
    #[test]
    fn test_error_response_with_details_serialization() {
        let error = ErrorResponse::with_details("Test error", "Error details");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Test error"));
        assert!(json.contains("Error details"));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"details\""));
    }

    /// Test ErrorResponse deserialization
    #[test]
    fn test_error_response_deserialization() {
        let json = r#"{"message":"Test error","details":null}"#;
        let error: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(error.message, "Test error");
        assert_eq!(error.details, None);
    }

    /// Test ErrorResponse with details deserialization
    #[test]
    fn test_error_response_with_details_deserialization() {
        let json = r#"{"message":"Test error","details":"Error details"}"#;
        let error: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(error.message, "Test error");
        assert_eq!(error.details, Some("Error details".to_string()));
    }

    /// Test ErrorResponse Display implementation
    #[test]
    fn test_error_response_display() {
        let error_no_details = ErrorResponse::new("Simple error");
        assert_eq!(format!("{}", error_no_details), "Simple error");

        let error_with_details = ErrorResponse::with_details("Main error", "Additional info");
        assert_eq!(
            format!("{}", error_with_details),
            "Main error: Additional info"
        );
    }

    /// Test ErrorResponse Debug implementation
    #[test]
    fn test_error_response_debug() {
        let error = ErrorResponse::new("Debug test");
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ErrorResponse"));
        assert!(debug_str.contains("Debug test"));
    }

    /// Test ErrorResponse as Error trait
    #[test]
    fn test_error_response_as_error() {
        let error = ErrorResponse::new("Error trait test");
        let error_trait: &dyn std::error::Error = &error;
        assert!(error_trait.to_string().contains("Error trait test"));
    }

    /// Test has_details method
    #[test]
    fn test_has_details() {
        let error_no_details = ErrorResponse::new("No details");
        let error_with_details = ErrorResponse::with_details("Has details", "Details here");

        assert!(!error_no_details.has_details());
        assert!(error_with_details.has_details());
    }

    /// Test empty message handling
    #[test]
    fn test_empty_message() {
        let error = ErrorResponse::new("");
        assert_eq!(error.message, "");
        assert!(!error.has_details());
    }

    /// Test empty details handling
    #[test]
    fn test_empty_details() {
        let error = ErrorResponse::with_details("Message", "");
        assert_eq!(error.message, "Message");
        assert_eq!(error.details, Some("".to_string()));
        assert!(error.has_details());
    }
}
