//! # Mock Llama.cpp Implementation
//!
//! This module provides a mock implementation of the LLM traits for development and testing.
//! This can be replaced with a real `llama-cpp-rs` implementation once the build issues are resolved.
//!
//! The mock implementation provides realistic interfaces and can be used to test the trait system
//! and develop the server/CLI integration without requiring actual model files.

use crate::llms::{
    errors::{LLMError, LLMResult},
    traits::{LLMModel, LLMProvider, StreamingResponseStream},
    types::{
        FinishReason, LLMConfig, LLMRequest, LLMResponse, ModelCapabilities, ModelInfo,
        StreamingResponse, TokenUsage,
    },
};
use async_trait::async_trait;
use chrono::Utc;
use futures_util::{StreamExt, stream};
use std::{collections::HashMap, path::Path, time::Duration};
use tokio::time::sleep;

/// Mock Llama.cpp provider implementation
///
/// This is a development implementation that simulates the behavior of a real LLM
/// without requiring actual model files or the `llama-cpp-rs` dependency.
#[derive(Debug, Clone)]
pub struct LlamaCppProvider {
    /// Configuration used to initialize models
    config: LLMConfig,
}

/// Mock Llama.cpp model implementation
///
/// Provides realistic responses for development and testing purposes.
#[derive(Debug, Clone)]
pub struct LlamaCppModel {
    /// Model configuration
    config: LLMConfig,
    /// Model information
    info: ModelInfo,
    /// Whether the model is ready for inference
    ready: bool,
    /// Simulated memory usage
    memory_usage: usize,
}

#[async_trait]
impl LLMProvider for LlamaCppProvider {
    type Model = LlamaCppModel;

    async fn new(config: LLMConfig) -> LLMResult<Self> {
        // Validate configuration
        if config.model_path.is_empty() {
            return Err(LLMError::invalid_config(
                "model_path",
                "Model path cannot be empty",
            ));
        }

        Ok(Self { config })
    }

    async fn load_model(&self, model_path: &str) -> LLMResult<Self::Model> {
        let mut config = self.config.clone();
        config.model_path = model_path.to_string();
        self.load_model_with_config(config).await
    }

    async fn load_model_with_config(&self, config: LLMConfig) -> LLMResult<Self::Model> {
        // Validate model path exists
        if !Path::new(&config.model_path).exists() {
            return Err(LLMError::model_not_found(&config.model_path));
        }

        // Check if model format is supported
        if !self.is_model_supported(&config.model_path) {
            return Err(LLMError::InvalidModelFormat {
                details: format!("Unsupported model format: {}", config.model_path),
            });
        }

        // Simulate model loading time
        sleep(Duration::from_millis(100)).await;

        // Extract model information
        let info = Self::extract_model_info(&config)?;

        Ok(LlamaCppModel {
            config,
            info,
            ready: true,
            memory_usage: 1024 * 1024 * 512, // 512MB simulated
        })
    }

    async fn list_available_models(&self) -> LLMResult<Vec<String>> {
        // This would typically scan a models directory
        // For now, return empty list as this is provider-specific
        Ok(Vec::new())
    }

    fn get_provider_info(&self) -> String {
        "Mock LlamaCpp Provider v1.0 (Development Only)".to_string()
    }

    fn is_model_supported(&self, model_path: &str) -> bool {
        // Check for GGUF format
        model_path.ends_with(".gguf") || model_path.ends_with(".GGUF")
    }
}

impl LlamaCppProvider {
    /// Extract model information from configuration
    fn extract_model_info(config: &LLMConfig) -> LLMResult<ModelInfo> {
        let name = Path::new(&config.model_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let context_length = config.context_size.unwrap_or(2048);

        Ok(ModelInfo {
            name,
            version: Some("mock-1.0".to_string()),
            architecture: Some("llama".to_string()),
            parameter_count: Some(7_000_000_000), // 7B parameters
            context_length: Some(context_length),
            capabilities: ModelCapabilities {
                text_generation: true,
                text_embedding: false,
                chat_format: true,
                function_calling: false,
                streaming: true,
                max_context_length: Some(context_length),
                supported_languages: vec!["en".to_string(), "es".to_string(), "fr".to_string()],
            },
        })
    }
}

#[async_trait]
impl LLMModel for LlamaCppModel {
    async fn generate(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        if !self.is_ready() {
            return Err(LLMError::ModelNotLoaded);
        }

        // Simulate processing time
        let processing_time = Duration::from_millis(100 + (request.prompt.len() as u64 * 2));
        sleep(processing_time).await;

        // Prepare the prompt
        let full_prompt = if let Some(system_msg) = &request.system_message {
            format!("{}\n\n{}", system_msg, request.prompt)
        } else {
            request.prompt.clone()
        };

        // Generate mock response based on prompt
        let generated_text = self.generate_mock_response(&request.prompt);

        // Simulate token counting
        let prompt_tokens = self.estimate_tokens(&full_prompt);
        let completion_tokens = self.estimate_tokens(&generated_text);

        // Determine finish reason
        let max_tokens = request.max_tokens.or(self.config.max_tokens).unwrap_or(512);

        let finish_reason = if completion_tokens >= max_tokens {
            FinishReason::MaxTokens
        } else if request
            .stop_sequences
            .iter()
            .any(|seq| generated_text.contains(seq))
        {
            FinishReason::StopSequence
        } else {
            FinishReason::EndOfText
        };

        Ok(LLMResponse {
            request_id: request.id,
            text: generated_text,
            finished: true,
            finish_reason: Some(finish_reason),
            usage: TokenUsage::new(prompt_tokens, completion_tokens),
            timestamp: Utc::now(),
            model_info: self.info.clone(),
            metadata: HashMap::new(),
        })
    }

    async fn generate_stream(&self, request: LLMRequest) -> LLMResult<StreamingResponseStream> {
        if !self.is_ready() {
            return Err(LLMError::ModelNotLoaded);
        }

        let response_text = self.generate_mock_response(&request.prompt);
        let words: Vec<&str> = response_text.split_whitespace().collect();
        let request_id = request.id;

        // Create streaming chunks
        let chunks: Vec<_> = words
            .iter()
            .enumerate()
            .map(|(i, word)| {
                let is_final = i == words.len() - 1;
                let text_delta = if i == 0 {
                    word.to_string()
                } else {
                    format!(" {}", word)
                };

                // Clone for move into async block
                let response_text_clone = response_text.clone();

                // Simulate streaming delay
                let delay = Duration::from_millis(50);

                Box::pin(async move {
                    sleep(delay).await;
                    Ok(StreamingResponse {
                        request_id,
                        text_delta,
                        is_final,
                        current_text: if is_final {
                            Some(response_text_clone)
                        } else {
                            None
                        },
                        finish_reason: if is_final {
                            Some(FinishReason::EndOfText)
                        } else {
                            None
                        },
                        usage: if is_final {
                            TokenUsage::new(10, 20)
                        } else {
                            TokenUsage::default()
                        },
                        timestamp: Utc::now(),
                    })
                })
            })
            .collect();

        // Convert to stream
        let stream = stream::iter(chunks).then(|chunk| chunk);
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
        self.memory_usage = 0;
        Ok(())
    }

    fn get_memory_usage(&self) -> Option<usize> {
        Some(self.memory_usage)
    }

    async fn tokenize(&self, text: &str) -> LLMResult<Vec<u32>> {
        // Simple mock tokenization - split by whitespace and assign IDs
        let tokens: Vec<u32> = text
            .split_whitespace()
            .enumerate()
            .map(|(i, _)| (i as u32) + 1000) // Offset to avoid special token ranges
            .collect();

        Ok(tokens)
    }
}

impl LlamaCppModel {
    /// Generate a mock response based on the input prompt
    fn generate_mock_response(&self, prompt: &str) -> String {
        // Simple rule-based mock responses
        let prompt_lower = prompt.to_lowercase();

        if prompt_lower.contains("hello") || prompt_lower.contains("hi") {
            "Hello! I'm a mock LLM implementation. How can I help you today?".to_string()
        } else if prompt_lower.contains("what")
            && prompt_lower.contains("your")
            && prompt_lower.contains("name")
        {
            "I'm a mock implementation of the RustyGPT LLM system. I'm designed for development and testing purposes.".to_string()
        } else if prompt_lower.contains("code") || prompt_lower.contains("program") {
            "I can help you with coding! Here's a simple example:\n\n```rust\nfn main() {\n    println!(\"Hello, RustyGPT!\");\n}\n```".to_string()
        } else if prompt_lower.contains("explain") {
            format!(
                "I'd be happy to explain that! The topic '{}' is quite interesting. This is a mock response that demonstrates the LLM trait system working correctly.",
                prompt.chars().take(50).collect::<String>()
            )
        } else {
            format!(
                "Thank you for your input: '{}'. This is a mock response from the LLM trait system. In a real implementation, this would be generated by a language model.",
                prompt.chars().take(100).collect::<String>()
            )
        }
    }

    /// Estimate token count for text (simple approximation)
    fn estimate_tokens(&self, text: &str) -> u32 {
        // Simple approximation: ~4 characters per token
        ((text.len() as f32) / 4.0).ceil() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_provider_creation() {
        let config = LLMConfig {
            model_path: "test.gguf".to_string(),
            ..Default::default()
        };
        let result = LlamaCppProvider::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_empty_model_path_error() {
        let config = LLMConfig {
            model_path: String::new(),
            ..Default::default()
        };
        let result = LlamaCppProvider::new(config).await;
        assert!(result.is_err());

        if let Err(LLMError::InvalidConfiguration { field, .. }) = result {
            assert_eq!(field, "model_path");
        } else {
            panic!("Expected InvalidConfiguration error");
        }
    }

    #[test]
    fn test_model_support_check() {
        let provider = LlamaCppProvider {
            config: LLMConfig::default(),
        };

        assert!(provider.is_model_supported("model.gguf"));
        assert!(provider.is_model_supported("model.GGUF"));
        assert!(!provider.is_model_supported("model.bin"));
        assert!(!provider.is_model_supported("model.safetensors"));
    }

    #[tokio::test]
    async fn test_model_loading_with_real_file() {
        let temp_dir = tempdir().unwrap();
        let model_path = temp_dir.path().join("test_model.gguf");

        // Create a mock model file
        fs::write(&model_path, "mock model content").unwrap();

        let config = LLMConfig {
            model_path: model_path.to_string_lossy().to_string(),
            ..Default::default()
        };

        let provider = LlamaCppProvider::new(config.clone()).await.unwrap();
        let model = provider.load_model_with_config(config).await.unwrap();

        assert!(model.is_ready());
        assert_eq!(model.get_model_info().name, "test_model");
    }

    #[tokio::test]
    async fn test_model_not_found_error() {
        let config = LLMConfig {
            model_path: "nonexistent.gguf".to_string(),
            ..Default::default()
        };

        let provider = LlamaCppProvider::new(config.clone()).await.unwrap();
        let result = provider.load_model_with_config(config).await;

        assert!(result.is_err());
        if let Err(LLMError::ModelNotFound { path }) = result {
            assert_eq!(path, "nonexistent.gguf");
        } else {
            panic!("Expected ModelNotFound error");
        }
    }

    #[tokio::test]
    async fn test_text_generation() {
        let temp_dir = tempdir().unwrap();
        let model_path = temp_dir.path().join("test_model.gguf");
        fs::write(&model_path, "mock model content").unwrap();

        let config = LLMConfig {
            model_path: model_path.to_string_lossy().to_string(),
            ..Default::default()
        };

        let provider = LlamaCppProvider::new(config.clone()).await.unwrap();
        let model = provider.load_model_with_config(config).await.unwrap();

        let request = LLMRequest::new("Hello, world!");
        let response = model.generate(request).await.unwrap();

        assert!(response.text.contains("Hello"));
        assert!(response.finished);
        assert!(response.usage.total_tokens > 0);
    }

    #[tokio::test]
    async fn test_streaming_generation() {
        let temp_dir = tempdir().unwrap();
        let model_path = temp_dir.path().join("test_model.gguf");
        fs::write(&model_path, "mock model content").unwrap();

        let config = LLMConfig {
            model_path: model_path.to_string_lossy().to_string(),
            ..Default::default()
        };

        let provider = LlamaCppProvider::new(config.clone()).await.unwrap();
        let model = provider.load_model_with_config(config).await.unwrap();

        let request = LLMRequest::new_streaming("Hello");
        let mut stream = model.generate_stream(request).await.unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk.unwrap());
        }

        assert!(!chunks.is_empty());
        assert!(chunks.last().unwrap().is_final);
    }

    #[tokio::test]
    async fn test_tokenization() {
        let temp_dir = tempdir().unwrap();
        let model_path = temp_dir.path().join("test_model.gguf");
        fs::write(&model_path, "mock model content").unwrap();

        let config = LLMConfig {
            model_path: model_path.to_string_lossy().to_string(),
            ..Default::default()
        };

        let provider = LlamaCppProvider::new(config.clone()).await.unwrap();
        let model = provider.load_model_with_config(config).await.unwrap();

        let tokens = model.tokenize("hello world test").await.unwrap();
        assert_eq!(tokens.len(), 3);

        let count = model.count_tokens("hello world test").await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_model_unload() {
        let temp_dir = tempdir().unwrap();
        let model_path = temp_dir.path().join("test_model.gguf");
        fs::write(&model_path, "mock model content").unwrap();

        let config = LLMConfig {
            model_path: model_path.to_string_lossy().to_string(),
            ..Default::default()
        };

        let provider = LlamaCppProvider::new(config.clone()).await.unwrap();
        let mut model = provider.load_model_with_config(config).await.unwrap();

        assert!(model.is_ready());
        assert!(model.get_memory_usage().unwrap() > 0);

        model.unload().await.unwrap();
        assert!(!model.is_ready());
        assert_eq!(model.get_memory_usage().unwrap(), 0);
    }

    #[test]
    fn test_provider_info() {
        let provider = LlamaCppProvider {
            config: LLMConfig::default(),
        };

        let info = provider.get_provider_info();
        assert!(info.contains("Mock LlamaCpp Provider"));
    }

    #[tokio::test]
    async fn test_list_available_models() {
        let provider = LlamaCppProvider {
            config: LLMConfig::default(),
        };

        let models = provider.list_available_models().await.unwrap();
        assert!(models.is_empty()); // Default implementation returns empty list
    }

    #[test]
    fn test_mock_response_generation() {
        let model = LlamaCppModel {
            config: LLMConfig::default(),
            info: ModelInfo {
                name: "test".to_string(),
                version: None,
                architecture: None,
                parameter_count: None,
                context_length: None,
                capabilities: ModelCapabilities::default(),
            },
            ready: true,
            memory_usage: 0,
        };

        // Test different prompt types
        assert!(model.generate_mock_response("Hello").contains("Hello"));
        assert!(
            model
                .generate_mock_response("What is your name?")
                .contains("mock implementation")
        );
        assert!(
            model
                .generate_mock_response("Show me some code")
                .contains("rust")
        );
        assert!(
            model
                .generate_mock_response("Explain quantum physics")
                .contains("explain")
        );
    }
}
