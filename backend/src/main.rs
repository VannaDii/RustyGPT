//! Main entry point for the RustyGPT backend CLI.

use clap::{Parser, Subcommand};
use std::error::Error;

mod app_state;
mod commands;
mod handlers;
mod middleware;
mod openapi;
mod routes;
mod services;
mod tracer;

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
    },

    /// Generate the OpenAPI specification
    Spec {
        /// Output path for the OpenAPI spec (YAML or JSON based on extension, or "json"/"yaml" for streaming)
        output_path: Option<String>,
    },

    /// Generate shell completion scripts for the CLI
    Completion {
        /// The shell type for which to generate the completion script (e.g., bash, zsh, fish, powershell)
        #[arg(
            long,
            short,
            help = "The shell type for which to generate the completion script (e.g., bash, zsh, fish, powershell)"
        )]
        shell: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port } => {
            commands::server::run(port).expect("Server exited");
        }
        Commands::Spec { output_path } => {
            commands::spec::generate_spec(output_path.as_deref())?;
        }
        Commands::Completion { shell } => {
            let shell = shell
                .parse::<clap_complete::Shell>()
                .expect("Invalid shell type provided");
            commands::completion::generate_completion(shell);
        }
    }

    Ok(())
}
