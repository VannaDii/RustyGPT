#![cfg_attr(not(test), forbid(unsafe_code))]
#![deny(warnings, clippy::pedantic)]

// Main integration test file
// This imports and re-exports modules from our test directory structure

mod common;
mod unit;
