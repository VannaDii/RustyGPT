//! Frontend configuration module
//!
//! This module provides configuration for frontend-specific URLs and settings.

/// Frontend configuration for URLs and external links
#[derive(Debug, Clone)]
pub struct FrontendConfig {
    /// Documentation URL
    pub documentation_url: String,
}

impl Default for FrontendConfig {
    fn default() -> Self {
        Self {
            documentation_url: option_env!("RUSTYGPT_DOCUMENTATION_URL")
                .unwrap_or("https://github.com/VannaDii/RustyGPT")
                .to_string(),
        }
    }
}

impl FrontendConfig {
    /// Create a new frontend configuration instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the documentation URL
    pub fn documentation_url(&self) -> &str {
        &self.documentation_url
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_frontend_config_default() {
        let config = FrontendConfig::default();
        assert!(!config.documentation_url.is_empty());
        assert!(config.documentation_url.starts_with("http"));
    }

    #[wasm_bindgen_test]
    fn test_frontend_config_new() {
        let config = FrontendConfig::new();
        assert!(!config.documentation_url().is_empty());
    }

    #[wasm_bindgen_test]
    fn test_frontend_config_clone() {
        let config1 = FrontendConfig::new();
        let config2 = config1.clone();
        assert_eq!(config1.documentation_url(), config2.documentation_url());
    }

    #[wasm_bindgen_test]
    fn test_frontend_config_debug() {
        let config = FrontendConfig::new();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("FrontendConfig"));
        assert!(debug_str.contains("documentation_url"));
    }
}
