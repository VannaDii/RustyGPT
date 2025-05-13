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
