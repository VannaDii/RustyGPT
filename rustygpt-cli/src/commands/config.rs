use std::fs;
use std::io::Write;

use shared::config::server::Config;

/// Generates a configuration file in the specified format.
///
/// # Arguments
/// * `format` - The format of the configuration file ("yaml" or "json").
///
/// # Errors
/// Returns an error if the format is unsupported or if writing the file fails.
pub fn generate_config(format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::with_defaults();
    let file_name = match format {
        "yaml" => "config.yaml",
        "json" => "config.json",
        _ => return Err("Unsupported format. Use 'yaml' or 'json'.".into()),
    };

    let serialized = match format {
        "yaml" => serde_yaml::to_string(&config)?,
        "json" => serde_json::to_string_pretty(&config)?,
        _ => unreachable!(),
    };

    let mut file = fs::File::create(file_name)?;
    file.write_all(serialized.as_bytes())?;

    println!("Configuration file '{}' generated successfully.", file_name);
    Ok(())
}
