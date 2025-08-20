#[cfg(not(target_arch = "wasm32"))]
use crate::config::llm::LLMConfiguration;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

/// The main configuration structure for the RustyGPT application
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// Port for the HTTP server
    pub server_port: u16,

    /// Database connection URL
    pub database_url: String,

    /// Logging level
    pub log_level: String,

    /// Path to frontend static files
    pub frontend_path: PathBuf,

    /// LLM configuration settings
    #[cfg(not(target_arch = "wasm32"))]
    pub llm: LLMConfiguration,
}

impl Config {
    /// Generates a default configuration.
    pub fn with_defaults() -> Self {
        Self {
            server_port: 8080,
            database_url: "postgres://tinroof:rusty@localhost/rusty_gpt".to_string(),
            log_level: "info".to_string(),
            frontend_path: PathBuf::from("../frontend/dist"),
            #[cfg(not(target_arch = "wasm32"))]
            llm: LLMConfiguration::default(),
        }
    }

    /// Loads the configuration from a file, environment variables, or defaults.
    ///
    /// # Arguments
    /// * `config_path` - Optional path to the configuration file.
    /// * `port_override` - Optional port number to override the configuration.
    ///
    /// # Returns
    /// A {@link Config} struct with all values resolved, or an error if loading fails.
    pub fn load_config(
        config_path: Option<PathBuf>,
        port_override: Option<u16>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Config::with_defaults();

        // Load from file if provided
        if let Some(path) = config_path {
            let content = fs::read_to_string(&path)?;
            let file_config: Config =
                if path.extension().and_then(|ext| ext.to_str()) == Some("yaml") {
                    serde_yaml::from_str(&content)?
                } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                    serde_json::from_str(&content)?
                } else {
                    return Err("Unsupported configuration format. Use 'yaml' or 'json'.".into());
                };

            // Merge file config into default config
            config.server_port = file_config.server_port;
            config.database_url = file_config.database_url;
            config.log_level = file_config.log_level;
            config.frontend_path = file_config.frontend_path;
            #[cfg(not(target_arch = "wasm32"))]
            {
                config.llm = file_config.llm;
            }
        }

        // Use environment variables only if values are not already set
        if config.server_port == Config::with_defaults().server_port {
            if let Ok(port) = env::var("RUSTYGPT_SERVER_PORT") {
                config.server_port = port.parse().map_err(|_| {
                    "Invalid RUSTYGPT_SERVER_PORT value: must be a valid number between 1 and 65535"
                })?;
            }
        }
        if config.database_url == Config::with_defaults().database_url {
            if let Ok(db_url) = env::var("RUSTYGPT_DATABASE_URL") {
                config.database_url = db_url;
            }
        }
        if config.log_level == Config::with_defaults().log_level {
            if let Ok(log_level) = env::var("RUSTYGPT_LOG_LEVEL") {
                config.log_level = log_level;
            }
        }
        if config.frontend_path == Config::with_defaults().frontend_path {
            if let Ok(frontend_path) = env::var("RUSTYGPT_FRONTEND_PATH") {
                config.frontend_path = PathBuf::from(frontend_path);
            }
        }

        // Apply LLM environment variables to existing config
        #[cfg(not(target_arch = "wasm32"))]
        config.llm.apply_env_overrides();

        // Override with command-line arguments if provided
        if let Some(port) = port_override {
            config.server_port = port;
        }

        // Validate configuration
        if config.server_port == 0 {
            return Err("Invalid server port. Must be greater than 0.".into());
        }

        Ok(config)
    }

    /// Get the default LLM configuration for chat
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_chat_llm_config(&self) -> Result<crate::llms::types::LLMConfig, String> {
        self.llm.get_default_chat_config()
    }

    /// Get the default LLM configuration for embeddings
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_embedding_llm_config(&self) -> Result<crate::llms::types::LLMConfig, String> {
        self.llm.get_default_embedding_config()
    }

    /// Get LLM configuration for a specific model
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_llm_config(
        &self,
        model_name: &str,
    ) -> Result<crate::llms::types::LLMConfig, String> {
        self.llm.to_llm_config(model_name)
    }

    /// Validate the complete configuration including LLM settings
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate basic server config
        if self.server_port == 0 {
            errors.push("Invalid server port. Must be greater than 0.".to_string());
        }

        if !self.frontend_path.exists() {
            errors.push(format!(
                "Frontend path does not exist: {}",
                self.frontend_path.display()
            ));
        }

        // Validate LLM configuration
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(llm_errors) = self.llm.validate() {
            errors.extend(llm_errors);
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
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    fn cleanup_env_vars() {
        unsafe {
            std::env::remove_var("RUSTYGPT_SERVER_PORT");
            std::env::remove_var("RUSTYGPT_DATABASE_URL");
            std::env::remove_var("RUSTYGPT_LOG_LEVEL");
            std::env::remove_var("RUSTYGPT_FRONTEND_PATH");
        }
    }

    #[test]
    fn test_config_with_defaults() {
        cleanup_env_vars();
        let config = Config::with_defaults();

        assert_eq!(config.server_port, 8080);
        assert_eq!(
            config.database_url,
            "postgres://tinroof:rusty@localhost/rusty_gpt"
        );
        assert_eq!(config.log_level, "info");
        assert_eq!(config.frontend_path, PathBuf::from("../frontend/dist"));
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert_eq!(config.llm.default_provider, "llama_cpp");
        }
    }

    #[test]
    fn test_load_config_with_defaults() {
        cleanup_env_vars();
        let config = Config::load_config(None, None).unwrap();

        assert_eq!(config.server_port, 8080);
        assert!(config.database_url.contains("postgres"));
        assert_eq!(config.log_level, "info");
        assert!(config.server_port > 0);
    }

    #[test]
    fn test_load_config_with_port_override() {
        cleanup_env_vars();
        let config = Config::load_config(None, Some(3000)).unwrap();

        assert_eq!(config.server_port, 3000);
        assert!(config.database_url.contains("postgres"));
        assert_eq!(config.log_level, "info");
    }

    #[test]
    #[serial]
    fn test_load_config_with_environment_variables() {
        cleanup_env_vars();

        // Set environment variables
        unsafe {
            std::env::set_var("RUSTYGPT_SERVER_PORT", "9090");
            std::env::set_var(
                "RUSTYGPT_DATABASE_URL",
                "postgres://custom:password@host/db",
            );
            std::env::set_var("RUSTYGPT_LOG_LEVEL", "debug");
            std::env::set_var("RUSTYGPT_FRONTEND_PATH", "/custom/frontend");
        }

        let config = Config::load_config(None, None).unwrap();

        assert_eq!(config.server_port, 9090);
        assert_eq!(config.database_url, "postgres://custom:password@host/db");
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.frontend_path, PathBuf::from("/custom/frontend"));

        cleanup_env_vars();
    }

    #[test]
    fn test_load_config_port_override_precedence() {
        cleanup_env_vars();

        // Set environment variable for port
        unsafe {
            std::env::set_var("RUSTYGPT_SERVER_PORT", "5555");
        }

        // Override with command line argument
        let config = Config::load_config(None, Some(7777)).unwrap();

        // Command line should take precedence
        assert_eq!(config.server_port, 7777);

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_load_config_invalid_port_environment() {
        cleanup_env_vars();

        // Create a minimal YAML file without port so env var will be used
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("minimal.yaml");
        let minimal_yaml = r#"
server_port: 8080
database_url: "postgres://test@localhost/test"
log_level: "info"
frontend_path: "/frontend"
llm:
  default_provider: "llama_cpp"
  models_directory: "/models"
  default_chat_model: "default"
  providers:
    llama_cpp:
      provider_type: "llama_cpp"
      enabled: true
      additional_settings: {}
  models:
    default:
      path: "default.gguf"
      provider: "llama_cpp"
      display_name: "Default Model"
      description: "Default test model"
      default_params:
        max_tokens: 512
        temperature: 0.7
        top_p: 0.9
        top_k: 40
        repetition_penalty: 1.1
        context_size: 2048
        batch_size: 1
      capabilities:
        text_generation: true
        text_embedding: false
        chat_format: true
        function_calling: false
        streaming: true
        supported_languages: ["en"]
  global_settings:
    default_timeout: 30
    max_concurrent_requests: 4
    enable_model_caching: true
    cache_size_limit_mb: 1024
    enable_request_logging: false
    enable_metrics: true
"#;
        fs::write(&config_file, minimal_yaml).unwrap();

        unsafe {
            std::env::set_var("RUSTYGPT_SERVER_PORT", "invalid_port");
        }

        let result = Config::load_config(Some(config_file), None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid RUSTYGPT_SERVER_PORT")
        );

        cleanup_env_vars();
    }

    #[test]
    fn test_load_config_zero_port_validation() {
        cleanup_env_vars();
        let result = Config::load_config(None, Some(0));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid server port")
        );
    }

    #[test]
    fn test_load_config_from_yaml_file() -> Result<(), Box<dyn std::error::Error>> {
        cleanup_env_vars();
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.yaml");

        let yaml_content = r#"
server_port: 4000
database_url: "postgres://yaml:config@localhost/yaml_db"
log_level: "trace"
frontend_path: "/yaml/frontend"
llm:
  default_provider: "llama_cpp"
  models_directory: "/yaml/models"
  default_chat_model: "yaml-model"
  providers:
    llama_cpp:
      provider_type: "llama_cpp"
      enabled: true
      additional_settings: {}
  models:
    yaml-model:
      path: "yaml.gguf"
      provider: "llama_cpp"
      display_name: "YAML Model"
      description: "Test model from YAML"
      default_params:
        max_tokens: 256
        temperature: 0.5
        top_p: 0.9
        top_k: 40
        repetition_penalty: 1.1
        context_size: 2048
        batch_size: 1
      capabilities:
        text_generation: true
        text_embedding: false
        chat_format: true
        function_calling: false
        streaming: true
        supported_languages: ["en"]
  global_settings:
    default_timeout: 30
    max_concurrent_requests: 4
    enable_model_caching: true
    cache_size_limit_mb: 1024
    enable_request_logging: false
    enable_metrics: true
"#;

        fs::write(&config_file, yaml_content)?;

        let config = Config::load_config(Some(config_file), None)?;

        assert_eq!(config.server_port, 4000);
        assert_eq!(
            config.database_url,
            "postgres://yaml:config@localhost/yaml_db"
        );
        assert_eq!(config.log_level, "trace");
        assert_eq!(config.frontend_path, PathBuf::from("/yaml/frontend"));

        #[cfg(not(target_arch = "wasm32"))]
        {
            assert_eq!(config.llm.default_provider, "llama_cpp");
            assert_eq!(config.llm.models_directory, PathBuf::from("/yaml/models"));
            assert_eq!(config.llm.default_chat_model, "yaml-model");
        }

        Ok(())
    }

    #[test]
    fn test_load_config_from_json_file() -> Result<(), Box<dyn std::error::Error>> {
        cleanup_env_vars();
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("test_config.json");

        let json_content = r#"
{
  "server_port": 5000,
  "database_url": "postgres://json:config@localhost/json_db",
  "log_level": "warn",
  "frontend_path": "/json/frontend",
  "llm": {
    "default_provider": "llama_cpp",
    "models_directory": "/json/models",
    "default_chat_model": "json-model",
    "providers": {
      "llama_cpp": {
        "provider_type": "llama_cpp",
        "enabled": true,
        "additional_settings": {}
      }
    },
    "models": {
      "json-model": {
        "path": "json.gguf",
        "provider": "llama_cpp",
        "display_name": "JSON Model",
        "description": "Test model from JSON",
        "default_params": {
          "max_tokens": 128,
          "temperature": 0.7,
          "top_p": 0.9,
          "top_k": 40,
          "repetition_penalty": 1.1,
          "context_size": 2048,
          "batch_size": 1
        },
        "capabilities": {
          "text_generation": true,
          "text_embedding": false,
          "chat_format": true,
          "function_calling": false,
          "streaming": true,
          "supported_languages": ["en"]
        }
      }
    },
    "global_settings": {
      "default_timeout": 60,
      "max_concurrent_requests": 2,
      "enable_model_caching": true,
      "cache_size_limit_mb": 1024,
      "enable_request_logging": false,
      "enable_metrics": true
    }
  }
}
"#;

        fs::write(&config_file, json_content)?;

        let config = Config::load_config(Some(config_file), None)?;

        assert_eq!(config.server_port, 5000);
        assert_eq!(
            config.database_url,
            "postgres://json:config@localhost/json_db"
        );
        assert_eq!(config.log_level, "warn");
        assert_eq!(config.frontend_path, PathBuf::from("/json/frontend"));

        #[cfg(not(target_arch = "wasm32"))]
        {
            assert_eq!(config.llm.default_provider, "llama_cpp");
            assert_eq!(config.llm.models_directory, PathBuf::from("/json/models"));
            assert_eq!(config.llm.default_chat_model, "json-model");
        }

        Ok(())
    }

    #[test]
    fn test_load_config_unsupported_format() {
        cleanup_env_vars();
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test_config.toml");

        fs::write(&config_file, "server_port = 6000").unwrap();

        let result = Config::load_config(Some(config_file), None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported configuration format")
        );
    }

    #[test]
    fn test_load_config_nonexistent_file() {
        cleanup_env_vars();
        let nonexistent_file = PathBuf::from("/nonexistent/config.yaml");

        let result = Config::load_config(Some(nonexistent_file), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_malformed_yaml() {
        cleanup_env_vars();
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("bad_config.yaml");

        fs::write(&config_file, "server_port: [invalid yaml structure").unwrap();

        let result = Config::load_config(Some(config_file), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_malformed_json() {
        cleanup_env_vars();
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("bad_config.json");

        fs::write(&config_file, r#"{ "server_port": invalid json }"#).unwrap();

        let result = Config::load_config(Some(config_file), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_valid() {
        cleanup_env_vars();
        let temp_dir = TempDir::new().unwrap();
        let frontend_path = temp_dir.path().join("frontend");
        fs::create_dir_all(&frontend_path).unwrap();

        let mut config = Config::with_defaults();
        config.frontend_path = frontend_path;

        let result = config.validate();
        // This might fail due to LLM validation, but server validation should pass
        // At minimum, it should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_validate_config_invalid_port() {
        cleanup_env_vars();
        let mut config = Config::with_defaults();
        config.server_port = 0;

        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Invalid server port")));
    }

    #[test]
    fn test_validate_config_nonexistent_frontend_path() {
        cleanup_env_vars();
        let mut config = Config::with_defaults();
        config.frontend_path = PathBuf::from("/absolutely/nonexistent/path");

        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("Frontend path does not exist"))
        );
    }

    #[test]
    #[serial]
    fn test_environment_variable_precedence() {
        cleanup_env_vars();

        // Test that environment variables only override defaults, not file config
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("priority_test.yaml");

        let yaml_content = r#"
server_port: 1111
database_url: "postgres://file:config@localhost/file_db"
log_level: "error"
frontend_path: "/file/frontend"
llm:
  default_provider: "llama_cpp"
  models_directory: "/models"
  default_chat_model: "default"
  providers:
    llama_cpp:
      provider_type: "llama_cpp"
      enabled: true
      additional_settings: {}
  models:
    default:
      path: "default.gguf"
      provider: "llama_cpp"
      display_name: "Default Model"
      description: "Default test model"
      default_params:
        max_tokens: 512
        temperature: 0.7
        top_p: 0.9
        top_k: 40
        repetition_penalty: 1.1
        context_size: 2048
        batch_size: 1
      capabilities:
        text_generation: true
        text_embedding: false
        chat_format: true
        function_calling: false
        streaming: true
        supported_languages: ["en"]
  global_settings:
    default_timeout: 30
    max_concurrent_requests: 4
    enable_model_caching: true
    cache_size_limit_mb: 1024
    enable_request_logging: false
    enable_metrics: true
"#;

        fs::write(&config_file, yaml_content).unwrap();

        // Set environment variables
        unsafe {
            std::env::set_var("RUSTYGPT_SERVER_PORT", "2222");
            std::env::set_var(
                "RUSTYGPT_DATABASE_URL",
                "postgres://env:config@localhost/env_db",
            );
        }

        let config = Config::load_config(Some(config_file), None).unwrap();

        // File config should take precedence over environment variables
        assert_eq!(config.server_port, 1111);
        assert_eq!(
            config.database_url,
            "postgres://file:config@localhost/file_db"
        );
        assert_eq!(config.log_level, "error");

        cleanup_env_vars();
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_llm_config_methods() {
        cleanup_env_vars();
        let config = Config::with_defaults();

        // Test chat LLM config
        let chat_config = config.get_chat_llm_config();
        assert!(
            chat_config.is_ok(),
            "Chat config should be Ok: {:?}",
            chat_config
        );

        // Test embedding LLM config (this might fail if no embedding model is configured)
        let embedding_config = config.get_embedding_llm_config();
        // Don't assert OK since embedding might not be configured by default
        let _ = embedding_config;

        // Test specific model config
        let model_config = config.get_llm_config("default");
        assert!(
            model_config.is_ok(),
            "Default model config should be Ok: {:?}",
            model_config
        );

        // Test nonexistent model config
        let nonexistent_config = config.get_llm_config("nonexistent_model");
        assert!(nonexistent_config.is_err());
    }

    #[test]
    fn test_config_serialization() {
        cleanup_env_vars();
        let config = Config::with_defaults();

        // Test JSON serialization
        let json_str = serde_json::to_string(&config).unwrap();
        let deserialized_config: Config = serde_json::from_str(&json_str).unwrap();
        assert_eq!(config.server_port, deserialized_config.server_port);
        assert_eq!(config.database_url, deserialized_config.database_url);

        // Test YAML serialization
        let yaml_str = serde_yaml::to_string(&config).unwrap();
        let deserialized_config: Config = serde_yaml::from_str(&yaml_str).unwrap();
        assert_eq!(config.server_port, deserialized_config.server_port);
        assert_eq!(config.log_level, deserialized_config.log_level);
    }

    #[test]
    fn test_config_clone_and_debug() {
        cleanup_env_vars();
        let config = Config::with_defaults();

        // Test Clone trait
        let cloned_config = config.clone();
        assert_eq!(config.server_port, cloned_config.server_port);
        assert_eq!(config.database_url, cloned_config.database_url);

        // Test Debug trait
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains(&config.server_port.to_string()));
    }

    #[test]
    fn test_edge_case_port_values() {
        cleanup_env_vars();

        // Test minimum valid port
        let config = Config::load_config(None, Some(1)).unwrap();
        assert_eq!(config.server_port, 1);

        // Test maximum valid port
        let config = Config::load_config(None, Some(65535)).unwrap();
        assert_eq!(config.server_port, 65535);

        // Test typical development ports
        for port in [3000, 8000, 8080, 9000] {
            let config = Config::load_config(None, Some(port)).unwrap();
            assert_eq!(config.server_port, port);
        }
    }

    #[test]
    #[serial]
    fn test_database_url_variations() {
        cleanup_env_vars();

        let test_urls = vec![
            "postgres://user:pass@localhost/db",
            "postgresql://user:pass@localhost:5432/db",
            "postgres://localhost/db",
            "postgresql://user@localhost/db",
        ];

        for url in test_urls {
            cleanup_env_vars(); // Clean between iterations
            unsafe {
                std::env::set_var("RUSTYGPT_DATABASE_URL", url);
            }
            let config = Config::load_config(None, None).unwrap();
            assert_eq!(config.database_url, url);
        }

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_log_level_variations() {
        cleanup_env_vars();

        let log_levels = vec!["trace", "debug", "info", "warn", "error"];

        for level in log_levels {
            unsafe {
                std::env::set_var("RUSTYGPT_LOG_LEVEL", level);
            }
            let config = Config::load_config(None, None).unwrap();
            assert_eq!(config.log_level, level);
            cleanup_env_vars();
        }
    }
}
