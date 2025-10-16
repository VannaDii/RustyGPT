//! # LLM Integration Module
//!
//! This module provides a unified interface for working with Large Language Models (LLMs).
//! It defines traits and implementations for various LLM backends, starting with `llama-cpp-rs`.
//!
//! ## Architecture
//!
//! The module is structured around several core traits:
//! - [`LLMProvider`] - Main trait defining the LLM interface
//! - [`LLMModel`] - Represents a loaded model instance
//! - [`LLMResponse`] - Standardized response format
//! - [`LLMConfig`] - Configuration for model initialization
//!
//! ## Usage
//!
//! ```rust,ignore
//! use shared::llms::{LLMProvider, LLMRequest};
//! use shared::llms::llama_cpp::LlamaCppProvider;
//! use shared::llms::types::LLMConfig;
//!
//! // Initialize the provider with configuration
//! let config = LLMConfig::default();
//! let provider = LlamaCppProvider::new(config).await?;
//!
//! // Load a model
//! let model = provider.load_model("path/to/model.gguf").await?;
//!
//! // Generate text
//! let request = LLMRequest::new("Hello, world!");
//! let response = model.generate(request).await?;
//! ```

pub mod context;
pub mod errors;
pub mod examples;
pub mod hardware;
pub mod llama_cpp;
pub mod traits;
pub mod types;

// Re-export the main public APIs
pub use context::ThreadContextBuilder;
pub use errors::{LLMError, LLMResult};
pub use hardware::{GpuType, OptimalParams, SystemHardware};
pub use traits::{LLMModel, LLMProvider};
pub use types::{
    LLMConfig, LLMRequest, LLMResponse, ModelCapabilities, ModelInfo, StreamingResponse, TokenUsage,
};
