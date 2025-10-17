#![allow(clippy::all, clippy::pedantic)]

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
