//! Command to generate the OpenAPI specification and write it to a file.

use crate::openapi::ApiDoc;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use utoipa::OpenApi;

/// Generates the OpenAPI specification and writes it to the specified output path or streams it to stdout.
///
/// # Arguments
/// * `output_path` - The path where the OpenAPI spec will be written. The format (YAML or JSON)
///   is determined by the file extension. If no path is provided, it streams YAML to stdout.
///   If the path is "json" or "yaml", it streams the spec in the respective format to stdout.
///
/// # Errors
/// Returns an error if the file cannot be written.
pub fn generate_spec(output_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let openapi = ApiDoc::openapi();

    match output_path {
        Some("json") => {
            let json = openapi.to_json()?;
            io::stdout().write_all(json.as_bytes())?;
        }
        Some("yaml") => {
            let yaml = openapi.to_yaml()?;
            io::stdout().write_all(yaml.as_bytes())?;
        }
        Some(path) => {
            let path = Path::new(path);
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("yaml");

            match extension {
                "json" => {
                    let json = openapi.to_json()?;
                    fs::write(path, json)?;
                }
                _ => {
                    let yaml = openapi.to_yaml()?;
                    fs::write(path, yaml)?;
                }
            }

            println!("OpenAPI spec written to {}", path.display());
        }
        None => {
            let yaml = openapi.to_yaml()?;
            io::stdout().write_all(yaml.as_bytes())?;
        }
    }

    Ok(())
}
