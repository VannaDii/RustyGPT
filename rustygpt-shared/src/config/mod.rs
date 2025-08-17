//! # Configuration
//!
//! This module contains all configuration-related structures and functions
//! for both server and LLM configurations.

pub mod llm;
pub mod server;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod server_integration_test;
