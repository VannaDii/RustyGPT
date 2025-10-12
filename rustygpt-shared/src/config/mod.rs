//! # Configuration
//!
//! This module contains all configuration-related structures and functions
//! for both server and LLM configurations.

#[cfg(not(target_arch = "wasm32"))]
pub mod llm;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
