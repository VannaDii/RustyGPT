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
