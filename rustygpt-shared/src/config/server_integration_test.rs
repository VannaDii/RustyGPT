//! Test to verify the server configuration integration works correctly

#[cfg(test)]
mod tests {
    use crate::config::server::Config;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_server_config_compatibility() {
        // Clean up any existing environment variables first
        unsafe {
            std::env::remove_var("RUSTYGPT_SERVER_PORT");
            std::env::remove_var("RUSTYGPT_LOG_LEVEL");
            std::env::remove_var("RUSTYGPT_MODELS_DIR");
            std::env::remove_var("RUSTYGPT_GPU_LAYERS");
        }

        // Test that the server can load the default configuration
        let config = Config::with_defaults();

        // Verify all fields are accessible
        assert_eq!(config.server_port, 8080);
        assert!(config.database_url.contains("postgres"));
        assert_eq!(config.log_level, "info");
        assert!(config.frontend_path.to_string_lossy().contains("frontend"));

        // Verify LLM configuration is accessible
        assert_eq!(config.llm.default_provider, "llama_cpp");
        assert!(config.llm.providers.contains_key("llama_cpp"));
        assert!(config.llm.models.contains_key("default"));

        // Test LLM configuration methods
        let chat_config = config.get_chat_llm_config().unwrap();
        assert!(chat_config.model_path.contains("default.gguf"));

        // Test that we can get a specific model config
        let model_config = config.get_llm_config("default").unwrap();
        assert_eq!(model_config.max_tokens, Some(512));
    }

    #[test]
    fn test_config_loading_with_port_override() {
        // Clean up any existing environment variables first
        unsafe {
            std::env::remove_var("RUSTYGPT_SERVER_PORT");
            std::env::remove_var("RUSTYGPT_LOG_LEVEL");
            std::env::remove_var("RUSTYGPT_MODELS_DIR");
            std::env::remove_var("RUSTYGPT_GPU_LAYERS");
        }

        // Test the load_config method that the server uses
        let config = Config::load_config(None, Some(3000)).unwrap();

        // Port should be overridden
        assert_eq!(config.server_port, 3000);

        // Other values should be defaults
        assert!(config.database_url.contains("postgres"));
        assert_eq!(config.log_level, "info");

        // LLM config should be loaded
        assert_eq!(config.llm.default_provider, "llama_cpp");
    }

    #[test]
    fn test_config_loading_from_yaml_file() {
        // Clean up any existing environment variables first that might interfere
        unsafe {
            std::env::remove_var("RUSTYGPT_DEFAULT_PROVIDER");
            std::env::remove_var("RUSTYGPT_DEFAULT_MODEL");
            std::env::remove_var("RUSTYGPT_MODELS_DIR");
            std::env::remove_var("RUSTYGPT_GPU_LAYERS");
            std::env::remove_var("RUSTYGPT_THREADS");
            std::env::remove_var("RUSTYGPT_LLM_TIMEOUT");
            std::env::remove_var("RUSTYGPT_MAX_CONCURRENT_REQUESTS");
        }

        let temp_dir = tempdir().unwrap();
        let config_file = temp_dir.path().join("test_config.yaml");

        // Create a test configuration file
        let yaml_content = r#"
server_port: 9000
database_url: "postgres://test:test@localhost/test_db"
log_level: "debug"
frontend_path: "/custom/frontend/path"
llm:
  default_provider: "llama_cpp"
  models_directory: "/custom/models"
  default_chat_model: "custom-model"
  providers:
    llama_cpp:
      provider_type: "llama_cpp"
      enabled: true
      n_gpu_layers: 5
      n_threads: 8
      additional_settings: {}
  models:
    custom-model:
      path: "custom.gguf"
      provider: "llama_cpp"
      display_name: "Custom Model"
      description: "A custom test model"
      default_params:
        max_tokens: 1024
        temperature: 0.8
        top_p: 0.95
        top_k: 50
        repetition_penalty: 1.05
        context_size: 4096
        batch_size: 256
      capabilities:
        text_generation: true
        text_embedding: false
        chat_format: true
        function_calling: false
        streaming: true
        supported_languages: ["en"]
  global_settings:
    default_timeout: 45
    max_concurrent_requests: 8
    enable_model_caching: true
    cache_size_limit_mb: 8192
    enable_request_logging: true
    enable_metrics: true
"#;

        std::fs::write(&config_file, yaml_content).unwrap();

        // Load configuration from file
        let config = Config::load_config(Some(config_file), None).unwrap();

        // Verify server configuration was loaded
        assert_eq!(config.server_port, 9000);
        assert_eq!(
            config.database_url,
            "postgres://test:test@localhost/test_db"
        );
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.frontend_path, PathBuf::from("/custom/frontend/path"));

        // Verify LLM configuration was loaded
        assert_eq!(config.llm.default_provider, "llama_cpp");
        assert_eq!(config.llm.models_directory, PathBuf::from("/custom/models"));
        assert_eq!(config.llm.default_chat_model, "custom-model");

        // Verify provider settings
        let llama_config = config.llm.get_provider_config("llama_cpp").unwrap();
        assert_eq!(llama_config.n_gpu_layers, Some(5));
        assert_eq!(llama_config.n_threads, Some(8));

        // Verify model settings
        let model_config = config.llm.get_model_config("custom-model").unwrap();
        assert_eq!(model_config.path, "custom.gguf");
        assert_eq!(model_config.display_name, "Custom Model");
        assert_eq!(model_config.default_params.max_tokens, 1024);

        // Verify global settings
        assert_eq!(config.llm.global_settings.default_timeout, 45);
        assert_eq!(config.llm.global_settings.max_concurrent_requests, 8);

        // Test LLM config conversion
        let llm_config = config.get_llm_config("custom-model").unwrap();
        assert!(llm_config.model_path.contains("custom.gguf"));
        assert_eq!(llm_config.max_tokens, Some(1024));
        assert_eq!(llm_config.temperature, Some(0.8));
        assert_eq!(llm_config.n_gpu_layers, Some(5));
        assert_eq!(llm_config.n_threads, Some(8));
    }
}
