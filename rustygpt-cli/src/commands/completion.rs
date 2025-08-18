//! Module for generating shell completion scripts for the CLI.

use clap::CommandFactory;
use clap_complete::{generate, shells::Shell};
use std::io;

/// Generates shell completion scripts for the CLI.
///
/// # Arguments
/// * `shell` - The {@link Shell} type for which to generate the completion script.
///
/// # Examples
/// ```
/// commands::completion::generate_completion(Shell::Bash);
/// ```
pub fn generate_completion(shell: Shell) {
    let mut app = crate::Cli::command();
    generate(shell, &mut app, "rustygpt", &mut io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_completion_bash() {
        // Test that bash completion generation doesn't panic
        generate_completion(Shell::Bash);
    }

    #[test]
    fn test_generate_completion_zsh() {
        // Test that zsh completion generation doesn't panic
        generate_completion(Shell::Zsh);
    }

    #[test]
    fn test_generate_completion_fish() {
        // Test that fish completion generation doesn't panic
        generate_completion(Shell::Fish);
    }

    #[test]
    fn test_generate_completion_powershell() {
        // Test that PowerShell completion generation doesn't panic
        generate_completion(Shell::PowerShell);
    }

    #[test]
    fn test_generate_completion_elvish() {
        // Test that Elvish completion generation doesn't panic
        generate_completion(Shell::Elvish);
    }
}
