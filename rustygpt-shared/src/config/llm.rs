//! # LLM Configuration
//!
//! This module provides configuration structures for LLM providers and models.

use crate::llms::types::LLMConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, path::PathBuf};

/// Configuration for LLM providers and models
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LLMConfiguration {
    /// Default LLM provider to use
    pub default_provider: String,

    /// Models directory where model files are stored
    pub models_directory: PathBuf,

    /// Default model to use for chat
    pub default_chat_model: String,

    /// Default model to use for embeddings (if supported)
    pub default_embedding_model: Option<String>,

    /// Provider-specific configurations
    pub providers: HashMap<String, ProviderConfig>,

    /// Model-specific configurations
    pub models: HashMap<String, ModelConfig>,

    /// Global LLM settings
    pub global_settings: GlobalLLMSettings,
}

/// Configuration for a specific LLM provider
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProviderConfig {
    /// Provider type (e.g., "llama_cpp", "candle", "onnx")
    pub provider_type: String,

    /// Whether this provider is enabled
    pub enabled: bool,

    /// Number of GPU layers to use (if applicable)
    pub n_gpu_layers: Option<u32>,

    /// Number of CPU threads to use
    pub n_threads: Option<u32>,

    /// Additional provider-specific settings
    pub additional_settings: HashMap<String, serde_json::Value>,
}

/// Configuration for a specific model
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelConfig {
    /// Path to the model file (relative to models_directory or absolute)
    pub path: String,

    /// Provider to use for this model
    pub provider: String,

    /// Model display name
    pub display_name: String,

    /// Model description
    pub description: Option<String>,

    /// Default generation parameters
    pub default_params: ModelParameters,

    /// Model capabilities
    pub capabilities: ModelCapabilities,
}

/// Default parameters for text generation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelParameters {
    /// Maximum tokens to generate
    pub max_tokens: u32,

    /// Sampling temperature
    pub temperature: f32,

    /// Top-p sampling parameter
    pub top_p: f32,

    /// Top-k sampling parameter
    pub top_k: u32,

    /// Repetition penalty
    pub repetition_penalty: f32,

    /// Context window size
    pub context_size: u32,

    /// Batch size for processing
    pub batch_size: u32,
}

/// Model capabilities
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelCapabilities {
    /// Supports text generation
    pub text_generation: bool,

    /// Supports text embedding
    pub text_embedding: bool,

    /// Supports chat format
    pub chat_format: bool,

    /// Supports function calling
    pub function_calling: bool,

    /// Supports streaming
    pub streaming: bool,

    /// Supported languages (ISO codes)
    pub supported_languages: Vec<String>,
}

/// Global LLM settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GlobalLLMSettings {
    /// Default timeout for LLM operations (in seconds)
    pub default_timeout: u64,

    /// Maximum concurrent LLM requests
    pub max_concurrent_requests: u32,

    /// Enable model caching
    pub enable_model_caching: bool,

    /// Model cache size limit (in MB)
    pub cache_size_limit_mb: u64,

    /// Enable request logging
    pub enable_request_logging: bool,

    /// Enable performance metrics
    pub enable_metrics: bool,
}

impl Default for LLMConfiguration {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert(
            "llama_cpp".to_string(),
            ProviderConfig {
                provider_type: "llama_cpp".to_string(),
                enabled: true,
                n_gpu_layers: Some(0), // CPU-only by default
                n_threads: None,       // Use system default
                additional_settings: HashMap::new(),
            },
        );

        let mut models = HashMap::new();
        models.insert(
            "default".to_string(),
            ModelConfig {
                path: "models/default.gguf".to_string(),
                provider: "llama_cpp".to_string(),
                display_name: "Default Model".to_string(),
                description: Some("Default language model for general tasks".to_string()),
                default_params: ModelParameters::default(),
                capabilities: ModelCapabilities::default(),
            },
        );

        Self {
            default_provider: "llama_cpp".to_string(),
            models_directory: PathBuf::from("./models"),
            default_chat_model: "default".to_string(),
            default_embedding_model: None,
            providers,
            models,
            global_settings: GlobalLLMSettings::default(),
        }
    }
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            repetition_penalty: 1.1,
            context_size: 2048,
            batch_size: 512,
        }
    }
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            text_generation: true,
            text_embedding: false,
            chat_format: true,
            function_calling: false,
            streaming: true,
            supported_languages: vec!["en".to_string()],
        }
    }
}

impl Default for GlobalLLMSettings {
    fn default() -> Self {
        Self {
            default_timeout: 30, // 30 seconds
            max_concurrent_requests: 4,
            enable_model_caching: true,
            cache_size_limit_mb: 4096, // 4GB
            enable_request_logging: true,
            enable_metrics: true,
        }
    }
}

impl LLMConfiguration {
    /// Load LLM configuration from environment variables and defaults
    pub fn load_from_env() -> Self {
        let mut config = Self::default();

        // Load from environment variables
        if let Ok(models_dir) = env::var("RUSTYGPT_MODELS_DIR") {
            config.models_directory = PathBuf::from(models_dir);
        }

        if let Ok(default_provider) = env::var("RUSTYGPT_DEFAULT_PROVIDER") {
            config.default_provider = default_provider;
        }

        if let Ok(default_model) = env::var("RUSTYGPT_DEFAULT_MODEL") {
            config.default_chat_model = default_model;
        }

        if let Ok(gpu_layers) = env::var("RUSTYGPT_GPU_LAYERS") {
            if let Ok(layers) = gpu_layers.parse::<u32>() {
                if let Some(llama_config) = config.providers.get_mut("llama_cpp") {
                    llama_config.n_gpu_layers = Some(layers);
                }
            }
        }

        if let Ok(threads) = env::var("RUSTYGPT_THREADS") {
            if let Ok(thread_count) = threads.parse::<u32>() {
                if let Some(llama_config) = config.providers.get_mut("llama_cpp") {
                    llama_config.n_threads = Some(thread_count);
                }
            }
        }

        if let Ok(timeout) = env::var("RUSTYGPT_LLM_TIMEOUT") {
            if let Ok(timeout_secs) = timeout.parse::<u64>() {
                config.global_settings.default_timeout = timeout_secs;
            }
        }

        if let Ok(max_requests) = env::var("RUSTYGPT_MAX_CONCURRENT_REQUESTS") {
            if let Ok(max) = max_requests.parse::<u32>() {
                config.global_settings.max_concurrent_requests = max;
            }
        }

        config
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) {
        // Load from environment variables
        if let Ok(models_dir) = env::var("RUSTYGPT_MODELS_DIR") {
            self.models_directory = PathBuf::from(models_dir);
        }

        if let Ok(default_provider) = env::var("RUSTYGPT_DEFAULT_PROVIDER") {
            self.default_provider = default_provider;
        }

        if let Ok(default_model) = env::var("RUSTYGPT_DEFAULT_MODEL") {
            self.default_chat_model = default_model;
        }

        if let Ok(gpu_layers) = env::var("RUSTYGPT_GPU_LAYERS") {
            if let Ok(layers) = gpu_layers.parse::<u32>() {
                if let Some(llama_config) = self.providers.get_mut("llama_cpp") {
                    llama_config.n_gpu_layers = Some(layers);
                }
            }
        }

        if let Ok(threads) = env::var("RUSTYGPT_THREADS") {
            if let Ok(thread_count) = threads.parse::<u32>() {
                if let Some(llama_config) = self.providers.get_mut("llama_cpp") {
                    llama_config.n_threads = Some(thread_count);
                }
            }
        }

        if let Ok(timeout) = env::var("RUSTYGPT_LLM_TIMEOUT") {
            if let Ok(timeout_secs) = timeout.parse::<u64>() {
                self.global_settings.default_timeout = timeout_secs;
            }
        }

        if let Ok(max_requests) = env::var("RUSTYGPT_MAX_CONCURRENT_REQUESTS") {
            if let Ok(max) = max_requests.parse::<u32>() {
                self.global_settings.max_concurrent_requests = max;
            }
        }
    }

    /// Get configuration for a specific model
    pub fn get_model_config(&self, model_name: &str) -> Option<&ModelConfig> {
        self.models.get(model_name)
    }

    /// Get configuration for a specific provider
    pub fn get_provider_config(&self, provider_name: &str) -> Option<&ProviderConfig> {
        self.providers.get(provider_name)
    }

    /// Convert model configuration to LLMConfig for use with the trait system
    pub fn to_llm_config(&self, model_name: &str) -> Result<LLMConfig, String> {
        let model_config = self
            .get_model_config(model_name)
            .ok_or_else(|| format!("Model '{}' not found in configuration", model_name))?;

        let provider_config = self
            .get_provider_config(&model_config.provider)
            .ok_or_else(|| {
                format!(
                    "Provider '{}' not found in configuration",
                    model_config.provider
                )
            })?;

        // Resolve model path (absolute or relative to models_directory)
        let model_path = if PathBuf::from(&model_config.path).is_absolute() {
            model_config.path.clone()
        } else {
            self.models_directory
                .join(&model_config.path)
                .to_string_lossy()
                .to_string()
        };

        let mut additional_params = HashMap::new();
        for (key, value) in &provider_config.additional_settings {
            additional_params.insert(key.clone(), value.clone());
        }

        Ok(LLMConfig {
            model_path,
            max_tokens: Some(model_config.default_params.max_tokens),
            temperature: Some(model_config.default_params.temperature),
            top_p: Some(model_config.default_params.top_p),
            top_k: Some(model_config.default_params.top_k),
            repetition_penalty: Some(model_config.default_params.repetition_penalty),
            n_threads: provider_config.n_threads,
            n_gpu_layers: provider_config.n_gpu_layers,
            context_size: Some(model_config.default_params.context_size),
            batch_size: Some(model_config.default_params.batch_size),
            additional_params,
        })
    }

    /// Get the default chat model configuration
    pub fn get_default_chat_config(&self) -> Result<LLMConfig, String> {
        self.to_llm_config(&self.default_chat_model)
    }

    /// Get the default embedding model configuration (if available)
    pub fn get_default_embedding_config(&self) -> Result<LLMConfig, String> {
        let embedding_model = self
            .default_embedding_model
            .as_ref()
            .ok_or_else(|| "No default embedding model configured".to_string())?;

        self.to_llm_config(embedding_model)
    }

    /// Add a new model configuration
    pub fn add_model(&mut self, name: String, config: ModelConfig) {
        self.models.insert(name, config);
    }

    /// Add a new provider configuration
    pub fn add_provider(&mut self, name: String, config: ProviderConfig) {
        self.providers.insert(name, config);
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check if models directory exists or can be created
        if !self.models_directory.exists() {
            errors.push(format!(
                "Models directory does not exist: {}",
                self.models_directory.display()
            ));
        }

        // Check if default provider exists
        if !self.providers.contains_key(&self.default_provider) {
            errors.push(format!(
                "Default provider '{}' not found in providers configuration",
                self.default_provider
            ));
        }

        // Check if default chat model exists
        if !self.models.contains_key(&self.default_chat_model) {
            errors.push(format!(
                "Default chat model '{}' not found in models configuration",
                self.default_chat_model
            ));
        }

        // Check if embedding model exists (if configured)
        if let Some(ref embedding_model) = self.default_embedding_model {
            if !self.models.contains_key(embedding_model) {
                errors.push(format!(
                    "Default embedding model '{}' not found in models configuration",
                    embedding_model
                ));
            }
        }

        // Validate each model configuration
        for (model_name, model_config) in &self.models {
            if !self.providers.contains_key(&model_config.provider) {
                errors.push(format!(
                    "Model '{}' references unknown provider '{}'",
                    model_name, model_config.provider
                ));
            }

            // Check if model file path is reasonable
            let model_path = if PathBuf::from(&model_config.path).is_absolute() {
                PathBuf::from(&model_config.path)
            } else {
                self.models_directory.join(&model_config.path)
            };

            if !model_path.exists() {
                errors.push(format!(
                    "Model '{}' file does not exist: {}",
                    model_name,
                    model_path.display()
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_default_llm_configuration() {
        let config = LLMConfiguration::default();

        assert_eq!(config.default_provider, "llama_cpp");
        assert_eq!(config.default_chat_model, "default");
        assert!(config.providers.contains_key("llama_cpp"));
        assert!(config.models.contains_key("default"));
    }

    #[test]
    fn test_model_config_conversion() {
        let config = LLMConfiguration::default();

        let llm_config = config.to_llm_config("default").unwrap();
        assert!(llm_config.model_path.contains("default.gguf"));
        assert_eq!(llm_config.max_tokens, Some(512));
        assert_eq!(llm_config.temperature, Some(0.7));
    }

    #[test]
    fn test_get_default_chat_config() {
        let config = LLMConfiguration::default();

        let chat_config = config.get_default_chat_config().unwrap();
        assert!(chat_config.model_path.contains("default.gguf"));
    }

    #[test]
    fn test_validation_with_missing_files() {
        let config = LLMConfiguration::default();

        let result = config.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .any(|e| e.contains("Models directory does not exist"))
        );
    }

    #[test]
    fn test_validation_with_valid_setup() {
        let temp_dir = tempdir().unwrap();
        let models_dir = temp_dir.path().join("models");
        fs::create_dir_all(&models_dir).unwrap();

        let model_file = models_dir.join("default.gguf");
        fs::write(&model_file, "mock model content").unwrap();

        let mut config = LLMConfiguration::default();
        config.models_directory = models_dir;

        // Update the default model path to match our temp directory structure
        if let Some(default_model_config) = config.models.get_mut("default") {
            default_model_config.path = "default.gguf".to_string();
        }

        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_model_and_provider() {
        let mut config = LLMConfiguration::default();

        let new_provider = ProviderConfig {
            provider_type: "candle".to_string(),
            enabled: true,
            n_gpu_layers: Some(10),
            n_threads: Some(8),
            additional_settings: HashMap::new(),
        };

        let new_model = ModelConfig {
            path: "models/candle-model.bin".to_string(),
            provider: "candle".to_string(),
            display_name: "Candle Model".to_string(),
            description: Some("A Candle-based model".to_string()),
            default_params: ModelParameters::default(),
            capabilities: ModelCapabilities::default(),
        };

        config.add_provider("candle".to_string(), new_provider);
        config.add_model("candle-model".to_string(), new_model);

        assert!(config.providers.contains_key("candle"));
        assert!(config.models.contains_key("candle-model"));

        let llm_config = config.to_llm_config("candle-model").unwrap();
        assert!(llm_config.model_path.contains("candle-model.bin"));
    }

    #[test]
    fn test_env_loading() {
        // Note: This test doesn't actually set env vars to avoid affecting other tests
        // In a real scenario, you could use a mocking library or test with actual env vars
        let config = LLMConfiguration::load_from_env();

        // Should return default configuration when no env vars are set
        assert_eq!(config.default_provider, "llama_cpp");
        assert_eq!(config.models_directory, PathBuf::from("./models"));
    }
}
