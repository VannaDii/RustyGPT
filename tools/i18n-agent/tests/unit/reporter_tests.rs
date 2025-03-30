use anyhow::Result;
use assert_fs::TempDir;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

// Import the modules we're testing
use i18n_agent::analyzer::{AuditResult, TranslationData};
use i18n_agent::reporter::{generate_report, print_audit_report};

// Import test utilities
use crate::common::test_utils::create_test_keys_in_use;

#[test]
fn test_print_audit_report() -> Result<()> {
    // Create a simple AuditResult
    let keys_in_use = create_test_keys_in_use();

    let mut translations = HashMap::new();

    // English translation
    let en_all_keys = HashSet::from([
        "common.button.submit".to_string(),
        "common.button.cancel".to_string(),
        "common.button.reset".to_string(),
        "profile.title".to_string(),
    ]);

    let en_data = TranslationData {
        file_path: PathBuf::from("en.json"),
        all_keys: en_all_keys,
        unused_keys: HashSet::new(),
        content: serde_json::json!({}),
    };
    translations.insert("en".to_string(), en_data);

    // Spanish translation
    let es_all_keys = HashSet::from([
        "common.button.submit".to_string(),
        "common.button.cancel".to_string(),
        "profile.title".to_string(),
    ]);

    let es_data = TranslationData {
        file_path: PathBuf::from("es.json"),
        all_keys: es_all_keys,
        unused_keys: HashSet::new(),
        content: serde_json::json!({}),
    };
    translations.insert("es".to_string(), es_data);

    let audit_result = AuditResult {
        keys_in_use,
        translations,
        reference_language: "en".to_string(),
    };

    // Test print_audit_report (should not panic)
    print_audit_report(&audit_result, "text");

    Ok(())
}

#[test]
fn test_generate_report_text_format() -> Result<()> {
    // Create a test directory
    let temp_dir = TempDir::new()?;

    // Create a simple AuditResult
    let keys_in_use = create_test_keys_in_use();

    let mut translations = HashMap::new();

    // English translation
    let en_all_keys = HashSet::from([
        "common.button.submit".to_string(),
        "common.button.cancel".to_string(),
        "common.button.reset".to_string(),
        "profile.title".to_string(),
    ]);

    let en_data = TranslationData {
        file_path: PathBuf::from("en.json"),
        all_keys: en_all_keys,
        unused_keys: HashSet::new(),
        content: serde_json::json!({}),
    };
    translations.insert("en".to_string(), en_data);

    // Spanish translation
    let es_all_keys = HashSet::from([
        "common.button.submit".to_string(),
        "common.button.cancel".to_string(),
        "profile.title".to_string(),
    ]);

    let es_data = TranslationData {
        file_path: PathBuf::from("es.json"),
        all_keys: es_all_keys,
        unused_keys: HashSet::new(),
        content: serde_json::json!({}),
    };
    translations.insert("es".to_string(), es_data);

    let audit_result = AuditResult {
        keys_in_use,
        translations,
        reference_language: "en".to_string(),
    };

    // Generate text report
    generate_report(&audit_result, temp_dir.path(), "text")?;

    // Verify report was created
    let report_path = temp_dir.path().join("translation_report.txt");
    assert!(report_path.exists());

    // Check content
    let content = fs::read_to_string(&report_path)?;
    assert!(content.contains("Translation Audit Report"));
    assert!(content.contains("en.json"));
    assert!(content.contains("es.json"));

    Ok(())
}

#[test]
fn test_generate_report_json_format() -> Result<()> {
    // Create a test directory
    let temp_dir = TempDir::new()?;

    // Create a simple AuditResult (similar to previous test)
    let keys_in_use = create_test_keys_in_use();

    let mut translations = HashMap::new();

    // English translation
    let en_all_keys = HashSet::from([
        "common.button.submit".to_string(),
        "common.button.cancel".to_string(),
        "common.button.reset".to_string(),
        "profile.title".to_string(),
    ]);

    let en_data = TranslationData {
        file_path: PathBuf::from("en.json"),
        all_keys: en_all_keys,
        unused_keys: HashSet::new(),
        content: serde_json::json!({}),
    };
    translations.insert("en".to_string(), en_data);

    let audit_result = AuditResult {
        keys_in_use,
        translations,
        reference_language: "en".to_string(),
    };

    // Generate JSON report
    generate_report(&audit_result, temp_dir.path(), "json")?;

    // Verify report was created
    let report_path = temp_dir.path().join("translation_report.json");
    assert!(report_path.exists());

    // Check content
    let content = fs::read_to_string(&report_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;

    // Verify JSON structure
    assert!(json.is_object());
    assert!(json.as_object().unwrap().contains_key("summary"));
    assert!(json.as_object().unwrap().contains_key("files"));

    Ok(())
}

#[test]
fn test_generate_report_html_format() -> Result<()> {
    // Create a test directory
    let temp_dir = TempDir::new()?;

    // Create a simple AuditResult (similar to previous test)
    let keys_in_use = create_test_keys_in_use();

    let mut translations = HashMap::new();

    // English translation
    let en_all_keys = HashSet::from([
        "common.button.submit".to_string(),
        "common.button.cancel".to_string(),
        "profile.title".to_string(),
    ]);

    let en_data = TranslationData {
        file_path: PathBuf::from("en.json"),
        all_keys: en_all_keys,
        unused_keys: HashSet::new(),
        content: serde_json::json!({}),
    };
    translations.insert("en".to_string(), en_data);

    let audit_result = AuditResult {
        keys_in_use,
        translations,
        reference_language: "en".to_string(),
    };

    // Generate HTML report
    generate_report(&audit_result, temp_dir.path(), "html")?;

    // Verify report was created
    let report_path = temp_dir.path().join("translation_report.html");
    assert!(report_path.exists());

    // Check content
    let content = fs::read_to_string(&report_path)?;
    assert!(content.contains("<!DOCTYPE html>"));
    assert!(content.contains("<title>Translation Audit Report</title>"));
    assert!(content.contains("<h1>Translation Audit Report</h1>"));

    Ok(())
}

#[test]
fn test_generate_report_invalid_format() -> Result<()> {
    // Create a test directory
    let temp_dir = TempDir::new()?;

    // Create a simple AuditResult
    let keys_in_use = HashSet::new();
    let translations = HashMap::new();

    let audit_result = AuditResult {
        keys_in_use,
        translations,
        reference_language: "en".to_string(),
    };

    // Generate report with invalid format
    let result = generate_report(&audit_result, temp_dir.path(), "invalid");

    // Should return an error
    assert!(result.is_err());

    Ok(())
}
