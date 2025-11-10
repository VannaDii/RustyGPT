//! # LLM Example Usage
//!
//! This module provides examples of how to use the LLM trait system.

use crate::llms::{
    errors::LLMResult,
    llama_cpp::LlamaCppProvider,
    traits::LLMProvider,
    types::{LLMConfig, LLMRequest},
};

/// Example function demonstrating basic LLM usage
///
/// # Errors
///
/// This mock does not currently return an error, but a real implementation would
/// propagate any [`LLMError`](crate::llms::errors::LLMError) that occurs while constructing the provider,
/// loading models, or generating text.
pub async fn basic_example() -> LLMResult<String> {
    // Create a configuration for the LLM
    let _config = LLMConfig {
        model_path: "path/to/model.gguf".to_string(),
        max_tokens: Some(100),
        temperature: Some(0.7),
        ..Default::default()
    };

    futures_util::future::ready(()).await;

    // In a real implementation, you would:
    // let provider = LlamaCppProvider::new(config).await?;
    // let model = provider.load_model("model.gguf").await?;
    // let request = LLMRequest::new("Write a short poem about Rust programming")
    //     .with_max_tokens(100)
    //     .with_temperature(0.8);
    // let response = model.generate(request).await?;
    // Ok(response.text)

    // For now, return a mock response
    Ok("Mock response from LLM".to_string())
}

/// Example function demonstrating text generation
///
/// # Errors
///
/// Returns an [`LLMError`](crate::llms::errors::LLMError) if the underlying `LlamaCppProvider::new` call fails
/// or if subsequent model loading/generation encounters an error.
pub async fn text_generation_example() -> LLMResult<String> {
    // This example shows how to use the LLM for text generation
    // In a real application, you would have an actual model file

    let config = LLMConfig {
        model_path: "example.gguf".to_string(),
        max_tokens: Some(50),
        temperature: Some(0.8),
        ..Default::default()
    };

    let _provider = LlamaCppProvider::new(config).await?;

    // Note: This would require an actual model file to work
    // let model = provider.load_model("example.gguf").await?;

    // Create a request
    let _request = LLMRequest::new("Write a short poem about Rust programming")
        .with_max_tokens(50)
        .with_temperature(0.8);

    // In a real implementation:
    // let response = model.generate(request).await?;
    // Ok(response.text)

    Ok(
        "Mock response: Rust programming is fast and safe, memory management without a race..."
            .to_string(),
    )
}

/// Example function demonstrating streaming generation
///
/// # Errors
///
/// Returns an [`LLMError`](crate::llms::errors::LLMError) if provider initialization or streaming generation
/// fails in a real implementation.
pub async fn streaming_example() -> LLMResult<Vec<String>> {
    // For this example, we'll simulate streaming without requiring actual models
    let _config = LLMConfig::default();

    futures_util::future::ready(()).await;

    // In a real implementation, you would:
    // let provider = LlamaCppProvider::new(config).await?;
    // let model = provider.load_model("model.gguf").await?;
    // let request = LLMRequest::new_streaming("Tell me about the weather");
    // let mut stream = model.generate_stream(request).await?;
    // let mut chunks = Vec::new();
    //
    // while let Some(chunk) = stream.next().await {
    //     let chunk = chunk?;
    //     chunks.push(chunk.text_delta);
    //     if chunk.is_final {
    //         break;
    //     }
    // }
    //
    // Ok(chunks)

    // For now, return mock streaming chunks
    Ok(vec![
        "Mock".to_string(),
        " streaming".to_string(),
        " response".to_string(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_example() {
        let result = basic_example().await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Mock response"));
    }

    #[tokio::test]
    async fn test_text_generation_example() {
        let result = text_generation_example().await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Mock response"));
    }

    #[tokio::test]
    async fn test_streaming_example() {
        let result = streaming_example().await;
        assert!(result.is_ok());
        let chunks = result.unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], "Mock");
    }
}
