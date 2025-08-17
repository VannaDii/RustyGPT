//! # Configuration Integration Tests
//!
//! Tests that verify the complete configuration system works with LLM settings.

#[cfg(test)]
mod tests {
    use crate::config::{
        llm::{LLMConfiguration, ModelCapabilities, ModelConfig, ModelParameters, ProviderConfig},
        server::Config,
    };
    use std::{collections::HashMap, env};
    use tempfile::tempdir;

    #[test]
    fn test_default_config_includes_llm() {
        let config = Config::with_defaults();

        // Verify LLM configuration is included
        assert_eq!(config.llm.default_provider, "llama_cpp");
        assert!(config.llm.providers.contains_key("llama_cpp"));
        assert!(config.llm.models.contains_key("default"));

        // Verify we can get LLM configs
        let chat_config = config.get_chat_llm_config().unwrap();
        assert!(chat_config.model_path.contains("default.gguf"));
        assert_eq!(chat_config.max_tokens, Some(512));
    }

    #[test]
    fn test_config_with_custom_llm_settings() {
        let mut config = Config::with_defaults();

        // Add a custom model
        let custom_model = ModelConfig {
            path: "custom-model.gguf".to_string(),
            provider: "llama_cpp".to_string(),
            display_name: "Custom Model".to_string(),
            description: Some("A custom model for testing".to_string()),
            default_params: ModelParameters {
                max_tokens: 1024,
                temperature: 0.8,
                top_p: 0.95,
                top_k: 50,
                repetition_penalty: 1.05,
                context_size: 4096,
                batch_size: 256,
            },
            capabilities: ModelCapabilities {
                text_generation: true,
                text_embedding: false,
                chat_format: true,
                function_calling: true,
                streaming: true,
                supported_languages: vec!["en".to_string(), "es".to_string()],
            },
        };

        config.llm.add_model("custom".to_string(), custom_model);

        // Test getting the custom model config
        let custom_config = config.get_llm_config("custom").unwrap();
        assert!(custom_config.model_path.contains("custom-model.gguf"));
        assert_eq!(custom_config.max_tokens, Some(1024));
        assert_eq!(custom_config.temperature, Some(0.8));
    }

    #[test]
    fn test_config_with_multiple_providers() {
        let mut config = Config::with_defaults();

        // Add a Candle provider
        let candle_provider = ProviderConfig {
            provider_type: "candle".to_string(),
            enabled: true,
            n_gpu_layers: Some(10),
            n_threads: Some(8),
            additional_settings: {
                let mut settings = HashMap::new();
                settings.insert(
                    "use_flash_attention".to_string(),
                    serde_json::Value::Bool(true),
                );
                settings
            },
        };

        config
            .llm
            .add_provider("candle".to_string(), candle_provider);

        // Add a model that uses Candle
        let candle_model = ModelConfig {
            path: "candle-model.safetensors".to_string(),
            provider: "candle".to_string(),
            display_name: "Candle Model".to_string(),
            description: Some("A model using the Candle framework".to_string()),
            default_params: ModelParameters::default(),
            capabilities: ModelCapabilities::default(),
        };

        config
            .llm
            .add_model("candle-model".to_string(), candle_model);

        // Test getting the Candle model config
        let candle_config = config.get_llm_config("candle-model").unwrap();
        assert!(
            candle_config
                .model_path
                .contains("candle-model.safetensors")
        );
        assert_eq!(candle_config.n_gpu_layers, Some(10));
        assert_eq!(candle_config.n_threads, Some(8));

        // Verify additional settings
        assert!(
            candle_config
                .additional_params
                .contains_key("use_flash_attention")
        );
    }

    #[test]
    fn test_config_yaml_serialization() {
        let config = Config::with_defaults();

        // Test that we can serialize the config to YAML
        let yaml_string = serde_yaml::to_string(&config).unwrap();
        assert!(yaml_string.contains("server_port"));
        assert!(yaml_string.contains("llm"));
        assert!(yaml_string.contains("default_provider"));
        assert!(yaml_string.contains("providers"));
        assert!(yaml_string.contains("models"));

        // Test that we can deserialize it back
        let deserialized: Config = serde_yaml::from_str(&yaml_string).unwrap();
        assert_eq!(deserialized.server_port, config.server_port);
        assert_eq!(
            deserialized.llm.default_provider,
            config.llm.default_provider
        );
    }

    #[test]
    fn test_config_json_serialization() {
        let config = Config::with_defaults();

        // Test that we can serialize the config to JSON
        let json_string = serde_json::to_string_pretty(&config).unwrap();
        assert!(json_string.contains("server_port"));
        assert!(json_string.contains("llm"));

        // Test that we can deserialize it back
        let deserialized: Config = serde_json::from_str(&json_string).unwrap();
        assert_eq!(deserialized.server_port, config.server_port);
        assert_eq!(
            deserialized.llm.default_provider,
            config.llm.default_provider
        );
    }

    #[test]
    fn test_env_var_integration() {
        // Test loading configuration with environment variables
        let temp_dir = tempdir().unwrap();
        let models_dir = temp_dir.path().join("models");
        std::fs::create_dir_all(&models_dir).unwrap();

        // Create a test model file
        let model_file = models_dir.join("test-model.gguf");
        std::fs::write(&model_file, "mock model content").unwrap();

        // Set environment variables
        unsafe {
            env::set_var("RUSTYGPT_MODELS_DIR", models_dir.to_str().unwrap());
            env::set_var("RUSTYGPT_DEFAULT_PROVIDER", "test_provider");
            env::set_var("RUSTYGPT_DEFAULT_MODEL", "test_model");
            env::set_var("RUSTYGPT_GPU_LAYERS", "5");
            env::set_var("RUSTYGPT_THREADS", "8");
            env::set_var("RUSTYGPT_LLM_TIMEOUT", "60");
            env::set_var("RUSTYGPT_MAX_CONCURRENT_REQUESTS", "8");
        }

        // Load LLM configuration from environment
        let llm_config = LLMConfiguration::load_from_env();

        assert_eq!(llm_config.models_directory, models_dir);
        assert_eq!(llm_config.default_provider, "test_provider");
        assert_eq!(llm_config.default_chat_model, "test_model");
        assert_eq!(llm_config.global_settings.default_timeout, 60);
        assert_eq!(llm_config.global_settings.max_concurrent_requests, 8);

        // Check provider-specific settings
        if let Some(provider_config) = llm_config.get_provider_config("llama_cpp") {
            assert_eq!(provider_config.n_gpu_layers, Some(5));
            assert_eq!(provider_config.n_threads, Some(8));
        }

        // Clean up environment variables
        unsafe {
            env::remove_var("RUSTYGPT_MODELS_DIR");
            env::remove_var("RUSTYGPT_DEFAULT_PROVIDER");
            env::remove_var("RUSTYGPT_DEFAULT_MODEL");
            env::remove_var("RUSTYGPT_GPU_LAYERS");
            env::remove_var("RUSTYGPT_THREADS");
            env::remove_var("RUSTYGPT_LLM_TIMEOUT");
            env::remove_var("RUSTYGPT_MAX_CONCURRENT_REQUESTS");
        }
    }

    #[test]
    fn test_config_validation() {
        let temp_dir = tempdir().unwrap();
        let models_dir = temp_dir.path().join("models");
        let frontend_dir = temp_dir.path().join("frontend");
        std::fs::create_dir_all(&models_dir).unwrap();
        std::fs::create_dir_all(&frontend_dir).unwrap();

        // Create test model files
        let default_model = models_dir.join("default.gguf");
        std::fs::write(&default_model, "mock model content").unwrap();

        let mut config = Config::with_defaults();
        config.llm.models_directory = models_dir.clone();
        config.frontend_path = frontend_dir;

        // Update the default model path to match our temp directory structure
        if let Some(default_model_config) = config.llm.models.get_mut("default") {
            default_model_config.path = "default.gguf".to_string();
        }

        // This should pass validation
        let result = config.validate();
        if result.is_err() {
            println!("Validation errors: {:?}", result.as_ref().unwrap_err());
        }
        assert!(result.is_ok());

        // Test with invalid server port
        config.server_port = 0;
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Invalid server port")));
    }

    #[test]
    fn test_absolute_vs_relative_model_paths() {
        let temp_dir = tempdir().unwrap();
        let models_dir = temp_dir.path().join("models");
        std::fs::create_dir_all(&models_dir).unwrap();

        let mut config = Config::with_defaults();
        config.llm.models_directory = models_dir.clone();

        // Test relative path
        let relative_model = ModelConfig {
            path: "relative-model.gguf".to_string(),
            provider: "llama_cpp".to_string(),
            display_name: "Relative Model".to_string(),
            description: None,
            default_params: ModelParameters::default(),
            capabilities: ModelCapabilities::default(),
        };

        config.llm.add_model("relative".to_string(), relative_model);

        let relative_config = config.get_llm_config("relative").unwrap();
        assert!(
            relative_config
                .model_path
                .contains("models/relative-model.gguf")
        );

        // Test absolute path
        let absolute_path = temp_dir.path().join("absolute-model.gguf");
        let absolute_model = ModelConfig {
            path: absolute_path.to_string_lossy().to_string(),
            provider: "llama_cpp".to_string(),
            display_name: "Absolute Model".to_string(),
            description: None,
            default_params: ModelParameters::default(),
            capabilities: ModelCapabilities::default(),
        };

        config.llm.add_model("absolute".to_string(), absolute_model);

        let absolute_config = config.get_llm_config("absolute").unwrap();
        assert_eq!(absolute_config.model_path, absolute_path.to_string_lossy());
    }

    #[test]
    fn test_model_capabilities_integration() {
        let config = Config::with_defaults();

        // Test that we can access model capabilities through the config
        if let Some(model_config) = config.llm.get_model_config("default") {
            assert!(model_config.capabilities.text_generation);
            assert!(model_config.capabilities.streaming);
            assert!(model_config.capabilities.chat_format);
            assert!(
                model_config
                    .capabilities
                    .supported_languages
                    .contains(&"en".to_string())
            );
        } else {
            panic!("Default model not found");
        }
    }

    #[test]
    fn test_global_llm_settings() {
        let config = Config::with_defaults();
        let settings = &config.llm.global_settings;

        assert_eq!(settings.default_timeout, 30);
        assert_eq!(settings.max_concurrent_requests, 4);
        assert!(settings.enable_model_caching);
        assert_eq!(settings.cache_size_limit_mb, 4096);
        assert!(settings.enable_request_logging);
        assert!(settings.enable_metrics);
    }

    #[test]
    fn test_error_handling_for_missing_models() {
        let config = Config::with_defaults();

        // Test error when requesting non-existent model
        let result = config.get_llm_config("non-existent-model");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Model 'non-existent-model' not found")
        );

        // Test error when requesting non-existent embedding model
        let result = config.get_embedding_llm_config();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("No default embedding model configured")
        );
    }
}
