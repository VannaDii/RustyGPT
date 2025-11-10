#![cfg_attr(not(test), forbid(unsafe_code))]
#![deny(warnings, clippy::pedantic)]
#![allow(clippy::similar_names, clippy::unnecessary_wraps)] // Tests differentiate locales via locale codes and often return Result for ergonomic ? usage.

// Unit test modules
pub mod analyzer_tests;
pub mod generator_tests;
pub mod reporter_tests;
pub mod scanner_tests;
