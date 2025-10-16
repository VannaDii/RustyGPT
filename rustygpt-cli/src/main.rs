#![allow(clippy::all, clippy::pedantic)]

//! Main entry point for the RustyGPT backend CLI.

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use server::server::run;
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
            default_value = "8080",
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
    /// Start an interactive chat session with the AI
    Chat(commands::chat::ChatArgs),
    /// Reply to an existing message
    Reply(commands::chat::ReplyArgs),
    /// Follow SSE updates for a thread
    Follow(commands::chat::FollowArgs),
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
    /// Perform a device login and store session cookies
    Login(commands::session::LoginArgs),
    /// Call /api/auth/me using the stored session
    Me(commands::session::MeArgs),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, config } => {
            let resolved_config = Config::load_config(config, Some(port))
                .map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
            run(resolved_config).await.expect("Server exited");
        }
        Commands::Chat(args) => {
            commands::chat::handle_chat(args).await?;
        }
        Commands::Reply(args) => {
            commands::chat::handle_reply(args).await?;
        }
        Commands::Follow(args) => {
            commands::chat::handle_follow(args).await?;
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
        Commands::Login(args) => {
            commands::session::login(args).await?;
        }
        Commands::Me(args) => {
            commands::session::me(args).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing() {
        // Test basic CLI structure can be parsed
        let cli = Cli::try_parse_from(["cli", "serve", "--port", "8080"]);
        assert!(cli.is_ok());

        match cli.unwrap().command {
            Commands::Serve { port, .. } => assert_eq!(port, 8080),
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_spec_command() {
        let cli = Cli::try_parse_from(["cli", "spec", "test.json"]);
        if let Err(e) = &cli {
            panic!("CLI parse error: {}", e);
        }

        match cli.unwrap().command {
            Commands::Spec { output_path } => {
                assert_eq!(output_path, Some("test.json".to_string()))
            }
            _ => panic!("Expected Spec command"),
        }
    }

    #[test]
    fn test_cli_completion_command() {
        let cli = Cli::try_parse_from(["cli", "completion", "--shell", "bash"]);
        assert!(cli.is_ok());

        match cli.unwrap().command {
            Commands::Completion { shell } => assert_eq!(shell, "bash"),
            _ => panic!("Expected Completion command"),
        }
    }

    #[test]
    fn test_cli_login_command() {
        let cli = Cli::try_parse_from(["cli", "login"]);
        assert!(cli.is_ok());

        match cli.unwrap().command {
            Commands::Login(args) => assert!(args.config.is_none()),
            _ => panic!("Expected Login command"),
        }
    }

    #[test]
    fn test_cli_me_command() {
        let cli = Cli::try_parse_from(["cli", "me", "--config", "./conf.toml"]);
        assert!(cli.is_ok());

        match cli.unwrap().command {
            Commands::Me(args) => assert_eq!(args.config, Some(PathBuf::from("./conf.toml"))),
            _ => panic!("Expected Me command"),
        }
    }

    #[test]
    fn test_cli_config_command() {
        let cli = Cli::try_parse_from(["cli", "config", "--format", "json"]);
        assert!(cli.is_ok());

        match cli.unwrap().command {
            Commands::Config { format } => assert_eq!(format, Some("json".to_string())),
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_cli_config_command_default() {
        let cli = Cli::try_parse_from(["cli", "config"]);
        assert!(cli.is_ok());

        match cli.unwrap().command {
            Commands::Config { format } => assert_eq!(format, None),
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_cli_serve_command_defaults() {
        let cli = Cli::try_parse_from(["cli", "serve"]);
        assert!(cli.is_ok());

        match cli.unwrap().command {
            Commands::Serve { port, config } => {
                assert_eq!(port, 8080);
                assert_eq!(config, None);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_invalid_command() {
        let cli = Cli::try_parse_from(["cli", "invalid-command"]);
        assert!(cli.is_err());
    }

    #[test]
    fn test_cli_serve_with_config() {
        let cli = Cli::try_parse_from(["cli", "serve", "--config", "/path/to/config.yaml"]);
        assert!(cli.is_ok());

        match cli.unwrap().command {
            Commands::Serve { config, .. } => {
                assert_eq!(config, Some(PathBuf::from("/path/to/config.yaml")));
            }
            _ => panic!("Expected Serve command"),
        }
    }
}
