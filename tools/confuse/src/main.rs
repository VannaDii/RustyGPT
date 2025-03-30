use clap::Parser;
use colored::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::LinesStream;

// Define a Task struct representing a command to run concurrently.
#[derive(Debug, Clone)]
struct Task {
    /// The (optional) name for this task. If None, a name will be derived.
    name: Option<String>,
    /// The command to run (e.g., "cargo")
    command: String,
    /// The command arguments (e.g., ["watch", "-x", "run"])
    args: Vec<String>,
    /// The working directory (applied globally if not overridden per command)
    working_dir: Option<PathBuf>,
}

impl Task {
    /// Derive a base name from the working directory and command.
    fn derive_base_name(&self) -> String {
        if let Some(ref dir) = self.working_dir {
            if let Some(dir_name) = dir.file_name() {
                format!("{}:{}", dir_name.to_string_lossy(), self.command)
            } else {
                self.command.clone()
            }
        } else {
            self.command.clone()
        }
    }
}

// ProcessInfo holds unique display information for a task's output.
#[derive(Debug, Clone)]
struct ProcessInfo {
    /// The unique name used as a log prefix.
    unique_name: String,
    /// The color assigned for log prefixing.
    color: Color,
}

/// Spawns the process for a task and streams its stdout and stderr with a colored prefix.
async fn spawn_and_stream(task: Task, proc_info: ProcessInfo) {
    let mut cmd = Command::new(&task.command);
    cmd.args(&task.args);
    if let Some(ref dir) = task.working_dir {
        cmd.current_dir(dir);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("Failed to spawn process");

    // Handle stdout.
    if let Some(stdout) = child.stdout.take() {
        let lines = BufReader::new(stdout).lines();
        let reader = LinesStream::new(lines);
        let name = proc_info.unique_name.clone();
        let color = proc_info.color;
        tokio::spawn(async move { stream_output(name, color, reader).await });
    }
    // Handle stderr.
    if let Some(stderr) = child.stderr.take() {
        let lines = BufReader::new(stderr).lines();
        let reader = LinesStream::new(lines);
        let name = proc_info.unique_name.clone();
        let color = proc_info.color;
        tokio::spawn(async move { stream_output(name, color, reader).await });
    }

    let status = child.wait().await.expect("Process failed");
    println!("{} exited with status: {:?}", proc_info.unique_name, status);
}

/// Reads lines from the given reader and prints them with the process prefix.
async fn stream_output<R>(name: String, color: Color, mut lines: R)
where
    R: tokio_stream::Stream<Item = Result<String, std::io::Error>> + Unpin,
{
    let icon = icon_for_color(color);
    while let Some(Ok(line)) = lines.next().await {
        // Combine the icon and the task name in the prefix.
        let prefix = format!("[{}:{}]", icon, name).color(color);
        println!("{}: {}", prefix, line);
    }
}

/// CLI definitions using Clap. This closely mimics concurrently’s CLI.
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
    #[arg(short = 'c', long = "prefix-colors")]
    prefix_colors: Option<String>,

    /// Optional working directory to apply to all commands.
    #[arg(short, long)]
    cwd: Option<PathBuf>,
}

/// Map a string to a colored::Color. Defaults to White if unrecognized.
fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        _ => Color::White,
    }
}

/// Parse a command string into a Task. If the command contains a colon,
/// the part before it is used as the task name (unless overridden by CLI names).
fn parse_command(cmd_str: &str, default_cwd: Option<PathBuf>) -> Task {
    // Split the input into a prefix (everything before the colon, if present)
    // and the remainder (the actual command and arguments).
    let (prefix_opt, command_str) = match cmd_str.find(':') {
        Some(idx) => (Some(cmd_str[..idx].trim()), cmd_str[idx + 1..].trim()),
        None => (None, cmd_str.trim()),
    };

    // Parse the command and its arguments using shlex.
    let parts = shlex::split(command_str).expect("Failed to parse command arguments");
    if parts.is_empty() {
        panic!("No command provided in '{}'", cmd_str);
    }

    // Extract name and working directory from the prefix if available.
    let (name, working_dir) = if let Some(prefix) = prefix_opt {
        if let Some(at_index) = prefix.find('@') {
            // The prefix is of the form "<name>@<working_dir>"
            let name_part = prefix[..at_index].trim();
            let cwd_part = prefix[at_index + 1..].trim();
            (
                if name_part.is_empty() {
                    None
                } else {
                    Some(name_part.to_string())
                },
                Some(PathBuf::from(cwd_part)),
            )
        } else {
            // The prefix only specifies the name.
            (
                if prefix.is_empty() {
                    None
                } else {
                    Some(prefix.to_string())
                },
                default_cwd,
            )
        }
    } else {
        (None, default_cwd)
    };

    Task {
        name,
        command: parts[0].clone(),
        args: parts[1..].to_vec(),
        working_dir,
    }
}

/// Returns a unique icon for each color for accessibility, ensuring varied shapes.
fn icon_for_color(color: Color) -> &'static str {
    match color {
        Color::Blue => "🔵",          // Blue circle
        Color::BrightYellow => "🟨",  // Yellow square
        Color::Red => "🔺",           // Red triangle
        Color::BrightCyan => "🔷",    // Cyan diamond
        Color::Green => "🟢",         // Green circle
        Color::BrightMagenta => "🔶", // Bright Magenta gets an orange diamond (unique shape)
        Color::Magenta => "◆",        // Magenta diamond (different style)
        Color::BrightGreen => "🟩",   // Bright Green square
        Color::Cyan => "⬢",           // Cyan hexagon
        Color::BrightRed => "🟥",     // Bright Red square
        Color::Yellow => "🔻",        // Yellow triangle (inverted for contrast)
        Color::BrightBlue => "🟦",    // Bright Blue square
        _ => "⬜",                    // Fallback icon (white square)
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Set up SIGTERM and SIGINT handling for graceful shutdown (Unix only).
    use tokio::signal::unix::{SignalKind, signal};
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to bind SIGTERM");
    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to bind SIGINT");
    tokio::spawn(async move {
        tokio::select! {
            _ = sigterm.recv() => {
                eprintln!("Received SIGTERM. Shutting down gracefully...");
            },
            _ = sigint.recv() => {
                eprintln!("Received SIGINT. Shutting down gracefully...");
            }
        }
        std::process::exit(0);
    });

    // Parse optional names and colors from comma-separated strings.
    let cli_names: Option<Vec<String>> = cli.names.as_ref().map(|s| {
        s.split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>()
    });
    let cli_colors: Option<Vec<Color>> = cli.prefix_colors.as_ref().map(|s| {
        s.split(',')
            .map(|s| parse_color(s.trim()))
            .collect::<Vec<_>>()
    });

    // Define a default color palette.
    let default_colors = [
        Color::Blue,          // Dark blue
        Color::BrightYellow,  // Bright yellow for contrast
        Color::Red,           // Dark red
        Color::BrightCyan,    // Bright cyan for contrast
        Color::Green,         // Dark green
        Color::BrightMagenta, // Bright magenta for contrast
        Color::Magenta,       // Darker magenta
        Color::BrightGreen,   // Bright green for contrast
        Color::Cyan,          // Dark cyan
        Color::BrightRed,     // Bright red for contrast
        Color::Yellow,        // Darker yellow
        Color::BrightBlue,    // Bright blue for contrast
    ];

    // Build tasks from the positional commands.
    let mut tasks: Vec<Task> = cli
        .commands
        .iter()
        .map(|s| parse_command(s, cli.cwd.clone()))
        .collect();

    // Override task names with CLI names if provided.
    if let Some(names) = cli_names {
        for (i, task) in tasks.iter_mut().enumerate() {
            if i < names.len() {
                task.name = Some(names[i].clone());
            }
        }
    }

    // For each task, if no name is provided, derive one.
    for task in tasks.iter_mut() {
        if task.name.is_none() {
            task.name = Some(task.derive_base_name());
        }
    }

    // Ensure unique names by appending incremental numbers for duplicates.
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    let mut tasks_with_names = Vec::new();
    for task in tasks.into_iter() {
        let base_name = task.name.clone().unwrap();
        let count = name_counts.entry(base_name.clone()).or_insert(0);
        let unique_name = if *count > 0 {
            format!("{}#{}", base_name, *count)
        } else {
            base_name.clone()
        };
        *count += 1;
        tasks_with_names.push((task, unique_name));
    }

    // Spawn all tasks concurrently.
    let mut handles = Vec::new();
    for (i, (task, unique_name)) in tasks_with_names.into_iter().enumerate() {
        let color = match &cli_colors {
            Some(colors) if i < colors.len() => colors[i],
            _ => default_colors[i % default_colors.len()],
        };
        let proc_info = ProcessInfo { unique_name, color };
        handles.push(tokio::spawn(spawn_and_stream(task, proc_info)));
    }

    // Wait for all tasks to complete.
    for handle in handles {
        let _ = handle.await;
    }
}
