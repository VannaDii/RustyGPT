//! Main entry point for the RustyGPT backend CLI.

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use shared::config::server::Config;
use std::error::Error;
use std::path::PathBuf;

mod app_state;
mod handlers;
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

/// RustyGPT CLI
#[derive(Parser)]
#[command(name = "RustyGPT CLI")]
#[command(about = "Backend server and tools for RustyGPT", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Subcommands for the RustyGPT CLI
#[derive(Subcommand)]
enum Commands {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, config } => {
            let resolved_config = Config::load_config(config, Some(port))?;
            server::run(resolved_config).await.expect("Server exited");
        }
    }

    Ok(())
}
