//! Chat command implementation for RustyGPT CLI.
//!
//! This module provides an interactive chat interface that connects to the
//! shared LLM functionality. Users can have conversations with AI models
//! directly from the command line.

use anyhow::{Context, Result};
use shared::llms::{LLMConfig, LLMProvider, LLMRequest, llama_cpp::LlamaCppProvider};
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Start an interactive chat session with the configured LLM.
///
/// # Arguments
/// * `model_path` - Optional path to the model file. If not provided, uses default.
/// * `max_tokens` - Optional maximum tokens per response.
/// * `temperature` - Optional temperature for response generation (0.0-1.0).
/// * `system_message` - Optional system message to set the AI's behavior.
///
/// # Returns
/// A [`Result`] indicating success or failure of the chat session.
///
/// # Errors
/// Returns an error if:
/// - The model file cannot be loaded
/// - Invalid configuration parameters are provided
/// - I/O errors occur during the chat session
pub async fn start_chat(
    model_path: Option<PathBuf>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    system_message: Option<String>,
) -> Result<()> {
    println!("🚀 Starting RustyGPT Chat...");

    // Create LLM configuration
    let config = create_llm_config(model_path, max_tokens, temperature)?;

    // Initialize the LLM provider
    let provider = LlamaCppProvider::new(config.clone())
        .await
        .context("Failed to initialize LLM provider")?;

    // Load the model
    let model = provider
        .load_model_with_config(config)
        .await
        .context("Failed to load LLM model")?;

    println!("✅ Model loaded successfully!");
    println!("Type 'exit', 'quit', or press Ctrl+C to end the chat.\n");

    // Start the interactive chat loop
    run_chat_loop(model, system_message).await?;

    Ok(())
}

/// Create an LLM configuration from command line parameters.
///
/// # Arguments
/// * `model_path` - Optional path to the model file
/// * `max_tokens` - Optional maximum tokens per response
/// * `temperature` - Optional temperature for generation
///
/// # Returns
/// A configured [`LLMConfig`] instance.
///
/// # Errors
/// Returns an error if the configuration parameters are invalid.
fn create_llm_config(
    model_path: Option<PathBuf>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<LLMConfig> {
    let model_path = model_path
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "mock_model.gguf".to_string());

    // Validate temperature if provided
    if let Some(temp) = temperature {
        if !(0.0..=1.0).contains(&temp) {
            anyhow::bail!("Temperature must be between 0.0 and 1.0, got {}", temp);
        }
    }

    // Validate max_tokens if provided
    if let Some(tokens) = max_tokens {
        if tokens == 0 {
            anyhow::bail!("Max tokens must be greater than 0");
        }
    }

    // Try to create an optimized configuration for the current hardware
    match LLMConfig::optimized_for_hardware(&model_path, None) {
        Ok(mut config) => {
            // Apply user overrides
            if let Some(tokens) = max_tokens {
                config.max_tokens = Some(tokens);
            }
            if let Some(temp) = temperature {
                config.temperature = Some(temp);
            }
            Ok(config)
        }
        Err(_) => {
            // Fall back to default configuration if hardware detection fails
            Ok(LLMConfig {
                model_path,
                max_tokens,
                temperature,
                ..Default::default()
            })
        }
    }
}

/// Run the interactive chat loop.
///
/// # Arguments
/// * `model` - The loaded LLM model instance
/// * `system_message` - Optional system message to include with requests
///
/// # Returns
/// A [`Result`] indicating success or failure of the chat session.
///
/// # Errors
/// Returns an error if I/O operations fail or model generation fails.
async fn run_chat_loop(
    model: impl shared::llms::LLMModel,
    system_message: Option<String>,
) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        // Prompt for user input
        print!("You: ");
        io::stdout().flush()?;

        // Read user input
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            // EOF (Ctrl+D)
            println!("\nGoodbye! 👋");
            break;
        }

        let input = line.trim();

        // Check for exit commands
        if input.is_empty() {
            continue;
        }

        if matches!(input.to_lowercase().as_str(), "exit" | "quit" | "q") {
            println!("Goodbye! 👋");
            break;
        }

        // Create LLM request
        let mut request = LLMRequest::new(input);
        if let Some(ref sys_msg) = system_message {
            request = request.with_system_message(sys_msg);
        }

        // Generate response
        print!("AI: ");
        io::stdout().flush()?;

        match model.generate(request).await {
            Ok(response) => {
                println!("{}", response.text);

                // Show token usage if available
                if response.usage.total_tokens > 0 {
                    println!(
                        "   [Tokens: {} prompt + {} completion = {} total]",
                        response.usage.prompt_tokens,
                        response.usage.completion_tokens,
                        response.usage.total_tokens
                    );
                }
            }
            Err(e) => {
                eprintln!("❌ Error generating response: {}", e);
            }
        }

        println!(); // Add spacing between exchanges
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_create_llm_config_default() {
        let config = create_llm_config(None, None, None).unwrap();
        assert_eq!(config.model_path, "mock_model.gguf");
        // Note: max_tokens and temperature might be set by hardware optimization
        // so we only test that the function succeeds
    }

    #[test]
    fn test_create_llm_config_with_parameters() {
        let model_path = Some(PathBuf::from("/path/to/model.gguf"));
        let max_tokens = Some(512);
        let temperature = Some(0.7);

        let config = create_llm_config(model_path, max_tokens, temperature).unwrap();
        assert_eq!(config.model_path, "/path/to/model.gguf");
        // User-provided parameters should override defaults
        assert_eq!(config.max_tokens, Some(512));
        assert_eq!(config.temperature, Some(0.7));
    }

    #[test]
    fn test_create_llm_config_invalid_temperature() {
        let result = create_llm_config(None, None, Some(1.5));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Temperature must be between 0.0 and 1.0")
        );
    }

    #[test]
    fn test_create_llm_config_invalid_temperature_negative() {
        let result = create_llm_config(None, None, Some(-0.1));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Temperature must be between 0.0 and 1.0")
        );
    }

    #[test]
    fn test_create_llm_config_invalid_max_tokens() {
        let result = create_llm_config(None, Some(0), None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Max tokens must be greater than 0")
        );
    }

    #[test]
    fn test_create_llm_config_valid_temperature_boundaries() {
        // Test boundary values
        let config_zero = create_llm_config(None, None, Some(0.0)).unwrap();
        assert_eq!(config_zero.temperature, Some(0.0));

        let config_one = create_llm_config(None, None, Some(1.0)).unwrap();
        assert_eq!(config_one.temperature, Some(1.0));
    }

    #[test]
    fn test_create_llm_config_with_temp_file_path() {
        let temp_dir = tempdir().unwrap();
        let model_file = temp_dir.path().join("test_model.gguf");
        fs::write(&model_file, "dummy model content").unwrap();

        let config = create_llm_config(Some(model_file.clone()), None, None).unwrap();
        assert_eq!(config.model_path, model_file.to_string_lossy());
    }

    #[test]
    fn test_create_llm_config_all_parameters() {
        let model_path = Some(PathBuf::from("/custom/model.gguf"));
        let max_tokens = Some(1024);
        let temperature = Some(0.3);

        let config = create_llm_config(model_path, max_tokens, temperature).unwrap();
        assert_eq!(config.model_path, "/custom/model.gguf");
        assert_eq!(config.max_tokens, Some(1024));
        assert_eq!(config.temperature, Some(0.3));
    }

    #[test]
    fn test_create_llm_config_large_max_tokens() {
        let max_tokens = Some(4096);
        let config = create_llm_config(None, max_tokens, None).unwrap();
        assert_eq!(config.max_tokens, Some(4096));
    }
}
