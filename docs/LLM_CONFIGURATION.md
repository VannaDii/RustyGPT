# LLM Configuration Integration Guide

This guide explains how to use the new LLM configuration system that has been integrated into RustyGPT.

## Overview

The LLM configuration system provides a comprehensive way to configure and manage Large Language Models (LLMs) in RustyGPT. It supports:

- Multiple LLM providers (currently supports llama-cpp-rs with mock implementation)
- Multiple models per provider
- Environment variable configuration
- YAML/JSON configuration files
- Provider-specific settings
- Model-specific parameters
- Global LLM settings

## Configuration Structure

### Main Configuration

The main `Config` struct now includes LLM settings:

```rust
use shared::config::server::Config;

let config = Config::with_defaults();
println!("Default LLM provider: {}", config.llm.default_provider);
```

### LLM Configuration

The LLM configuration (`LLMConfiguration`) includes:

- **Providers**: Different LLM backends (llama_cpp, candle, etc.)
- **Models**: Specific model configurations with their settings
- **Global Settings**: System-wide LLM settings

## Using Configuration Files

### YAML Configuration

Create a `config.yaml` file (see `config.example.yaml` for a complete example):

```yaml
# Server settings
server_port: 8080
log_level: 'info'

# LLM Configuration
llm:
  default_provider: 'llama_cpp'
  models_directory: './models'
  default_chat_model: 'llama2-7b-chat'

  providers:
    llama_cpp:
      provider_type: 'llama_cpp'
      enabled: true
      n_gpu_layers: 0
      n_threads: 4

  models:
    llama2-7b-chat:
      path: 'llama-2-7b-chat.Q4_K_M.gguf'
      provider: 'llama_cpp'
      display_name: 'Llama 2 7B Chat'
      default_params:
        max_tokens: 512
        temperature: 0.7
        context_size: 4096
```

### Loading Configuration

```rust
use shared::config::server::Config;

// Load from file with environment variable overrides
let config = Config::load_from_file_and_env(Some("config.yaml"), None).unwrap();

// Get LLM configuration for chat
let chat_config = config.get_chat_llm_config().unwrap();
println!("Model path: {}", chat_config.model_path);
```

## Environment Variables

You can override configuration using environment variables:

```bash
# LLM-specific settings
export RUSTYGPT_MODELS_DIR="/path/to/models"
export RUSTYGPT_DEFAULT_PROVIDER="llama_cpp"
export RUSTYGPT_DEFAULT_MODEL="my-model"
export RUSTYGPT_GPU_LAYERS="10"
export RUSTYGPT_THREADS="8"
export RUSTYGPT_LLM_TIMEOUT="60"
export RUSTYGPT_MAX_CONCURRENT_REQUESTS="4"

# Server settings
export RUSTYGPT_SERVER_PORT="3000"
export RUSTYGPT_LOG_LEVEL="debug"
```

## Code Examples

### Basic LLM Usage

```rust
use shared::{
    config::server::Config,
    llms::{llama_cpp::LlamaCppProvider, traits::LLMProvider, types::LLMRequest}
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::with_defaults();

    // Get LLM configuration for chat
    let llm_config = config.get_chat_llm_config()?;

    // Create provider
    let provider = LlamaCppProvider::new(llm_config).await?;

    // Load model (in real implementation)
    // let model = provider.load_model("model.gguf").await?;

    // Create request
    let request = LLMRequest::new("Hello, how are you?")
        .with_max_tokens(100)
        .with_temperature(0.7);

    // Generate response (in real implementation)
    // let response = model.generate(request).await?;
    // println!("Response: {}", response.text);

    Ok(())
}
```

### Working with Multiple Models

```rust
use shared::config::server::Config;

// Load configuration
let config = Config::with_defaults();

// Get configuration for a specific model
let model_config = config.get_llm_config("llama2-13b-chat")?;
println!("Model path: {}", model_config.model_path);
println!("Context size: {:?}", model_config.context_size);

// Check model capabilities
if let Some(model_info) = config.llm.get_model_config("llama2-13b-chat") {
    if model_info.capabilities.function_calling {
        println!("This model supports function calling!");
    }
}
```

### Adding Custom Models

```rust
use shared::config::{
    server::Config,
    llm::{ModelConfig, ModelParameters, ModelCapabilities}
};

let mut config = Config::with_defaults();

// Add a new model
let custom_model = ModelConfig {
    path: "custom-model.gguf".to_string(),
    provider: "llama_cpp".to_string(),
    display_name: "Custom Model".to_string(),
    description: Some("My custom model".to_string()),
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

// Now you can use the custom model
let custom_config = config.get_llm_config("custom")?;
```

### Validation

```rust
use shared::config::server::Config;

let config = Config::with_defaults();

// Validate the configuration
match config.validate() {
    Ok(()) => println!("Configuration is valid!"),
    Err(errors) => {
        println!("Configuration errors:");
        for error in errors {
            println!("  - {}", error);
        }
    }
}
```

## Model Directory Structure

Organize your models in the configured models directory:

```
models/
├── llama-2-7b-chat.Q4_K_M.gguf
├── llama-2-13b-chat.Q4_K_M.gguf
├── code-llama-7b.Q4_K_M.gguf
└── embeddings/
    └── all-MiniLM-L6-v2.gguf
```

## Provider-Specific Settings

### Llama.cpp Provider

```yaml
providers:
  llama_cpp:
    provider_type: 'llama_cpp'
    enabled: true
    n_gpu_layers: 10 # Number of layers to offload to GPU
    n_threads: 8 # Number of CPU threads
    additional_settings:
      use_mmap: true # Use memory mapping
      use_mlock: false # Lock model in RAM
```

### Adding New Providers

```rust
use shared::config::llm::{ProviderConfig, LLMConfiguration};
use std::collections::HashMap;

let mut config = LLMConfiguration::default();

// Add a new provider
let candle_provider = ProviderConfig {
    provider_type: "candle".to_string(),
    enabled: true,
    n_gpu_layers: Some(10),
    n_threads: Some(8),
    additional_settings: {
        let mut settings = HashMap::new();
        settings.insert("use_flash_attention".to_string(), serde_json::Value::Bool(true));
        settings.insert("dtype".to_string(), serde_json::Value::String("f16".to_string()));
        settings
    },
};

config.add_provider("candle".to_string(), candle_provider);
```

## Global Settings

Configure system-wide LLM behavior:

```yaml
llm:
  global_settings:
    default_timeout: 30 # Request timeout in seconds
    max_concurrent_requests: 4 # Maximum concurrent LLM requests
    enable_model_caching: true # Cache loaded models in memory
    cache_size_limit_mb: 4096 # Cache size limit in MB
    enable_request_logging: true # Log requests for debugging
    enable_metrics: true # Collect performance metrics
```

## Integration with Server and CLI

### Server Integration

```rust
// In server code
use shared::config::server::Config;

let config = Config::load_from_file_and_env(Some("config.yaml"), None)?;

// Initialize LLM system with configuration
let llm_config = config.get_chat_llm_config()?;
// Initialize your LLM provider here
```

### CLI Integration

```rust
// In CLI code
use shared::config::server::Config;

let config = Config::load_from_file_and_env(None, Some(args.port))?;

// Use LLM configuration
let llm_config = config.get_chat_llm_config()?;
// Initialize your LLM provider here
```

## Error Handling

The configuration system provides detailed error messages:

```rust
use shared::config::server::Config;

match Config::load_from_file_and_env(Some("config.yaml"), None) {
    Ok(config) => {
        // Use configuration
    },
    Err(e) => {
        eprintln!("Configuration error: {}", e);
        std::process::exit(1);
    }
}

// Validate configuration
if let Err(errors) = config.validate() {
    for error in errors {
        eprintln!("Validation error: {}", error);
    }
}
```

## Testing

The configuration system includes comprehensive tests. Run them with:

```bash
cargo test -p shared config::
```

## Next Steps

1. **Implement Real Provider**: Replace the mock `LlamaCppProvider` with the actual llama-cpp-rs integration
2. **Add More Providers**: Implement support for Candle, ONNX, or other LLM frameworks
3. **Web UI Integration**: Add configuration management to the web interface
4. **Model Management**: Add automatic model downloading and management features
5. **Performance Monitoring**: Implement the metrics collection system

This configuration system provides a solid foundation for managing LLMs in RustyGPT and can be easily extended as new providers and features are added.
