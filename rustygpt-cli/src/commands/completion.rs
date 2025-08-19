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
    use clap::CommandFactory;
    use clap_complete::generate;
    use std::io::Cursor;

    /// Helper function to generate completion script to a buffer instead of stdout
    fn generate_completion_to_buffer(shell: Shell) -> Vec<u8> {
        let mut app = crate::Cli::command();
        let mut buffer = Cursor::new(Vec::new());
        generate(shell, &mut app, "rustygpt", &mut buffer);
        buffer.into_inner()
    }

    #[test]
    fn test_generate_completion_bash() {
        // Test that bash completion generation doesn't panic and produces output
        let output = generate_completion_to_buffer(Shell::Bash);
        assert!(!output.is_empty(), "Bash completion should generate output");
        let content = String::from_utf8_lossy(&output);
        assert!(
            content.contains("rustygpt"),
            "Bash completion should contain command name"
        );
    }

    #[test]
    fn test_generate_completion_zsh() {
        // Test that zsh completion generation doesn't panic and produces output
        let output = generate_completion_to_buffer(Shell::Zsh);
        assert!(!output.is_empty(), "Zsh completion should generate output");
        let content = String::from_utf8_lossy(&output);
        assert!(
            content.contains("rustygpt"),
            "Zsh completion should contain command name"
        );
    }

    #[test]
    fn test_generate_completion_fish() {
        // Test that fish completion generation doesn't panic and produces output
        let output = generate_completion_to_buffer(Shell::Fish);
        assert!(!output.is_empty(), "Fish completion should generate output");
        let content = String::from_utf8_lossy(&output);
        assert!(
            content.contains("rustygpt"),
            "Fish completion should contain command name"
        );
    }

    #[test]
    fn test_generate_completion_powershell() {
        // Test that PowerShell completion generation doesn't panic and produces output
        let output = generate_completion_to_buffer(Shell::PowerShell);
        assert!(
            !output.is_empty(),
            "PowerShell completion should generate output"
        );
        let content = String::from_utf8_lossy(&output);
        assert!(
            content.contains("rustygpt"),
            "PowerShell completion should contain command name"
        );
    }

    #[test]
    fn test_generate_completion_elvish() {
        // Test that Elvish completion generation doesn't panic and produces output
        let output = generate_completion_to_buffer(Shell::Elvish);
        assert!(
            !output.is_empty(),
            "Elvish completion should generate output"
        );
        let content = String::from_utf8_lossy(&output);
        assert!(
            content.contains("rustygpt"),
            "Elvish completion should contain command name"
        );
    }
}
