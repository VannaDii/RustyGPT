#![cfg_attr(not(test), forbid(unsafe_code))]
#![deny(warnings, clippy::pedantic)]
#![allow(clippy::multiple_crate_versions)] // TODO(deps-001): remove once transitive dependencies converge.

// Make modules public so they can be tested
pub mod analyzer;
pub mod generator;
pub mod reporter;
pub mod scanner;
