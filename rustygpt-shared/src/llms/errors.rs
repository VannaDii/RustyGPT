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
    fn test_model_init_failed_creation() {
        let error = LLMError::model_init_failed("Failed to load model");
        assert!(matches!(error, LLMError::ModelInitializationFailed { .. }));
        assert_eq!(
            error.to_string(),
            "Model initialization failed: Failed to load model"
        );
    }

    #[test]
    fn test_model_not_found_creation() {
        let error = LLMError::model_not_found("/path/to/missing/model.gguf");
        assert!(matches!(error, LLMError::ModelNotFound { .. }));
        assert_eq!(
            error.to_string(),
            "Model not found at path: /path/to/missing/model.gguf"
        );
    }

    #[test]
    fn test_generation_failed_creation() {
        let error = LLMError::generation_failed("Context too long");
        assert!(matches!(error, LLMError::GenerationFailed { .. }));
        assert_eq!(
            error.to_string(),
            "Text generation failed: Context too long"
        );
    }

    #[test]
    fn test_invalid_config_creation() {
        let error = LLMError::invalid_config("temperature", "must be between 0.0 and 2.0");
        assert!(matches!(error, LLMError::InvalidConfiguration { .. }));
        assert_eq!(
            error.to_string(),
            "Invalid configuration: temperature - must be between 0.0 and 2.0"
        );
    }

    #[test]
    fn test_invalid_input_creation() {
        let error = LLMError::invalid_input("Empty prompt provided");
        assert!(matches!(error, LLMError::InvalidInput { .. }));
        assert_eq!(error.to_string(), "Invalid input: Empty prompt provided");
    }

    #[test]
    fn test_unsupported_operation_creation() {
        let error = LLMError::unsupported_operation("multimodal generation");
        assert!(matches!(error, LLMError::UnsupportedOperation { .. }));
        assert_eq!(
            error.to_string(),
            "Operation not supported: multimodal generation"
        );
    }

    #[test]
    fn test_internal_error_creation() {
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
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let llm_error: LLMError = io_error.into();
        assert!(matches!(llm_error, LLMError::IoError { .. }));
        assert!(llm_error.to_string().contains("File not found"));
    }

    #[test]
    fn test_serde_json_error_conversion() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json");
        assert!(json_error.is_err());
        let llm_error: LLMError = json_error.unwrap_err().into();
        assert!(matches!(llm_error, LLMError::InvalidConfiguration { .. }));
        assert!(
            llm_error
                .to_string()
                .contains("Invalid configuration: json")
        );
    }

    #[test]
    fn test_all_error_variants_display() {
        let errors = vec![
            LLMError::ModelInitializationFailed {
                message: "Init failed".to_string(),
            },
            LLMError::ModelNotFound {
                path: "/path/to/model".to_string(),
            },
            LLMError::InvalidModelFormat {
                details: "Corrupted file".to_string(),
            },
            LLMError::GenerationFailed {
                reason: "Out of memory".to_string(),
            },
            LLMError::TokenizationError {
                details: "Invalid token".to_string(),
            },
            LLMError::ModelNotLoaded,
            LLMError::InvalidConfiguration {
                field: "temperature".to_string(),
                message: "Too high".to_string(),
            },
            LLMError::ResourceExhaustion {
                resource: "GPU memory".to_string(),
                details: "8GB required".to_string(),
            },
            LLMError::Timeout { seconds: 30 },
            LLMError::InternalError {
                source: Box::new(std::io::Error::other("Generic error")),
            },
            LLMError::InvalidInput {
                message: "Empty prompt".to_string(),
            },
            LLMError::UnsupportedOperation {
                operation: "embedding".to_string(),
            },
            LLMError::IoError {
                message: "Disk full".to_string(),
            },
        ];

        for error in errors {
            // Ensure each error has a proper display implementation
            let error_str = error.to_string();
            assert!(!error_str.is_empty());
            assert!(error_str.len() > 5); // Reasonable minimum length
        }
    }

    #[test]
    fn test_error_variant_matching() {
        let model_not_found = LLMError::ModelNotFound {
            path: "test.gguf".to_string(),
        };
        assert!(matches!(model_not_found, LLMError::ModelNotFound { .. }));

        let model_not_loaded = LLMError::ModelNotLoaded;
        assert!(matches!(model_not_loaded, LLMError::ModelNotLoaded));

        let timeout = LLMError::Timeout { seconds: 10 };
        assert!(matches!(timeout, LLMError::Timeout { .. }));
    }

    #[test]
    fn test_llm_result_type_alias() {
        let success: LLMResult<String> = Ok("Success".to_string());
        assert!(success.is_ok());
        if let Ok(value) = success {
            assert_eq!(value, "Success");
        }

        let failure: LLMResult<String> = Err(LLMError::ModelNotLoaded);
        assert!(failure.is_err());
        if let Err(error) = failure {
            assert!(matches!(error, LLMError::ModelNotLoaded));
        }
    }

    #[test]
    fn test_error_debug_trait() {
        let error = LLMError::InvalidInput {
            message: "test".to_string(),
        };
        let debug_str = format!("{error:?}");
        assert!(debug_str.contains("InvalidInput"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_builder_methods_with_different_string_types() {
        // Test with &str
        let error1 = LLMError::model_init_failed("test");
        assert!(matches!(error1, LLMError::ModelInitializationFailed { .. }));

        // Test with String
        let error2 = LLMError::model_not_found("test".to_string());
        assert!(matches!(error2, LLMError::ModelNotFound { .. }));

        // Test with owned String from format!
        let error3 = LLMError::generation_failed(format!("test {}", 123));
        assert!(matches!(error3, LLMError::GenerationFailed { .. }));

        // Test with String from concatenation
        let prefix = "Error";
        let suffix = "occurred";
        let error4 = LLMError::invalid_input(format!("{prefix} {suffix}"));
        assert!(matches!(error4, LLMError::InvalidInput { .. }));
    }

    #[test]
    fn test_error_source_extraction() {
        let original_error = std::io::Error::other("Source error");
        let llm_error = LLMError::internal(original_error);

        if let LLMError::InternalError { source } = llm_error {
            assert_eq!(source.to_string(), "Source error");
        } else {
            panic!("Expected InternalError variant");
        }
    }

    #[test]
    fn test_resource_exhaustion_variants() {
        let memory_error = LLMError::ResourceExhaustion {
            resource: "system memory".to_string(),
            details: "16GB required, 8GB available".to_string(),
        };
        assert!(
            memory_error
                .to_string()
                .contains("Resource exhaustion: system memory")
        );

        let gpu_error = LLMError::ResourceExhaustion {
            resource: "GPU VRAM".to_string(),
            details: "Model requires 12GB".to_string(),
        };
        assert!(
            gpu_error
                .to_string()
                .contains("Resource exhaustion: GPU VRAM")
        );
    }

    #[test]
    fn test_timeout_error_formatting() {
        let timeout_1s = LLMError::Timeout { seconds: 1 };
        assert_eq!(
            timeout_1s.to_string(),
            "Operation timed out after 1 seconds"
        );

        let timeout_sixty_seconds = LLMError::Timeout { seconds: 60 };
        assert_eq!(
            timeout_sixty_seconds.to_string(),
            "Operation timed out after 60 seconds"
        );

        let timeout_zero_seconds = LLMError::Timeout { seconds: 0 };
        assert_eq!(
            timeout_zero_seconds.to_string(),
            "Operation timed out after 0 seconds"
        );
    }
}
