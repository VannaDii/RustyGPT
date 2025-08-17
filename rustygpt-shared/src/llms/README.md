# RustyGPT LLM Trait System

This directory contains the core LLM (Large Language Model) integration system for RustyGPT. It provides a unified interface for working with different LLM backends through a trait-based architecture.

## Architecture Overview

The LLM system is built around several core traits:

- **`LLMProvider`** - Main trait for creating and managing LLM backends
- **`LLMModel`** - Interface for loaded model instances
- **`LLMEmbedding`** - Optional trait for models that support embeddings
- **`LLMFunctionCalling`** - Optional trait for models that support function calling

## Modules

### Core Modules

- **`traits.rs`** - Core trait definitions and contracts
- **`types.rs`** - Data types used throughout the system (requests, responses, configuration)
- **`errors.rs`** - Error handling and result types

### Implementations

- **`llama_cpp.rs`** - Mock implementation demonstrating the interface (ready for real llama-cpp-rs integration)

### Documentation & Examples

- **`examples.rs`** - Usage examples and demonstrations
- **`mod.rs`** - Module organization and public API exports

## Key Features

### Type Safety

All LLM interactions are strongly typed with comprehensive error handling:

```rust
use shared::llms::{LLMProvider, LLMRequest, LLMResult};
use shared::llms::llama_cpp::LlamaCppProvider;

async fn generate_text() -> LLMResult<String> {
    let config = LLMConfig::default();
    let provider = LlamaCppProvider::new(config).await?;
    let model = provider.load_model("model.gguf").await?;

    let request = LLMRequest::new("Hello, world!")
        .with_max_tokens(100)
        .with_temperature(0.7);

    let response = model.generate(request).await?;
    Ok(response.text)
}
```

### Streaming Support

Real-time streaming responses for better user experience:

```rust
let request = LLMRequest::new_streaming("Tell me a story");
let mut stream = model.generate_stream(request).await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    print!("{}", chunk.text_delta);
    if chunk.is_final {
        break;
    }
}
```

### Flexible Configuration

Comprehensive configuration options for different use cases:

```rust
let config = LLMConfig {
    model_path: "path/to/model.gguf".to_string(),
    max_tokens: Some(512),
    temperature: Some(0.8),
    top_p: Some(0.9),
    context_size: Some(4096),
    n_gpu_layers: Some(20),
    ..Default::default()
};
```

### Error Handling

Detailed error types for robust error handling:

```rust
match provider.load_model("model.gguf").await {
    Ok(model) => { /* use model */ },
    Err(LLMError::ModelNotFound { path }) => {
        eprintln!("Model not found: {}", path);
    },
    Err(LLMError::InvalidModelFormat { details }) => {
        eprintln!("Invalid model format: {}", details);
    },
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Current Implementation Status

### ✅ Completed

- Core trait system design
- Type definitions and error handling
- Mock LlamaCpp implementation for development
- Comprehensive test coverage (30 tests passing)
- Documentation and examples

### 🚧 In Progress

- Real llama-cpp-rs integration (blocked by build issues)
- Server and CLI integration

### 📋 Planned

- Additional provider implementations (Candle, ONNX, etc.)
- Embedding support
- Function calling capabilities
- Model caching and management
- Performance optimizations

## Integration

### Server Integration

The server can use the LLM system like this:

```rust
// In server/src/handlers/chat.rs
use shared::llms::{LLMProvider, LLMRequest};
use shared::llms::llama_cpp::LlamaCppProvider;

pub async fn handle_chat_request(prompt: String) -> Result<String, Box<dyn std::error::Error>> {
    let config = LLMConfig::default(); // Load from server config
    let provider = LlamaCppProvider::new(config).await?;
    let model = provider.load_model("models/chat-model.gguf").await?;

    let request = LLMRequest::new(prompt);
    let response = model.generate(request).await?;

    Ok(response.text)
}
```

### CLI Integration

The CLI can use the same interface:

```rust
// In cli/src/commands/chat.rs
use shared::llms::{LLMProvider, LLMRequest};
use shared::llms::llama_cpp::LlamaCppProvider;

pub async fn chat_command(args: ChatArgs) -> Result<(), Box<dyn std::error::Error>> {
    let config = LLMConfig::from_cli_args(&args);
    let provider = LlamaCppProvider::new(config).await?;
    let model = provider.load_model(&args.model_path).await?;

    let request = LLMRequest::new(args.prompt)
        .with_max_tokens(args.max_tokens)
        .with_temperature(args.temperature);

    let response = model.generate(request).await?;
    println!("{}", response.text);

    Ok(())
}
```

## Testing

Run the LLM tests with:

```bash
cargo test -p shared llms
```

All 30 tests should pass, covering:

- Provider creation and configuration
- Model loading and validation
- Text generation (mock)
- Streaming responses
- Error handling
- Tokenization
- Serialization/deserialization

## Future Extensions

The trait system is designed to be extensible. Future implementations might include:

- **Candle Provider** - Pure Rust inference with candle-transformers
- **ONNX Provider** - ONNX Runtime integration
- **Remote Provider** - API-based providers (OpenAI, Anthropic, etc.)
- **Embedding Provider** - Specialized embedding models
- **Multi-modal Provider** - Vision and audio model support

Each implementation follows the same trait contract, ensuring consistent behavior across different backends.
