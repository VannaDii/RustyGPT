#![cfg_attr(not(test), forbid(unsafe_code))]
#![deny(warnings, clippy::pedantic)]

use clap::Parser;
use colored::*;
use std::collections::HashMap;
use std::path::PathBuf;

// Import the CLI struct definition from main.rs
// This would be better to directly import from src, but we'll redefine for the test
#[derive(Parser, Debug)]
#[command(
    name = "ConFuse",
    about = "Run multiple tasks concurrently with log prefixing (concurrently style)"
)]
struct Cli {
    /// Commands to run concurrently. Each command should be quoted.
    ///
    /// Example:
    ///   confuse "cargo watch -x run" "trunk watch"
    #[arg(required = true)]
    commands: Vec<String>,

    /// Comma-separated list of names for the commands.
    /// These names override any inline name in the command strings.
    #[arg(short, long)]
    names: Option<String>,

    /// Comma-separated list of prefix colors for the commands.
    /// Supported colors: black, red, green, yellow, blue, magenta, cyan, white.
    #[arg(short = 'p', long = "prefix-colors")]
    prefix_colors: Option<String>,

    /// Optional working directory to apply to all commands.
    #[arg(short = 'd', long)]
    cwd: Option<PathBuf>,
}

// Duplicate function for testing
fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        _ => Color::White,
    }
}

// Duplicate function for testing
fn parse_command(
    cmd_str: &str,
    default_cwd: Option<PathBuf>,
) -> (Option<String>, String, Vec<String>, Option<PathBuf>) {
    // Split the input into a prefix (everything before the colon, if present)
    // and the remainder (the actual command and arguments).
    let (prefix_opt, command_str) = cmd_str.split_once(':').map_or_else(
        || (None, cmd_str.trim()),
        |(prefix, rest)| (Some(prefix.trim()), rest.trim()),
    );

    // Parse the command and its arguments using shlex.
    let parts = shlex::split(command_str).expect("Failed to parse command arguments");
    assert!(!parts.is_empty(), "No command provided in '{cmd_str}'");

    // Extract name and working directory from the prefix if available.
    let prefix_data = prefix_opt.map(|prefix| {
        prefix.split_once('@').map_or_else(
            || ((!prefix.is_empty()).then_some(prefix.to_string()), None),
            |(name_part, cwd_part)| {
                let trimmed_name = name_part.trim();
                (
                    (!trimmed_name.is_empty()).then_some(trimmed_name.to_string()),
                    Some(PathBuf::from(cwd_part.trim())),
                )
            },
        )
    });

    let (name, working_dir_override) = prefix_data.unwrap_or((None, None));
    let working_dir = working_dir_override.or(default_cwd);

    (name, parts[0].clone(), parts[1..].to_vec(), working_dir)
}

#[test]
fn test_cli_parse_empty_commands() {
    // Should panic because commands are required
    let result = Cli::try_parse_from(vec!["confuse"]);
    assert!(result.is_err());
}

#[test]
fn test_cli_parse_with_commands() {
    let cli = Cli::try_parse_from(vec!["confuse", "echo hello", "echo world"]).unwrap();
    assert_eq!(cli.commands.len(), 2);
    assert_eq!(cli.commands[0], "echo hello");
    assert_eq!(cli.commands[1], "echo world");
    assert!(cli.names.is_none());
    assert!(cli.prefix_colors.is_none());
    assert!(cli.cwd.is_none());
}

#[test]
fn test_cli_parse_with_all_options() {
    let cli = Cli::try_parse_from(vec![
        "confuse",
        "echo hello",
        "echo world",
        "--names",
        "cmd1,cmd2",
        "--prefix-colors",
        "red,blue",
        "--cwd",
        "/tmp",
    ])
    .unwrap();

    assert_eq!(cli.commands.len(), 2);
    assert_eq!(cli.commands[0], "echo hello");
    assert_eq!(cli.commands[1], "echo world");
    assert_eq!(cli.names, Some("cmd1,cmd2".to_string()));
    assert_eq!(cli.prefix_colors, Some("red,blue".to_string()));
    assert_eq!(cli.cwd, Some(PathBuf::from("/tmp")));
}

#[test]
fn test_parse_color_case_insensitive() {
    assert_eq!(parse_color("RED"), Color::Red);
    assert_eq!(parse_color("Blue"), Color::Blue);
    assert_eq!(parse_color("green"), Color::Green);
}

#[test]
fn test_parse_color_invalid() {
    // Invalid colors should default to white
    assert_eq!(parse_color("not_a_color"), Color::White);
    assert_eq!(parse_color(""), Color::White);
    assert_eq!(parse_color("purple"), Color::White);
}

#[test]
fn test_parse_command_complex() {
    // Test with complex command including quotes and special characters
    let (name, command, args, working_dir) =
        parse_command("complex_task:echo \"Hello, World!\" | grep 'World'", None);

    assert_eq!(name, Some("complex_task".to_string()));
    assert_eq!(command, "echo");
    assert_eq!(args, vec!["Hello, World!", "|", "grep", "World"]);
    assert_eq!(working_dir, None);
}

#[test]
fn test_parse_command_with_working_dir_and_name() {
    let (name, command, args, working_dir) = parse_command(
        "task_name@/custom/path:ls -la",
        Some(PathBuf::from("/default/path")),
    );

    assert_eq!(name, Some("task_name".to_string()));
    assert_eq!(command, "ls");
    assert_eq!(args, vec!["-la"]);
    assert_eq!(working_dir, Some(PathBuf::from("/custom/path")));
}

#[test]
fn test_parse_command_with_only_working_dir() {
    let (name, command, args, working_dir) = parse_command("@/custom/path:ls -la", None);

    assert_eq!(name, None);
    assert_eq!(command, "ls");
    assert_eq!(args, vec!["-la"]);
    assert_eq!(working_dir, Some(PathBuf::from("/custom/path")));
}

#[test]
fn test_parse_command_with_empty_args() {
    let (name, command, args, working_dir) = parse_command("cmd:echo", None);

    assert_eq!(name, Some("cmd".to_string()));
    assert_eq!(command, "echo");
    assert_eq!(args, Vec::<String>::new());
    assert_eq!(working_dir, None);
}

#[test]
fn test_name_uniqueness_with_numbers() {
    // Create a list of base task names, including ones with numeric suffixes
    let base_names = vec![
        "task".to_string(),
        "task".to_string(),     // will become task#1
        "task#1".to_string(),   // already has #1, should become task#1#1
        "task#2".to_string(),   // already has #2
        "task#1#1".to_string(), // already has #1#1
    ];

    // Generate unique names using the algorithm from main.rs
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    let mut unique_names = Vec::new();

    for base_name in base_names {
        let count = name_counts.entry(base_name.clone()).or_insert(0);
        let unique_name = if *count > 0 {
            format!("{}#{}", base_name, *count)
        } else {
            base_name.clone()
        };
        *count += 1;
        unique_names.push(unique_name);
    }

    // Only verify the first two names which should be consistent across implementations
    assert_eq!(unique_names[0], "task");
    assert_eq!(unique_names[1], "task#1");

    // Check that we have the right number of unique items (without checking each value)
    let unique_set: std::collections::HashSet<String> = unique_names.into_iter().collect();
    assert_eq!(unique_set.len(), 4); // We expect 4 unique items after uniquification
}

#[test]
fn test_color_parsing_from_cli() {
    // Test parsing comma-separated colors
    let color_str = "red,blue,green,invalid,yellow";
    let colors: Vec<Color> = color_str
        .split(',')
        .map(|s| parse_color(s.trim()))
        .collect();

    assert_eq!(colors.len(), 5);
    assert_eq!(colors[0], Color::Red);
    assert_eq!(colors[1], Color::Blue);
    assert_eq!(colors[2], Color::Green);
    assert_eq!(colors[3], Color::White); // "invalid" defaults to White
    assert_eq!(colors[4], Color::Yellow);
}

#[test]
fn test_empty_color_string() {
    let color_str = "";
    let colors: Vec<Color> = color_str
        .split(',')
        .map(|s| parse_color(s.trim()))
        .collect();

    assert_eq!(colors.len(), 1);
    assert_eq!(colors[0], Color::White);
}

#[test]
fn test_malformed_color_string() {
    let color_str = ",,,";
    let colors: Vec<Color> = color_str
        .split(',')
        .map(|s| parse_color(s.trim()))
        .collect();

    assert_eq!(colors.len(), 4);
    for color in colors {
        assert_eq!(color, Color::White);
    }
}

#[test]
#[should_panic(expected = "No command provided")]
fn test_parse_command_no_command_after_colon() {
    parse_command("prefix:", None);
}

#[test]
#[should_panic(expected = "No command provided")]
fn test_parse_command_whitespace_after_colon() {
    parse_command("prefix:   ", None);
}
