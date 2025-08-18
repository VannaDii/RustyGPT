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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_generate_config_yaml_format() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        // Change to temp directory for this test
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = generate_config("yaml");
        assert!(result.is_ok());

        // Verify file was created in current directory (temp directory)
        assert!(fs::metadata("config.yaml").is_ok());

        // Verify file content is valid YAML
        let content = fs::read_to_string("config.yaml").unwrap();
        assert!(!content.is_empty());
        assert!(content.contains("port:") || content.contains("host:"));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    #[serial]
    fn test_generate_config_json_format() {
        // Create temporary directory for test
        let temp_dir = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = generate_config("json");
        assert!(result.is_ok());

        // Verify file was created in current directory (temp directory)
        assert!(fs::metadata("config.json").is_ok());

        // Verify file content is valid JSON
        let content = fs::read_to_string("config.json").unwrap();
        assert!(!content.is_empty());
        assert!(content.starts_with('{'));
        assert!(content.ends_with('}') || content.trim_end().ends_with('}'));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_generate_config_unsupported_format() {
        let result = generate_config("xml");
        assert!(result.is_err());

        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Unsupported format"));
        assert!(error_message.contains("yaml") || error_message.contains("json"));
    }

    #[test]
    fn test_generate_config_empty_format() {
        let result = generate_config("");
        assert!(result.is_err());

        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Unsupported format"));
    }

    #[test]
    fn test_generate_config_case_sensitivity() {
        // Test that format matching is case-sensitive
        let result_upper = generate_config("YAML");
        assert!(result_upper.is_err());

        let result_mixed = generate_config("Json");
        assert!(result_mixed.is_err());
    }

    #[test]
    fn test_generate_config_with_special_characters() {
        let result = generate_config("yaml!");
        assert!(result.is_err());

        let result2 = generate_config("json.ext");
        assert!(result2.is_err());
    }

    #[test]
    #[serial]
    fn test_config_serialization_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        // Use a scoped approach to ensure cleanup
        let result = std::panic::catch_unwind(|| {
            std::env::set_current_dir(temp_dir.path()).unwrap();

            // Generate both formats
            let yaml_result = generate_config("yaml");
            let json_result = generate_config("json");

            assert!(yaml_result.is_ok());
            assert!(json_result.is_ok());

            // Both files should exist in current directory (temp directory)
            assert!(fs::metadata("config.yaml").is_ok());
            assert!(fs::metadata("config.json").is_ok());

            // Both should have content
            let yaml_content = fs::read_to_string("config.yaml").unwrap();
            let json_content = fs::read_to_string("config.json").unwrap();

            assert!(!yaml_content.is_empty());
            assert!(!json_content.is_empty());
        });

        // Always restore directory, even on panic
        std::env::set_current_dir(original_dir).unwrap();

        // Re-panic if the test failed
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }
}
