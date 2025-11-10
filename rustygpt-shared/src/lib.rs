#![cfg_attr(not(test), forbid(unsafe_code))]
#![deny(warnings, clippy::pedantic)]
#![allow(clippy::multiple_crate_versions)] // TODO(deps-001): remove once transitive dependencies converge.

pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod llms;
pub mod models;
