#![cfg_attr(not(test), forbid(unsafe_code))]
#![deny(warnings, clippy::pedantic)]
#![allow(clippy::similar_names)] // Fixture helpers intentionally mirror locale codes (en/es/etc.).

// Common test utilities
pub mod test_utils;
