//! Main entry point for the RustyGPT backend CLI.

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use std::error::Error;

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
        Commands::Spec { output_path } => {
            println!(
                "Generating OpenAPI spec at {}...",
                output_path.unwrap_or_else(|| "openapi.yaml".to_string())
            );
        }
        Commands::Completion { shell } => {
            println!("Generating shell completion script for {}...", shell);
        }
        Commands::Config { format } => {
            println!(
                "Generating configuration file in {} format...",
                format.unwrap_or_else(|| "yaml".to_string())
            );
        }
    }

    Ok(())
}
