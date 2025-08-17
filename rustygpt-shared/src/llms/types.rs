//! # LLM Type Definitions
//!
//! This module contains all the data types used throughout the LLM system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration for initializing an LLM model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// Path to the model file
    pub model_path: String,

    /// Maximum number of tokens to generate
    pub max_tokens: Option<u32>,

    /// Temperature for sampling (0.0 to 1.0)
    pub temperature: Option<f32>,

    /// Top-p sampling parameter
    pub top_p: Option<f32>,

    /// Top-k sampling parameter
    pub top_k: Option<u32>,

    /// Repetition penalty
    pub repetition_penalty: Option<f32>,

    /// Number of threads to use
    pub n_threads: Option<u32>,

    /// GPU layers to use (if available)
    pub n_gpu_layers: Option<u32>,

    /// Context window size
    pub context_size: Option<u32>,

    /// Batch size for processing
    pub batch_size: Option<u32>,

    /// Additional model-specific parameters
    pub additional_params: HashMap<String, serde_json::Value>,
}

impl LLMConfig {
    /// Apply optimal parameters based on hardware detection
    ///
    /// This method updates the configuration with hardware-optimized settings
    /// for best performance while ensuring system stability.
    ///
    /// # Arguments
    ///
    /// * `optimal_params` - The [`OptimalParams`](crate::llms::OptimalParams) calculated from hardware detection
    /// * `model_size_estimate` - Optional estimated model size in bytes for better optimization
    ///
    /// # Returns
    ///
    /// A new [`LLMConfig`] with optimized parameters applied.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use shared::llms::{LLMConfig, SystemHardware};
    ///
    /// let hardware = SystemHardware::detect()?;
    /// let optimal_params = hardware.calculate_optimal_params(Some(4_000_000_000)); // 4GB model
    /// let config = LLMConfig::default().apply_optimal_params(&optimal_params, None);
    /// ```
    pub fn apply_optimal_params(
        mut self,
        optimal_params: &crate::llms::OptimalParams,
        model_size_estimate: Option<u64>,
    ) -> Self {
        // Apply hardware-optimized settings
        self.n_threads = Some(optimal_params.n_threads);
        self.n_gpu_layers = Some(optimal_params.n_gpu_layers);
        self.context_size = Some(optimal_params.context_size);
        self.batch_size = Some(optimal_params.batch_size);

        // Add memory mapping if supported
        if optimal_params.use_mmap {
            self.additional_params
                .insert("use_mmap".to_string(), serde_json::Value::Bool(true));
        }

        // Add model size estimate if provided
        if let Some(size) = model_size_estimate {
            self.additional_params.insert(
                "estimated_model_size".to_string(),
                serde_json::Value::Number(serde_json::Number::from(size)),
            );
        }

        // Add memory buffer information
        self.additional_params.insert(
            "memory_buffer_percent".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(optimal_params.memory_buffer_percent as f64)
                    .unwrap_or_else(|| serde_json::Number::from(20)), // 20% as integer fallback
            ),
        );

        self
    }

    /// Create an optimized configuration for the detected hardware
    ///
    /// This is a convenience method that detects hardware and applies optimal parameters
    /// in one step.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to the model file to load
    /// * `model_size_estimate` - Optional estimated model size in bytes
    ///
    /// # Returns
    ///
    /// A [`Result`] containing either an optimized [`LLMConfig`] or a hardware detection error.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::llms::hardware::HardwareError`] if hardware detection fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use shared::llms::LLMConfig;
    ///
    /// let config = LLMConfig::optimized_for_hardware(
    ///     "/path/to/model.gguf",
    ///     Some(4_000_000_000), // 4GB model
    /// )?;
    /// ```
    pub fn optimized_for_hardware<P: Into<String>>(
        model_path: P,
        model_size_estimate: Option<u64>,
    ) -> Result<Self, crate::llms::hardware::HardwareError> {
        let hardware = crate::llms::SystemHardware::detect()?;
        let optimal_params = hardware.calculate_optimal_params(model_size_estimate);

        let config = Self {
            model_path: model_path.into(),
            ..Default::default()
        }
        .apply_optimal_params(&optimal_params, model_size_estimate);

        Ok(config)
    }

    /// Validate that the configuration is safe for the current hardware
    ///
    /// This method checks if the configuration parameters are reasonable
    /// for the detected hardware to prevent system overload.
    ///
    /// # Returns
    ///
    /// A [`Result`] indicating whether the configuration is safe, or containing
    /// a vector of warning messages if potential issues are detected.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use shared::llms::LLMConfig;
    ///
    /// let config = LLMConfig::default();
    /// match config.validate_for_hardware() {
    ///     Ok(()) => println!("Configuration is safe"),
    ///     Err(warnings) => {
    ///         for warning in warnings {
    ///             println!("Warning: {}", warning);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn validate_for_hardware(&self) -> Result<(), Vec<String>> {
        let mut warnings = Vec::new();

        // Try to detect hardware for validation
        match crate::llms::SystemHardware::detect() {
            Ok(hardware) => {
                let optimal_params = hardware.calculate_optimal_params(None);

                // Check thread count
                if let Some(threads) = self.n_threads {
                    if threads > hardware.cpu_threads {
                        warnings.push(format!(
                            "Thread count ({}) exceeds available CPU threads ({})",
                            threads, hardware.cpu_threads
                        ));
                    }
                    if threads > optimal_params.n_threads * 2 {
                        warnings.push(format!(
                            "Thread count ({}) is significantly higher than recommended ({})",
                            threads, optimal_params.n_threads
                        ));
                    }
                }

                // Check GPU layers
                if let Some(gpu_layers) = self.n_gpu_layers {
                    if gpu_layers > 0 && hardware.gpu_type == crate::llms::GpuType::None {
                        warnings.push(
                            "GPU layers specified but no compatible GPU detected".to_string(),
                        );
                    }
                    if gpu_layers > optimal_params.n_gpu_layers * 2 {
                        warnings.push(format!(
                            "GPU layers ({}) significantly exceed recommended ({})",
                            gpu_layers, optimal_params.n_gpu_layers
                        ));
                    }
                }

                // Check context size
                if let Some(context_size) = self.context_size {
                    if context_size > optimal_params.context_size * 2 {
                        warnings.push(format!(
                            "Context size ({}) may be too large for available memory. Recommended: {}",
                            context_size, optimal_params.context_size
                        ));
                    }
                }

                // Check batch size
                if let Some(batch_size) = self.batch_size {
                    if batch_size > optimal_params.batch_size * 2 {
                        warnings.push(format!(
                            "Batch size ({}) may be too large for available memory. Recommended: {}",
                            batch_size, optimal_params.batch_size
                        ));
                    }
                }
            }
            Err(_) => {
                warnings.push("Unable to detect hardware for validation".to_string());
            }
        }

        if warnings.is_empty() {
            Ok(())
        } else {
            Err(warnings)
        }
    }
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            model_path: String::new(),
            max_tokens: Some(512),
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(40),
            repetition_penalty: Some(1.1),
            n_threads: None,       // Will use system default
            n_gpu_layers: Some(0), // CPU-only by default
            context_size: Some(2048),
            batch_size: Some(512),
            additional_params: HashMap::new(),
        }
    }
}

/// Request for text generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    /// Unique identifier for this request
    pub id: Uuid,

    /// The prompt text
    pub prompt: String,

    /// System message/context (optional)
    pub system_message: Option<String>,

    /// Maximum tokens to generate for this request
    pub max_tokens: Option<u32>,

    /// Temperature override for this request
    pub temperature: Option<f32>,

    /// Whether to stream the response
    pub stream: bool,

    /// Stop sequences to end generation
    pub stop_sequences: Vec<String>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl LLMRequest {
    /// Create a new LLM request with default settings
    pub fn new<T: Into<String>>(prompt: T) -> Self {
        Self {
            id: Uuid::new_v4(),
            prompt: prompt.into(),
            system_message: None,
            max_tokens: None,
            temperature: None,
            stream: false,
            stop_sequences: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new streaming LLM request
    pub fn new_streaming<T: Into<String>>(prompt: T) -> Self {
        Self {
            stream: true,
            ..Self::new(prompt)
        }
    }

    /// Set the system message
    pub fn with_system_message<T: Into<String>>(mut self, system_message: T) -> Self {
        self.system_message = Some(system_message.into());
        self
    }

    /// Set the maximum tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Add a stop sequence
    pub fn with_stop_sequence<T: Into<String>>(mut self, stop_sequence: T) -> Self {
        self.stop_sequences.push(stop_sequence.into());
        self
    }

    /// Add metadata
    pub fn with_metadata<K: Into<String>>(mut self, key: K, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Response from text generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    /// Request ID this response corresponds to
    pub request_id: Uuid,

    /// Generated text
    pub text: String,

    /// Whether generation finished successfully
    pub finished: bool,

    /// Reason for finishing (if finished)
    pub finish_reason: Option<FinishReason>,

    /// Token usage statistics
    pub usage: TokenUsage,

    /// Generation timestamp
    pub timestamp: DateTime<Utc>,

    /// Model information
    pub model_info: ModelInfo,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResponse {
    /// Request ID this chunk corresponds to
    pub request_id: Uuid,

    /// Text chunk (delta from previous chunk)
    pub text_delta: String,

    /// Whether this is the final chunk
    pub is_final: bool,

    /// Current total text (optional, for convenience)
    pub current_text: Option<String>,

    /// Finish reason (only set on final chunk)
    pub finish_reason: Option<FinishReason>,

    /// Current token usage
    pub usage: TokenUsage,

    /// Chunk timestamp
    pub timestamp: DateTime<Utc>,
}

/// Reason why generation finished
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FinishReason {
    /// Reached maximum token limit
    MaxTokens,

    /// Hit a stop sequence
    StopSequence,

    /// Model decided to stop naturally
    EndOfText,

    /// Generation was cancelled
    Cancelled,

    /// An error occurred
    Error,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,

    /// Number of tokens generated
    pub completion_tokens: u32,

    /// Total tokens used (prompt + completion)
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Create new token usage
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }

    /// Update with new completion tokens
    pub fn add_completion_tokens(&mut self, tokens: u32) {
        self.completion_tokens += tokens;
        self.total_tokens = self.prompt_tokens + self.completion_tokens;
    }
}

/// Information about the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name/identifier
    pub name: String,

    /// Model version
    pub version: Option<String>,

    /// Model architecture (e.g., "llama", "gpt")
    pub architecture: Option<String>,

    /// Model size in parameters
    pub parameter_count: Option<u64>,

    /// Context window size
    pub context_length: Option<u32>,

    /// Supported capabilities
    pub capabilities: ModelCapabilities,
}

/// Model capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    /// Supports text generation
    pub text_generation: bool,

    /// Supports text embedding
    pub text_embedding: bool,

    /// Supports chat/conversation format
    pub chat_format: bool,

    /// Supports function calling
    pub function_calling: bool,

    /// Supports streaming responses
    pub streaming: bool,

    /// Maximum context length
    pub max_context_length: Option<u32>,

    /// Supported languages (ISO codes)
    pub supported_languages: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_default() {
        let config = LLMConfig::default();
        assert_eq!(config.max_tokens, Some(512));
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.context_size, Some(2048));
    }

    #[test]
    fn test_llm_request_creation() {
        let request = LLMRequest::new("Hello, world!");
        assert_eq!(request.prompt, "Hello, world!");
        assert!(!request.stream);
        assert!(request.system_message.is_none());
    }

    #[test]
    fn test_llm_request_builder() {
        let request = LLMRequest::new("Test prompt")
            .with_system_message("You are a helpful assistant")
            .with_max_tokens(100)
            .with_temperature(0.5)
            .with_stop_sequence("STOP");

        assert_eq!(request.prompt, "Test prompt");
        assert_eq!(
            request.system_message,
            Some("You are a helpful assistant".to_string())
        );
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.5));
        assert_eq!(request.stop_sequences, vec!["STOP"]);
    }

    #[test]
    fn test_streaming_request() {
        let request = LLMRequest::new_streaming("Stream this");
        assert!(request.stream);
        assert_eq!(request.prompt, "Stream this");
    }

    #[test]
    fn test_token_usage() {
        let mut usage = TokenUsage::new(50, 25);
        assert_eq!(usage.prompt_tokens, 50);
        assert_eq!(usage.completion_tokens, 25);
        assert_eq!(usage.total_tokens, 75);

        usage.add_completion_tokens(10);
        assert_eq!(usage.completion_tokens, 35);
        assert_eq!(usage.total_tokens, 85);
    }

    #[test]
    fn test_finish_reason_equality() {
        assert_eq!(FinishReason::MaxTokens, FinishReason::MaxTokens);
        assert_ne!(FinishReason::MaxTokens, FinishReason::StopSequence);
    }

    #[test]
    fn test_model_capabilities_default() {
        let capabilities = ModelCapabilities::default();
        assert!(!capabilities.text_generation);
        assert!(!capabilities.streaming);
        assert!(capabilities.supported_languages.is_empty());
    }

    #[test]
    fn test_serialization() {
        let request = LLMRequest::new("Test")
            .with_max_tokens(100)
            .with_temperature(0.8);

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: LLMRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(request.prompt, deserialized.prompt);
        assert_eq!(request.max_tokens, deserialized.max_tokens);
        assert_eq!(request.temperature, deserialized.temperature);
    }
}
