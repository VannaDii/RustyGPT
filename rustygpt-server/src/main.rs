#![cfg_attr(not(test), forbid(unsafe_code))]
#![deny(warnings, clippy::pedantic)]
#![allow(clippy::multiple_crate_versions)] // TODO(deps-001): remove once transitive dependencies converge.

//! Main entry point for the `RustyGPT` backend CLI.

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use shared::config::server::Config;
use std::error::Error;
use std::path::PathBuf;

mod app_state;
mod auth;
mod db;
mod handlers;
mod http;
mod middleware;
mod openapi;
mod routes;
mod server;
mod services;
mod tracer;

#[cfg(test)]
mod main_tests;

#[cfg(test)]
mod server_test;

/// Main CLI structure for the `RustyGPT` server
#[derive(Parser)]
#[command(name = "RustyGPT CLI")]
#[command(about = "Backend server and tools for RustyGPT", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands for the `RustyGPT` CLI
#[derive(Subcommand)]
pub enum Commands {
    /// Start the backend server
    Serve {
        /// The port number to bind the server to (e.g., 8080). Example usage: `--port 8080`
        #[arg(
            long,
            short,
            help = "The port number to bind the server to (e.g., 8080). Example usage: `--port 8080`"
        )]
        port: u16,

        /// Path to the configuration file (optional)
        #[arg(
            long,
            short,
            help = "Path to the configuration file (e.g., config.yaml or config.json). If not provided, defaults will be used."
        )]
        config: Option<PathBuf>,
    },
}

/// Initializes environment variables and returns the parsed CLI.
///
/// # Returns
/// Returns the parsed [`Cli`] structure.
#[must_use]
pub fn initialize_cli() -> Cli {
    dotenv().ok();
    Cli::parse()
}

/// Handles the serve command by loading configuration and starting the server.
///
/// # Arguments
/// * `port` - The port number to bind the server to.
/// * `config` - Optional path to the configuration file.
///
/// # Errors
/// Returns an error if configuration loading or server startup fails.
///
/// # Panics
/// Panics if the server runtime exits unexpectedly.
pub async fn handle_serve_command(
    port: u16,
    config: Option<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let resolved_config = Config::load_config(config, Some(port))
        .map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
    server::run(resolved_config).await.expect("Server exited");
    Ok(())
}

/// Main application entry point.
///
/// # Errors
/// Returns an error if the application fails to initialize or run.
pub async fn run_app() -> Result<(), Box<dyn Error>> {
    let cli = initialize_cli();

    match cli.command {
        Commands::Serve { port, config } => {
            handle_serve_command(port, config).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    run_app().await
}
