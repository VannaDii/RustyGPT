//! # LLM Error Types
//!
//! This module defines error types for LLM operations.

use thiserror::Error;

/// Result type alias for LLM operations
pub type LLMResult<T> = Result<T, LLMError>;

/// Comprehensive error type for LLM operations
#[derive(Error, Debug)]
pub enum LLMError {
    /// Model initialization failed
    #[error("Model initialization failed: {message}")]
    ModelInitializationFailed { message: String },

    /// Model not found at the specified path
    #[error("Model not found at path: {path}")]
    ModelNotFound { path: String },

    /// Invalid model format or configuration
    #[error("Invalid model format: {details}")]
    InvalidModelFormat { details: String },

    /// Text generation failed
    #[error("Text generation failed: {reason}")]
    GenerationFailed { reason: String },

    /// Tokenization error
    #[error("Tokenization error: {details}")]
    TokenizationError { details: String },

    /// Model is not loaded
    #[error("Model is not loaded")]
    ModelNotLoaded,

    /// Invalid configuration
    #[error("Invalid configuration: {field} - {message}")]
    InvalidConfiguration { field: String, message: String },

    /// Resource exhaustion (memory, GPU, etc.)
    #[error("Resource exhaustion: {resource} - {details}")]
    ResourceExhaustion { resource: String, details: String },

    /// Operation timeout
    #[error("Operation timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// Internal library error
    #[error("Internal error: {source}")]
    InternalError {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Input validation error
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    /// Model capability not supported
    #[error("Operation not supported: {operation}")]
    UnsupportedOperation { operation: String },

    /// IO related errors
    #[error("IO error: {message}")]
    IoError { message: String },
}

impl LLMError {
    /// Create a new model initialization error
    pub fn model_init_failed<T: Into<String>>(message: T) -> Self {
        Self::ModelInitializationFailed {
            message: message.into(),
        }
    }

    /// Create a new model not found error
    pub fn model_not_found<T: Into<String>>(path: T) -> Self {
        Self::ModelNotFound { path: path.into() }
    }

    /// Create a new generation failed error
    pub fn generation_failed<T: Into<String>>(reason: T) -> Self {
        Self::GenerationFailed {
            reason: reason.into(),
        }
    }

    /// Create a new invalid configuration error
    pub fn invalid_config<T: Into<String>, U: Into<String>>(field: T, message: U) -> Self {
        Self::InvalidConfiguration {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a new invalid input error
    pub fn invalid_input<T: Into<String>>(message: T) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    /// Create a new unsupported operation error
    pub fn unsupported_operation<T: Into<String>>(operation: T) -> Self {
        Self::UnsupportedOperation {
            operation: operation.into(),
        }
    }

    /// Wrap an external error as an internal error
    pub fn internal<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::InternalError {
            source: Box::new(error),
        }
    }
}

// Convert from std::io::Error
impl From<std::io::Error> for LLMError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError {
            message: error.to_string(),
        }
    }
}

// Convert from serde_json::Error for configuration parsing
impl From<serde_json::Error> for LLMError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidConfiguration {
            field: "json".to_string(),
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = LLMError::model_init_failed("Failed to load model");
        assert!(matches!(error, LLMError::ModelInitializationFailed { .. }));
        assert_eq!(
            error.to_string(),
            "Model initialization failed: Failed to load model"
        );
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let llm_error: LLMError = io_error.into();
        assert!(matches!(llm_error, LLMError::IoError { .. }));
    }

    #[test]
    fn test_error_chaining() {
        let original_error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
        let llm_error = LLMError::internal(original_error);

        match llm_error {
            LLMError::InternalError { source } => {
                assert!(source.to_string().contains("Access denied"));
            }
            _ => panic!("Expected InternalError"),
        }
    }

    #[test]
    fn test_error_display() {
        let errors = vec![
            LLMError::ModelNotFound {
                path: "/path/to/model".to_string(),
            },
            LLMError::GenerationFailed {
                reason: "Out of memory".to_string(),
            },
            LLMError::InvalidInput {
                message: "Empty prompt".to_string(),
            },
            LLMError::UnsupportedOperation {
                operation: "embedding".to_string(),
            },
        ];

        for error in errors {
            // Ensure each error has a proper display implementation
            assert!(!error.to_string().is_empty());
        }
    }
}
