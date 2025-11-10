#![cfg_attr(not(test), forbid(unsafe_code))]
#![deny(warnings, clippy::pedantic)]
#![allow(clippy::multiple_crate_versions)] // TODO(deps-001): remove once transitive dependencies converge.

mod app_state;
mod auth;
pub(crate) mod db;
mod handlers;
mod http;
mod middleware;
pub mod openapi;
mod routes;
pub mod server;
mod services;
mod tracer;

#[cfg(test)]
mod server_test;
#[cfg(test)]
mod tracer_tests;
