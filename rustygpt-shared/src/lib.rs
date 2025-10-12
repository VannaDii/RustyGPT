#![allow(clippy::all, clippy::pedantic)]

pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod llms;
pub mod models;
