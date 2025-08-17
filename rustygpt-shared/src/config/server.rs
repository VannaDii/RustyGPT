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
            config.llm = file_config.llm;
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
    pub fn get_chat_llm_config(&self) -> Result<crate::llms::types::LLMConfig, String> {
        self.llm.get_default_chat_config()
    }

    /// Get the default LLM configuration for embeddings
    pub fn get_embedding_llm_config(&self) -> Result<crate::llms::types::LLMConfig, String> {
        self.llm.get_default_embedding_config()
    }

    /// Get LLM configuration for a specific model
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
