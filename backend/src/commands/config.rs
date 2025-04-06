use serde::{Deserialize, Serialize};
use serde_json;
use serde_yaml;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Represents the configuration structure for RustyGPT.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub server_port: u16,
    pub database_url: String,
    pub log_level: String,
    pub frontend_path: String,
}

impl Config {
    /// Generates a default configuration.
    pub fn default() -> Self {
        Self {
            server_port: 8080,
            database_url: "postgres://localhost:5432/rustygpt".to_string(),
            log_level: "info".to_string(),
            frontend_path: "../frontend/dist".to_string(),
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
        let mut config = Config::default();

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
        }

        // Use environment variables only if values are not already set
        if config.server_port == Config::default().server_port {
            if let Ok(port) = env::var("RUSTYGPT_SERVER_PORT") {
                config.server_port = port.parse().map_err(|_| {
                    "Invalid RUSTYGPT_SERVER_PORT value: must be a valid number between 1 and 65535"
                })?;
            }
        }
        if config.database_url == Config::default().database_url {
            if let Ok(db_url) = env::var("RUSTYGPT_DATABASE_URL") {
                config.database_url = db_url;
            }
        }
        if config.log_level == Config::default().log_level {
            if let Ok(log_level) = env::var("RUSTYGPT_LOG_LEVEL") {
                config.log_level = log_level;
            }
        }
        if config.frontend_path == Config::default().frontend_path {
            if let Ok(frontend_path) = env::var("RUSTYGPT_FRONTEND_PATH") {
                config.frontend_path = frontend_path;
            }
        }

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
}

/// Generates a configuration file in the specified format.
///
/// # Arguments
/// * `format` - The format of the configuration file ("yaml" or "json").
///
/// # Errors
/// Returns an error if the format is unsupported or if writing the file fails.
pub fn generate_config(format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let file_name = match format {
        "yaml" => "config.yaml",
        "json" => "config.json",
        _ => return Err("Unsupported format. Use 'yaml' or 'json'.".into()),
    };

    let serialized = match format {
        "yaml" => serde_yaml::to_string(&config)?,
        "json" => serde_json::to_string_pretty(&config)?,
        _ => unreachable!(),
    };

    let mut file = fs::File::create(file_name)?;
    file.write_all(serialized.as_bytes())?;

    println!("Configuration file '{}' generated successfully.", file_name);
    Ok(())
}
