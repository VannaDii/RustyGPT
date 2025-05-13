//! Main entry point for the RustyGPT backend CLI.

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use server::server;
use shared::config::server::Config;
use std::{error::Error, path::PathBuf};

mod commands;

/// RustyGPT CLI
#[derive(Parser)]
#[command(name = "RustyGPT CLI")]
#[command(about = "Command-line interface for RustyGPT", long_about = None)]
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

    /// Generate a configuration file
    Config {
        /// Format of the configuration file to generate (yaml or json). Defaults to yaml.
        #[arg(
            long,
            short,
            help = "Format of the configuration file to generate (yaml or json). Defaults to yaml."
        )]
        format: Option<String>,
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
        Commands::Spec { output_path } => {
            commands::spec::generate_spec(output_path.as_deref())?;
        }
        Commands::Completion { shell } => {
            let shell = shell
                .parse::<clap_complete::Shell>()
                .expect("Invalid shell type provided");
            commands::completion::generate_completion(shell);
        }
        Commands::Config { format } => {
            let format = format.unwrap_or_else(|| "yaml".to_string());
            commands::config::generate_config(&format)?;
        }
    }

    Ok(())
}
