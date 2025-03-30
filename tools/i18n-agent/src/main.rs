use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod analyzer;
mod generator;
mod reporter;
mod scanner;

/// i18n-agent: A tool for managing i18n translation files
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Source directory to scan
    #[arg(short, long, default_value = "../../frontend/src")]
    src: PathBuf,

    /// Translations directory
    #[arg(short, long, default_value = "../../frontend/translations")]
    trans: PathBuf,

    /// Output directory for reports and templates
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Create backups before modifying files
    #[arg(short, long)]
    backup: bool,

    /// Output format (json, text, html)
    #[arg(short, long, default_value = "text")]
    format: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan codebase for translation key usage
    Scan,

    /// Analyze translation files without making changes
    Audit,

    /// Remove unused keys from translation files
    Clean,

    /// Generate detailed translation status report
    Report,

    /// Create template files for missing translations
    Template,

    /// Create a merged translation file with all keys from the reference language
    Merge {
        /// Language code to merge with reference language
        #[arg(short, long)]
        lang: String,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    // Validate paths
    let src_dir = cli.src.clone();
    if !src_dir.exists() {
        return Err(anyhow::anyhow!(
            "Source directory does not exist: {:?}",
            src_dir
        ));
    }

    let trans_dir = cli.trans.clone();
    if !trans_dir.exists() {
        return Err(anyhow::anyhow!(
            "Translations directory does not exist: {:?}",
            trans_dir
        ));
    }

    // If output directory is specified, ensure it exists
    if let Some(output_dir) = &cli.output {
        std::fs::create_dir_all(output_dir).context(format!(
            "Failed to create output directory: {:?}",
            output_dir
        ))?;
    }

    // Process command
    match &cli.command {
        Commands::Scan => {
            println!(
                "{}",
                "Scanning codebase for translation key usage...".green()
            );
            let keys = scanner::scan_codebase(&src_dir)?;
            println!("Found {} unique translation keys in use.", keys.len());

            if cli.verbose {
                println!("\nKeys in use:");
                for key in keys {
                    println!("- {}", key);
                }
            }
        }
        Commands::Audit => {
            println!("{}", "Auditing translation files...".green());
            let keys_in_use = scanner::scan_codebase(&src_dir)?;
            let audit_result = analyzer::audit_translations(&trans_dir, &keys_in_use)?;

            reporter::print_audit_report(&audit_result, cli.format.as_str());
        }
        Commands::Clean => {
            println!("{}", "Cleaning translation files...".green());
            let keys_in_use = scanner::scan_codebase(&src_dir)?;
            let audit_result = analyzer::audit_translations(&trans_dir, &keys_in_use)?;

            // Create backups if requested
            if cli.backup {
                generator::create_backups(&trans_dir)?;
            }

            generator::clean_translation_files(&trans_dir, &audit_result)?;
            println!("Translation files cleaned successfully.");
        }
        Commands::Report => {
            println!("{}", "Generating detailed translation report...".green());
            let keys_in_use = scanner::scan_codebase(&src_dir)?;
            let audit_result = analyzer::audit_translations(&trans_dir, &keys_in_use)?;

            let output_dir = cli.output.clone().unwrap_or_else(|| PathBuf::from("."));
            reporter::generate_report(&audit_result, &output_dir, cli.format.as_str())?;

            println!("Report generated successfully in {:?}", output_dir);
        }
        Commands::Template => {
            println!("{}", "Generating translation templates...".green());
            let keys_in_use = scanner::scan_codebase(&src_dir)?;
            let audit_result = analyzer::audit_translations(&trans_dir, &keys_in_use)?;

            let output_dir = cli
                .output
                .clone()
                .unwrap_or_else(|| trans_dir.join("templates"));
            std::fs::create_dir_all(&output_dir)?;

            generator::create_translation_templates(&audit_result, &output_dir)?;

            println!(
                "Translation templates generated successfully in {:?}",
                output_dir
            );
        }
        Commands::Merge { lang } => {
            println!(
                "{}",
                format!("Creating merged translation file for {}...", lang).green()
            );
            let keys_in_use = scanner::scan_codebase(&src_dir)?;
            let audit_result = analyzer::audit_translations(&trans_dir, &keys_in_use)?;

            let output_dir = cli.output.clone().unwrap_or_else(|| trans_dir.clone());
            std::fs::create_dir_all(&output_dir)?;

            generator::create_merged_translation(&audit_result, lang, &output_dir)?;
        }
    }

    Ok(())
}
