//! # LLM Traits
//!
//! This module defines the core traits for LLM providers and models.

use crate::llms::{
    errors::LLMResult,
    types::{LLMConfig, LLMRequest, LLMResponse, ModelInfo, StreamingResponse},
};
use async_trait::async_trait;
use futures_util::Stream;
use std::pin::Pin;

/// Type alias for streaming response
pub type StreamingResponseStream =
    Pin<Box<dyn Stream<Item = LLMResult<StreamingResponse>> + Send + 'static>>;

/// Main trait for LLM providers
///
/// This trait defines the interface for different LLM backend implementations.
/// Providers are responsible for loading models and managing their lifecycle.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// The type of model this provider creates
    type Model: LLMModel;

    /// Initialize the provider with configuration
    ///
    /// # Arguments
    /// * `config` - Configuration for the provider
    ///
    /// # Returns
    /// A [`LLMResult`] containing the initialized provider or an error
    ///
    /// # Errors
    /// Returns an error if initialization fails due to invalid configuration,
    /// missing dependencies, or other provider-specific issues.
    async fn new(config: LLMConfig) -> LLMResult<Self>
    where
        Self: Sized;

    /// Load a model from the given path
    ///
    /// # Arguments
    /// * `model_path` - Path to the model file
    ///
    /// # Returns
    /// A [`LLMResult`] containing the loaded model or an error
    ///
    /// # Errors
    /// Returns an error if the model file is not found, has an invalid format,
    /// or cannot be loaded due to resource constraints.
    async fn load_model(&self, model_path: &str) -> LLMResult<Self::Model>;

    /// Load a model with custom configuration
    ///
    /// # Arguments
    /// * `config` - Configuration for loading the model
    ///
    /// # Returns
    /// A [`LLMResult`] containing the loaded model or an error
    ///
    /// # Errors
    /// Returns an error if the model cannot be loaded with the given configuration.
    async fn load_model_with_config(&self, config: LLMConfig) -> LLMResult<Self::Model>;

    /// Get information about available models
    ///
    /// # Returns
    /// A vector of available model paths or identifiers
    async fn list_available_models(&self) -> LLMResult<Vec<String>>;

    /// Get provider-specific information
    ///
    /// # Returns
    /// A string describing the provider (e.g., "llama-cpp-rs v0.4.0")
    fn get_provider_info(&self) -> String;

    /// Check if a model file is supported by this provider
    ///
    /// # Arguments
    /// * `model_path` - Path to the model file to check
    ///
    /// # Returns
    /// `true` if the model is supported, `false` otherwise
    fn is_model_supported(&self, model_path: &str) -> bool;
}

/// Trait for loaded LLM models
///
/// This trait defines the interface for interacting with a loaded model.
/// Models can generate text, provide embeddings, and stream responses.
#[async_trait]
pub trait LLMModel: Send + Sync {
    /// Generate text from a prompt
    ///
    /// # Arguments
    /// * `request` - The generation request containing prompt and parameters
    ///
    /// # Returns
    /// A [`LLMResult`] containing the generated response or an error
    ///
    /// # Errors
    /// Returns an error if generation fails due to invalid input, resource
    /// exhaustion, or other model-specific issues.
    async fn generate(&self, request: LLMRequest) -> LLMResult<LLMResponse>;

    /// Generate text with streaming response
    ///
    /// # Arguments
    /// * `request` - The generation request containing prompt and parameters
    ///
    /// # Returns
    /// A [`LLMResult`] containing a stream of response chunks or an error
    ///
    /// # Errors
    /// Returns an error if streaming cannot be initiated. Individual stream
    /// items may also contain errors.
    async fn generate_stream(&self, request: LLMRequest) -> LLMResult<StreamingResponseStream>;

    /// Get information about this model
    ///
    /// # Returns
    /// [`ModelInfo`] containing details about the model
    fn get_model_info(&self) -> ModelInfo;

    /// Check if the model is ready for inference
    ///
    /// # Returns
    /// `true` if the model is loaded and ready, `false` otherwise
    fn is_ready(&self) -> bool;

    /// Unload the model and free resources
    ///
    /// # Returns
    /// A [`LLMResult`] indicating success or failure
    ///
    /// # Errors
    /// Returns an error if the model cannot be properly unloaded
    async fn unload(&mut self) -> LLMResult<()>;

    /// Get current memory usage of the model
    ///
    /// # Returns
    /// Memory usage in bytes, or [`None`] if not available
    fn get_memory_usage(&self) -> Option<usize>;

    /// Tokenize text without generating
    ///
    /// # Arguments
    /// * `text` - Text to tokenize
    ///
    /// # Returns
    /// A [`LLMResult`] containing the token count or an error
    ///
    /// # Errors
    /// Returns an error if tokenization fails
    async fn tokenize(&self, text: &str) -> LLMResult<Vec<u32>>;

    /// Get the number of tokens in text
    ///
    /// # Arguments
    /// * `text` - Text to count tokens for
    ///
    /// # Returns
    /// A [`LLMResult`] containing the token count or an error
    ///
    /// # Errors
    /// Returns an error if tokenization fails
    async fn count_tokens(&self, text: &str) -> LLMResult<u32> {
        let tokens = self.tokenize(text).await?;
        Ok(tokens.len() as u32)
    }
}

/// Optional trait for models that support embeddings
#[async_trait]
pub trait LLMEmbedding: LLMModel {
    /// Generate embeddings for the given text
    ///
    /// # Arguments
    /// * `text` - Text to generate embeddings for
    ///
    /// # Returns
    /// A [`LLMResult`] containing the embedding vector or an error
    ///
    /// # Errors
    /// Returns an error if embedding generation fails
    async fn generate_embedding(&self, text: &str) -> LLMResult<Vec<f32>>;

    /// Generate embeddings for multiple texts
    ///
    /// # Arguments
    /// * `texts` - Texts to generate embeddings for
    ///
    /// # Returns
    /// A [`LLMResult`] containing the embedding vectors or an error
    ///
    /// # Errors
    /// Returns an error if embedding generation fails
    async fn generate_embeddings(&self, texts: &[&str]) -> LLMResult<Vec<Vec<f32>>>;

    /// Get the dimension of embeddings produced by this model
    ///
    /// # Returns
    /// The embedding dimension
    fn embedding_dimension(&self) -> usize;
}

/// Optional trait for models that support function calling
#[async_trait]
pub trait LLMFunctionCalling: LLMModel {
    /// Function definition for calling
    type FunctionDef: Send + Sync;

    /// Function call result
    type FunctionCall: Send + Sync;

    /// Register functions that the model can call
    ///
    /// # Arguments
    /// * `functions` - Function definitions to register
    ///
    /// # Returns
    /// A [`LLMResult`] indicating success or failure
    ///
    /// # Errors
    /// Returns an error if function registration fails
    async fn register_functions(&mut self, functions: Vec<Self::FunctionDef>) -> LLMResult<()>;

    /// Generate with function calling support
    ///
    /// # Arguments
    /// * `request` - The generation request
    ///
    /// # Returns
    /// A [`LLMResult`] containing the response with potential function calls
    ///
    /// # Errors
    /// Returns an error if generation or function calling fails
    async fn generate_with_functions(
        &self,
        request: LLMRequest,
    ) -> LLMResult<(LLMResponse, Vec<Self::FunctionCall>)>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llms::types::{ModelCapabilities, TokenUsage};
    use chrono::Utc;
    use std::collections::HashMap;

    // Mock implementations for testing
    struct MockProvider;

    #[async_trait]
    impl LLMProvider for MockProvider {
        type Model = MockModel;

        async fn new(_config: LLMConfig) -> LLMResult<Self> {
            Ok(MockProvider)
        }

        async fn load_model(&self, _model_path: &str) -> LLMResult<Self::Model> {
            Ok(MockModel {
                ready: true,
                info: ModelInfo {
                    name: "mock-model".to_string(),
                    version: Some("1.0".to_string()),
                    architecture: Some("mock".to_string()),
                    parameter_count: Some(1_000_000),
                    context_length: Some(2048),
                    capabilities: ModelCapabilities {
                        text_generation: true,
                        streaming: true,
                        ..Default::default()
                    },
                },
            })
        }

        async fn load_model_with_config(&self, config: LLMConfig) -> LLMResult<Self::Model> {
            self.load_model(&config.model_path).await
        }

        async fn list_available_models(&self) -> LLMResult<Vec<String>> {
            Ok(vec!["mock-model-1".to_string(), "mock-model-2".to_string()])
        }

        fn get_provider_info(&self) -> String {
            "Mock Provider v1.0".to_string()
        }

        fn is_model_supported(&self, model_path: &str) -> bool {
            model_path.ends_with(".mock")
        }
    }

    struct MockModel {
        ready: bool,
        info: ModelInfo,
    }

    #[async_trait]
    impl LLMModel for MockModel {
        async fn generate(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
            Ok(LLMResponse {
                request_id: request.id,
                text: format!("Echo: {}", request.prompt),
                finished: true,
                finish_reason: Some(crate::llms::types::FinishReason::EndOfText),
                usage: TokenUsage::new(10, 20),
                timestamp: Utc::now(),
                model_info: self.info.clone(),
                metadata: HashMap::new(),
            })
        }

        async fn generate_stream(
            &self,
            _request: LLMRequest,
        ) -> LLMResult<StreamingResponseStream> {
            // For testing, we'll just return an empty stream
            use futures_util::stream;
            let stream = stream::empty();
            Ok(Box::pin(stream))
        }

        fn get_model_info(&self) -> ModelInfo {
            self.info.clone()
        }

        fn is_ready(&self) -> bool {
            self.ready
        }

        async fn unload(&mut self) -> LLMResult<()> {
            self.ready = false;
            Ok(())
        }

        fn get_memory_usage(&self) -> Option<usize> {
            Some(1024 * 1024) // 1MB
        }

        async fn tokenize(&self, text: &str) -> LLMResult<Vec<u32>> {
            // Simple mock tokenization
            Ok(text
                .split_whitespace()
                .enumerate()
                .map(|(i, _)| i as u32)
                .collect())
        }
    }

    #[tokio::test]
    async fn test_provider_creation() {
        let config = LLMConfig::default();
        let provider = MockProvider::new(config).await.unwrap();
        assert_eq!(provider.get_provider_info(), "Mock Provider v1.0");
    }

    #[tokio::test]
    async fn test_model_loading() {
        let provider = MockProvider::new(LLMConfig::default()).await.unwrap();
        let model = provider.load_model("test.mock").await.unwrap();
        assert!(model.is_ready());
    }

    #[tokio::test]
    async fn test_text_generation() {
        let provider = MockProvider::new(LLMConfig::default()).await.unwrap();
        let model = provider.load_model("test.mock").await.unwrap();

        let request = LLMRequest::new("Hello, world!");
        let response = model.generate(request).await.unwrap();

        assert_eq!(response.text, "Echo: Hello, world!");
        assert!(response.finished);
    }

    #[tokio::test]
    async fn test_tokenization() {
        let provider = MockProvider::new(LLMConfig::default()).await.unwrap();
        let model = provider.load_model("test.mock").await.unwrap();

        let tokens = model.tokenize("hello world test").await.unwrap();
        assert_eq!(tokens.len(), 3);

        let count = model.count_tokens("hello world test").await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_model_unload() {
        let provider = MockProvider::new(LLMConfig::default()).await.unwrap();
        let mut model = provider.load_model("test.mock").await.unwrap();

        assert!(model.is_ready());
        model.unload().await.unwrap();
        assert!(!model.is_ready());
    }

    #[test]
    fn test_model_support_check() {
        let provider = MockProvider;
        assert!(provider.is_model_supported("model.mock"));
        assert!(!provider.is_model_supported("model.gguf"));
    }
}
