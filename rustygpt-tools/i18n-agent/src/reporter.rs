use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::analyzer::{calculate_coverage, get_missing_translations, AuditResult};

/// Prints an audit report to the console
pub fn print_audit_report(audit_result: &AuditResult, format: &str) {
    println!("\n{}", "Translation Audit Report".green().bold());
    println!("{}", "======================".green());

    println!("\n{}", "Translation Files:".yellow());
    for (lang_code, data) in &audit_result.translations {
        let is_reference = lang_code == &audit_result.reference_language;
        let total_keys = data.all_keys.len();
        let unused_keys = data.unused_keys.len();
        let _used_keys = total_keys - unused_keys;

        let missing = if is_reference {
            0
        } else {
            get_missing_translations(audit_result, lang_code).len()
        };

        let coverage = calculate_coverage(audit_result, lang_code);

        let status = if is_reference {
            format!("{} keys (reference)", total_keys).white()
        } else {
            format!("{} keys ({} missing)", total_keys, missing).white()
        };

        println!(
            "- {}.json: {} - {:.1}% coverage, {} unused keys ({}%)",
            lang_code,
            status,
            coverage,
            unused_keys,
            (unused_keys as f64 / total_keys as f64 * 100.0).round()
        );
    }

    println!("\n{}", "Keys in Use:".yellow());
    println!(
        "- Total unique keys in codebase: {}",
        audit_result.keys_in_use.len()
    );

    println!("\n{}", "Unused Keys Summary:".yellow());
    for (lang_code, data) in &audit_result.translations {
        println!(
            "- {}.json: {} unused keys",
            lang_code,
            data.unused_keys.len()
        );

        if !data.unused_keys.is_empty() && format == "text" {
            // Show a few examples of unused keys
            let examples: Vec<_> = data.unused_keys.iter().take(5).collect();
            for key in examples {
                println!("  - {}", key);
            }

            if data.unused_keys.len() > 5 {
                println!("  - ... and {} more", data.unused_keys.len() - 5);
            }
        }
    }

    println!("\n{}", "Missing Translations Summary:".yellow());
    for lang_code in audit_result.translations.keys() {
        if lang_code == &audit_result.reference_language {
            continue;
        }

        let missing = get_missing_translations(audit_result, lang_code);
        println!(
            "- {}.json: {} missing translations",
            lang_code,
            missing.len()
        );

        if !missing.is_empty() && format == "text" {
            // Show a few examples of missing keys
            let examples: Vec<_> = missing.iter().take(5).collect();
            for key in examples {
                println!("  - {}", key);
            }

            if missing.len() > 5 {
                println!("  - ... and {} more", missing.len() - 5);
            }
        }
    }

    println!("\nRun 'i18n-agent report' for a detailed report.");
}

/// Generates a detailed report in the specified format
pub fn generate_report(audit_result: &AuditResult, output_dir: &Path, format: &str) -> Result<()> {
    match format {
        "text" => generate_text_report(audit_result, output_dir),
        "json" => generate_json_report(audit_result, output_dir),
        "html" => generate_html_report(audit_result, output_dir),
        _ => Err(anyhow::anyhow!("Unsupported report format: {}", format)),
    }
}

/// Generates a text report
fn generate_text_report(audit_result: &AuditResult, output_dir: &Path) -> Result<()> {
    let report_path = output_dir.join("translation_report.txt");
    let mut report = String::new();

    report.push_str("Translation Audit Report\n");
    report.push_str("=======================\n\n");

    report.push_str("Translation Files:\n");
    for (lang_code, data) in &audit_result.translations {
        let is_reference = lang_code == &audit_result.reference_language;
        let total_keys = data.all_keys.len();
        let unused_keys = data.unused_keys.len();
        let _used_keys = total_keys - unused_keys;

        let missing = if is_reference {
            0
        } else {
            get_missing_translations(audit_result, lang_code).len()
        };

        let coverage = calculate_coverage(audit_result, lang_code);

        let status = if is_reference {
            format!("{} keys (reference)", total_keys)
        } else {
            format!("{} keys ({} missing)", total_keys, missing)
        };

        report.push_str(&format!(
            "- {}.json: {} - {:.1}% coverage, {} unused keys ({}%)\n",
            lang_code,
            status,
            coverage,
            unused_keys,
            (unused_keys as f64 / total_keys as f64 * 100.0).round()
        ));
    }

    report.push_str("\nKeys in Use:\n");
    report.push_str(&format!(
        "- Total unique keys in codebase: {}\n",
        audit_result.keys_in_use.len()
    ));

    report.push_str("\nUnused Keys:\n");
    for (lang_code, data) in &audit_result.translations {
        report.push_str(&format!(
            "- {}.json: {} unused keys\n",
            lang_code,
            data.unused_keys.len()
        ));

        if !data.unused_keys.is_empty() {
            // List all unused keys
            let mut sorted_keys: Vec<_> = data.unused_keys.iter().collect();
            sorted_keys.sort();

            for key in sorted_keys {
                report.push_str(&format!("  - {}\n", key));
            }

            report.push('\n');
        }
    }

    report.push_str("\nMissing Translations:\n");
    for lang_code in audit_result.translations.keys() {
        if lang_code == &audit_result.reference_language {
            continue;
        }

        let missing = get_missing_translations(audit_result, lang_code);
        report.push_str(&format!(
            "- {}.json: {} missing translations\n",
            lang_code,
            missing.len()
        ));

        if !missing.is_empty() {
            // List all missing keys
            let mut sorted_keys: Vec<_> = missing.iter().collect();
            sorted_keys.sort();

            for key in sorted_keys {
                report.push_str(&format!("  - {}\n", key));
            }

            report.push('\n');
        }
    }

    fs::write(&report_path, report)?;
    println!("Text report generated: {:?}", report_path);

    Ok(())
}

/// Generates a JSON report
fn generate_json_report(audit_result: &AuditResult, output_dir: &Path) -> Result<()> {
    let report_path = output_dir.join("translation_report.json");

    let mut report = serde_json::Map::new();

    // Add summary
    let mut summary = serde_json::Map::new();
    summary.insert(
        "total_keys_in_use".to_string(),
        serde_json::Value::Number(serde_json::Number::from(audit_result.keys_in_use.len())),
    );

    // Add translation files
    let mut files = serde_json::Map::new();
    for (lang_code, data) in &audit_result.translations {
        let mut file_data = serde_json::Map::new();

        let is_reference = lang_code == &audit_result.reference_language;
        let total_keys = data.all_keys.len();
        let unused_keys = data.unused_keys.len();
        let used_keys = total_keys - unused_keys;

        let missing = if is_reference {
            HashSet::new()
        } else {
            get_missing_translations(audit_result, lang_code)
        };

        let coverage = calculate_coverage(audit_result, lang_code);

        file_data.insert(
            "total_keys".to_string(),
            serde_json::Value::Number(serde_json::Number::from(total_keys)),
        );
        file_data.insert(
            "used_keys".to_string(),
            serde_json::Value::Number(serde_json::Number::from(used_keys)),
        );
        file_data.insert(
            "unused_keys_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(unused_keys)),
        );
        file_data.insert(
            "missing_translations_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(missing.len())),
        );
        // Handle the coverage percentage - convert to string to avoid precision issues
        file_data.insert(
            "coverage_percentage".to_string(),
            serde_json::Value::String(format!("{:.1}", coverage)),
        );
        file_data.insert(
            "is_reference".to_string(),
            serde_json::Value::Bool(is_reference),
        );

        // Add unused keys
        let unused_keys_array: Vec<serde_json::Value> = data
            .unused_keys
            .iter()
            .map(|k| serde_json::Value::String(k.clone()))
            .collect();
        file_data.insert(
            "unused_keys".to_string(),
            serde_json::Value::Array(unused_keys_array),
        );

        // Add missing translations
        let missing_keys_array: Vec<serde_json::Value> = missing
            .iter()
            .map(|k| serde_json::Value::String(k.clone()))
            .collect();
        file_data.insert(
            "missing_translations".to_string(),
            serde_json::Value::Array(missing_keys_array),
        );

        files.insert(
            format!("{}.json", lang_code),
            serde_json::Value::Object(file_data),
        );
    }

    report.insert("summary".to_string(), serde_json::Value::Object(summary));
    report.insert("files".to_string(), serde_json::Value::Object(files));

    let json = serde_json::Value::Object(report);
    let json_str = serde_json::to_string_pretty(&json)?;

    fs::write(&report_path, json_str)?;
    println!("JSON report generated: {:?}", report_path);

    Ok(())
}

/// Generates an HTML report
fn generate_html_report(audit_result: &AuditResult, output_dir: &Path) -> Result<()> {
    let report_path = output_dir.join("translation_report.html");

    // Create a basic HTML report with summary information
    let mut html = String::new();

    // Add HTML header
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<title>Translation Audit Report</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
    html.push_str("table { border-collapse: collapse; width: 100%; }\n");
    html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
    html.push_str("th { background-color: #f2f2f2; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");

    // Add report title
    html.push_str("<h1>Translation Audit Report</h1>\n");

    // Add summary section
    html.push_str("<h2>Summary</h2>\n");
    html.push_str("<p>Total unique keys in codebase: ");
    html.push_str(&audit_result.keys_in_use.len().to_string());
    html.push_str("</p>\n");

    // Add translation files table
    html.push_str("<h2>Translation Files</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>File</th><th>Total Keys</th><th>Used Keys</th><th>Unused Keys</th><th>Missing Translations</th><th>Coverage</th></tr>\n");

    for (lang_code, data) in &audit_result.translations {
        let is_reference = lang_code == &audit_result.reference_language;
        let total_keys = data.all_keys.len();
        let unused_keys = data.unused_keys.len();
        let used_keys = total_keys - unused_keys;

        let missing = if is_reference {
            0
        } else {
            get_missing_translations(audit_result, lang_code).len()
        };

        let coverage = calculate_coverage(audit_result, lang_code);

        html.push_str("<tr>");
        html.push_str(&format!(
            "<td>{}.json {}</td>",
            lang_code,
            if is_reference { "(reference)" } else { "" }
        ));
        html.push_str(&format!("<td>{}</td>", total_keys));
        html.push_str(&format!("<td>{}</td>", used_keys));
        html.push_str(&format!("<td>{}</td>", unused_keys));
        html.push_str(&format!("<td>{}</td>", missing));
        html.push_str(&format!("<td>{:.1}%</td>", coverage));
        html.push_str("</tr>\n");
    }

    html.push_str("</table>\n");

    // Close HTML tags
    html.push_str("</body>\n</html>");

    // Write the HTML to file
    fs::write(&report_path, html)?;
    println!("HTML report generated: {:?}", report_path);

    Ok(())
}

#[cfg(test)]
#[allow(clippy::similar_names)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;

    use crate::analyzer::{AuditResult, TranslationData};

    #[test]
    fn test_generate_text_report() -> Result<()> {
        // Create a temporary directory for output
        let temp_dir = TempDir::new()?;

        // Create a mock AuditResult
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("common.button.submit".to_string());
        keys_in_use.insert("common.button.cancel".to_string());
        keys_in_use.insert("profile.title".to_string());

        let mut translations = HashMap::new();

        // English (reference) translation data
        let en_content = json!({
            "common": {
                "button": {
                    "submit": "Submit",
                    "cancel": "Cancel"
                }
            },
            "profile": {
                "title": "Profile"
            }
        });

        let mut en_all_keys = HashSet::new();
        en_all_keys.insert("common.button.submit".to_string());
        en_all_keys.insert("common.button.cancel".to_string());
        en_all_keys.insert("profile.title".to_string());

        let en_data = TranslationData {
            file_path: PathBuf::from("en.json"),
            all_keys: en_all_keys,
            unused_keys: HashSet::new(),
            content: en_content,
        };
        translations.insert("en".to_string(), en_data);

        // Spanish translation data (missing cancel button)
        let es_content = json!({
            "common": {
                "button": {
                    "submit": "Enviar"
                }
            },
            "profile": {
                "title": "Perfil"
            }
        });

        let mut es_all_keys = HashSet::new();
        es_all_keys.insert("common.button.submit".to_string());
        es_all_keys.insert("profile.title".to_string());

        let es_data = TranslationData {
            file_path: PathBuf::from("es.json"),
            all_keys: es_all_keys,
            unused_keys: HashSet::new(),
            content: es_content,
        };
        translations.insert("es".to_string(), es_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test generate_text_report
        generate_text_report(&audit_result, temp_dir.path())?;

        // Verify report file was created
        let report_path = temp_dir.path().join("translation_report.txt");
        assert!(report_path.exists());

        // Check content of report
        let report_content = fs::read_to_string(&report_path)?;

        // Verify report contains expected sections
        assert!(report_content.contains("Translation Audit Report"));
        assert!(report_content.contains("Translation Files:"));
        assert!(report_content.contains("Keys in Use:"));
        assert!(report_content.contains("Unused Keys:"));
        assert!(report_content.contains("Missing Translations:"));

        // Verify report contains language-specific information
        assert!(report_content.contains("en.json"));
        assert!(report_content.contains("es.json"));
        assert!(report_content.contains("missing translations"));

        Ok(())
    }

    #[test]
    fn test_generate_json_report() -> Result<()> {
        // Create a temporary directory for output
        let temp_dir = TempDir::new()?;

        // Create a mock AuditResult (similar to test_generate_text_report)
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("common.button.submit".to_string());
        keys_in_use.insert("common.button.cancel".to_string());
        keys_in_use.insert("profile.title".to_string());

        let mut translations = HashMap::new();

        // English (reference) translation data
        let en_content = json!({
            "common": {
                "button": {
                    "submit": "Submit",
                    "cancel": "Cancel"
                }
            },
            "profile": {
                "title": "Profile"
            }
        });

        let mut en_all_keys = HashSet::new();
        en_all_keys.insert("common.button.submit".to_string());
        en_all_keys.insert("common.button.cancel".to_string());
        en_all_keys.insert("profile.title".to_string());

        let en_data = TranslationData {
            file_path: PathBuf::from("en.json"),
            all_keys: en_all_keys,
            unused_keys: HashSet::new(),
            content: en_content,
        };
        translations.insert("en".to_string(), en_data);

        // Spanish translation data (missing cancel button)
        let es_content = json!({
            "common": {
                "button": {
                    "submit": "Enviar"
                }
            },
            "profile": {
                "title": "Perfil"
            }
        });

        let mut es_all_keys = HashSet::new();
        es_all_keys.insert("common.button.submit".to_string());
        es_all_keys.insert("profile.title".to_string());

        let es_data = TranslationData {
            file_path: PathBuf::from("es.json"),
            all_keys: es_all_keys,
            unused_keys: HashSet::new(),
            content: es_content,
        };
        translations.insert("es".to_string(), es_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test generate_json_report
        generate_json_report(&audit_result, temp_dir.path())?;

        // Verify report file was created
        let report_path = temp_dir.path().join("translation_report.json");
        assert!(report_path.exists());

        // Check content of report
        let report_content = fs::read_to_string(&report_path)?;
        let report_json: serde_json::Value = serde_json::from_str(&report_content)?;

        // Verify report structure
        assert!(report_json.is_object());
        assert!(report_json.as_object().unwrap().contains_key("summary"));
        assert!(report_json.as_object().unwrap().contains_key("files"));

        // Verify summary information
        let summary = &report_json["summary"];
        assert_eq!(summary["total_keys_in_use"], 3);

        // Verify files information
        let files = &report_json["files"];
        assert!(files.as_object().unwrap().contains_key("en.json"));
        assert!(files.as_object().unwrap().contains_key("es.json"));

        // Verify English file details
        let en_file = &files["en.json"];
        assert_eq!(en_file["total_keys"], 3);
        assert_eq!(en_file["used_keys"], 3);
        assert_eq!(en_file["unused_keys_count"], 0);
        assert_eq!(en_file["missing_translations_count"], 0);
        assert_eq!(en_file["coverage_percentage"], "100.0");
        assert_eq!(en_file["is_reference"], true);

        // Verify Spanish file details
        let es_file = &files["es.json"];
        assert_eq!(es_file["total_keys"], 2);
        assert_eq!(es_file["used_keys"], 2);
        assert_eq!(es_file["unused_keys_count"], 0);
        assert_eq!(es_file["missing_translations_count"], 1);
        assert_eq!(es_file["coverage_percentage"], "66.7");
        assert_eq!(es_file["is_reference"], false);

        // Verify missing translations
        let missing_translations = &es_file["missing_translations"].as_array().unwrap();
        assert_eq!(missing_translations.len(), 1);
        assert_eq!(missing_translations[0], "common.button.cancel");

        Ok(())
    }

    #[test]
    fn test_generate_html_report() -> Result<()> {
        // Create a temporary directory for output
        let temp_dir = TempDir::new()?;

        // Create a mock AuditResult (similar to previous tests)
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("common.button.submit".to_string());
        keys_in_use.insert("common.button.cancel".to_string());
        keys_in_use.insert("profile.title".to_string());

        let mut translations = HashMap::new();

        // English (reference) translation data
        let en_content = json!({
            "common": {
                "button": {
                    "submit": "Submit",
                    "cancel": "Cancel"
                }
            },
            "profile": {
                "title": "Profile"
            }
        });

        let mut en_all_keys = HashSet::new();
        en_all_keys.insert("common.button.submit".to_string());
        en_all_keys.insert("common.button.cancel".to_string());
        en_all_keys.insert("profile.title".to_string());

        let en_data = TranslationData {
            file_path: PathBuf::from("en.json"),
            all_keys: en_all_keys,
            unused_keys: HashSet::new(),
            content: en_content,
        };
        translations.insert("en".to_string(), en_data);

        // Spanish translation data (missing cancel button)
        let es_content = json!({
            "common": {
                "button": {
                    "submit": "Enviar"
                }
            },
            "profile": {
                "title": "Perfil"
            }
        });

        let mut es_all_keys = HashSet::new();
        es_all_keys.insert("common.button.submit".to_string());
        es_all_keys.insert("profile.title".to_string());

        let es_data = TranslationData {
            file_path: PathBuf::from("es.json"),
            all_keys: es_all_keys,
            unused_keys: HashSet::new(),
            content: es_content,
        };
        translations.insert("es".to_string(), es_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test generate_html_report
        generate_html_report(&audit_result, temp_dir.path())?;

        // Verify report file was created
        let report_path = temp_dir.path().join("translation_report.html");
        assert!(report_path.exists());

        // Check content of report
        let report_content = fs::read_to_string(&report_path)?;

        // Verify report contains expected sections
        assert!(report_content.contains("<!DOCTYPE html>"));
        assert!(report_content.contains("<title>Translation Audit Report</title>"));
        assert!(report_content.contains("<h1>Translation Audit Report</h1>"));
        assert!(report_content.contains("<h2>Summary</h2>"));
        assert!(report_content.contains("<h2>Translation Files</h2>"));
        assert!(report_content.contains("<table>"));

        // Verify report contains language-specific information
        assert!(report_content.contains("en.json"));
        assert!(report_content.contains("es.json"));
        assert!(report_content.contains("(reference)"));

        Ok(())
    }

    #[test]
    fn test_generate_report_invalid_format() -> Result<()> {
        // Create a temporary directory for output
        let temp_dir = TempDir::new()?;

        // Create a minimal AuditResult
        let keys_in_use = HashSet::new();
        let translations = HashMap::new();
        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test generate_report with invalid format
        let result = generate_report(&audit_result, temp_dir.path(), "invalid_format");

        // Should return an error
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported report format"));

        Ok(())
    }

    #[test]
    fn test_generate_report_text_format() -> Result<()> {
        // Create a temporary directory for output
        let temp_dir = TempDir::new()?;

        // Create a minimal AuditResult
        let keys_in_use = HashSet::new();
        let translations = HashMap::new();
        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test generate_report with text format
        let result = generate_report(&audit_result, temp_dir.path(), "text");
        assert!(result.is_ok());

        // Verify report file was created
        let report_path = temp_dir.path().join("translation_report.txt");
        assert!(report_path.exists());

        Ok(())
    }

    #[test]
    fn test_generate_report_json_format() -> Result<()> {
        // Create a temporary directory for output
        let temp_dir = TempDir::new()?;

        // Create a minimal AuditResult
        let keys_in_use = HashSet::new();
        let translations = HashMap::new();
        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test generate_report with json format
        let result = generate_report(&audit_result, temp_dir.path(), "json");
        assert!(result.is_ok());

        // Verify report file was created
        let report_path = temp_dir.path().join("translation_report.json");
        assert!(report_path.exists());

        Ok(())
    }

    #[test]
    fn test_generate_report_html_format() -> Result<()> {
        // Create a temporary directory for output
        let temp_dir = TempDir::new()?;

        // Create a minimal AuditResult
        let keys_in_use = HashSet::new();
        let translations = HashMap::new();
        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test generate_report with html format
        let result = generate_report(&audit_result, temp_dir.path(), "html");
        assert!(result.is_ok());

        // Verify report file was created
        let report_path = temp_dir.path().join("translation_report.html");
        assert!(report_path.exists());

        Ok(())
    }

    #[test]
    fn test_print_audit_report_basic() -> Result<()> {
        // Create a basic AuditResult for testing print_audit_report
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("common.title".to_string());

        let mut translations = HashMap::new();

        // English translation data
        let en_content = json!({
            "common": {
                "title": "Title"
            }
        });

        let mut en_all_keys = HashSet::new();
        en_all_keys.insert("common.title".to_string());

        let en_data = TranslationData {
            file_path: PathBuf::from("en.json"),
            all_keys: en_all_keys,
            unused_keys: HashSet::new(),
            content: en_content,
        };
        translations.insert("en".to_string(), en_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test print_audit_report - this should not panic
        print_audit_report(&audit_result, "text");

        Ok(())
    }

    #[test]
    fn test_print_audit_report_empty_translations() -> Result<()> {
        // Create an AuditResult with no translations
        let keys_in_use = HashSet::new();
        let translations = HashMap::new();
        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test print_audit_report with empty data - should not panic
        print_audit_report(&audit_result, "text");

        Ok(())
    }

    #[test]
    fn test_print_audit_report_json_format() -> Result<()> {
        // Create a basic AuditResult
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("test.key".to_string());

        let mut translations = HashMap::new();

        let en_content = json!({
            "test": {
                "key": "Test"
            }
        });

        let mut en_all_keys = HashSet::new();
        en_all_keys.insert("test.key".to_string());

        let en_data = TranslationData {
            file_path: PathBuf::from("en.json"),
            all_keys: en_all_keys,
            unused_keys: HashSet::new(),
            content: en_content,
        };
        translations.insert("en".to_string(), en_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test print_audit_report with json format
        print_audit_report(&audit_result, "json");

        Ok(())
    }

    #[test]
    fn test_print_audit_report_with_unused_keys() -> Result<()> {
        // Create AuditResult with some unused keys
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("used.key".to_string());

        let mut translations = HashMap::new();

        let en_content = json!({
            "used": {
                "key": "Used"
            },
            "unused": {
                "key1": "Unused 1",
                "key2": "Unused 2"
            }
        });

        let mut en_all_keys = HashSet::new();
        en_all_keys.insert("used.key".to_string());
        en_all_keys.insert("unused.key1".to_string());
        en_all_keys.insert("unused.key2".to_string());

        let mut unused_keys = HashSet::new();
        unused_keys.insert("unused.key1".to_string());
        unused_keys.insert("unused.key2".to_string());

        let en_data = TranslationData {
            file_path: PathBuf::from("en.json"),
            all_keys: en_all_keys,
            unused_keys,
            content: en_content,
        };
        translations.insert("en".to_string(), en_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test print_audit_report with unused keys
        print_audit_report(&audit_result, "text");

        Ok(())
    }

    #[test]
    fn test_print_audit_report_with_missing_translations() -> Result<()> {
        // Create AuditResult with missing translations
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("common.title".to_string());
        keys_in_use.insert("common.subtitle".to_string());

        let mut translations = HashMap::new();

        // English (reference) has both keys
        let en_content = json!({
            "common": {
                "title": "Title",
                "subtitle": "Subtitle"
            }
        });

        let mut en_all_keys = HashSet::new();
        en_all_keys.insert("common.title".to_string());
        en_all_keys.insert("common.subtitle".to_string());

        let en_data = TranslationData {
            file_path: PathBuf::from("en.json"),
            all_keys: en_all_keys,
            unused_keys: HashSet::new(),
            content: en_content,
        };
        translations.insert("en".to_string(), en_data);

        // Spanish is missing subtitle
        let es_content = json!({
            "common": {
                "title": "Título"
            }
        });

        let mut es_all_keys = HashSet::new();
        es_all_keys.insert("common.title".to_string());

        let es_data = TranslationData {
            file_path: PathBuf::from("es.json"),
            all_keys: es_all_keys,
            unused_keys: HashSet::new(),
            content: es_content,
        };
        translations.insert("es".to_string(), es_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test print_audit_report with missing translations
        print_audit_report(&audit_result, "text");

        Ok(())
    }
}
