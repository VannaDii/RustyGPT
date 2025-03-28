use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

use crate::analyzer::{
    get_missing_translations, get_value_by_path, remove_key_from_json, set_value_by_path,
    AuditResult,
};

/// Creates backups of translation files
pub fn create_backups(trans_dir: &Path) -> Result<()> {
    // Create backup directory
    let backup_dir = trans_dir.join("backups");
    fs::create_dir_all(&backup_dir)?;

    // Get current timestamp for backup folder name
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
    let timestamped_backup_dir = backup_dir.join(timestamp);
    fs::create_dir_all(&timestamped_backup_dir)?;

    // Copy all JSON files to backup directory
    for entry in fs::read_dir(trans_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            let file_name = path.file_name().unwrap();
            let backup_path = timestamped_backup_dir.join(file_name);

            fs::copy(&path, &backup_path)?;
            println!("Backed up {:?} to {:?}", path, backup_path);
        }
    }

    Ok(())
}

/// Cleans translation files by removing unused keys
pub fn clean_translation_files(_trans_dir: &Path, audit_result: &AuditResult) -> Result<()> {
    for (lang_code, data) in &audit_result.translations {
        if data.unused_keys.is_empty() {
            println!("No unused keys to remove from {}.json", lang_code);
            continue;
        }

        // Clone the content to avoid modifying the original
        let mut content = data.content.clone();
        let mut removed_count = 0;

        // Remove each unused key
        for key in &data.unused_keys {
            if remove_key_from_json(&mut content, key) {
                removed_count += 1;
            }
        }

        // Ensure we're not removing all keys
        let used_keys = audit_result.keys_in_use.intersection(&data.all_keys);
        if used_keys.count() == 0
            && !content
                .as_object()
                .unwrap_or(&serde_json::Map::new())
                .is_empty()
        {
            println!(
                "Warning: Would remove all keys from {}.json, skipping to avoid data loss",
                lang_code
            );
            continue;
        }

        // Write the cleaned content back to the file
        let file_path = &data.file_path;
        let json_str = serde_json::to_string_pretty(&content)?;
        fs::write(file_path, json_str)?;

        println!(
            "Removed {} unused keys from {}.json",
            removed_count, lang_code
        );
    }

    Ok(())
}

/// Creates template files for missing translations
pub fn create_translation_templates(audit_result: &AuditResult, output_dir: &Path) -> Result<()> {
    let reference_lang = &audit_result.reference_language;

    if !audit_result.translations.contains_key(reference_lang) {
        return Err(anyhow::anyhow!(
            "Reference language {} not found in translations",
            reference_lang
        ));
    }

    let reference_data = &audit_result.translations[reference_lang];

    for lang_code in audit_result.translations.keys() {
        if lang_code == reference_lang {
            continue;
        }

        let missing = get_missing_translations(audit_result, lang_code);

        if missing.is_empty() {
            println!("No missing translations for {}.json", lang_code);
            continue;
        }

        // Create a template with missing keys
        let mut template = Value::Object(Map::new());

        for key in &missing {
            if let Some(value) = get_value_by_path(&reference_data.content, key) {
                // Create a template entry with the reference value as a comment
                let template_value =
                    Value::String(format!("TODO: {}", value.as_str().unwrap_or("?")));
                set_value_by_path(&mut template, key, template_value);
            }
        }

        // Write the template to a file
        let template_path = output_dir.join(format!("{}_missing.json", lang_code));
        let json_str = serde_json::to_string_pretty(&template)?;
        fs::write(&template_path, json_str).context(format!(
            "Failed to write template file: {:?}",
            template_path
        ))?;

        println!(
            "Created template for {}.json with {} missing translations: {:?}",
            lang_code,
            missing.len(),
            template_path
        );
    }

    Ok(())
}

/// Creates a merged translation file with all keys from the reference language
pub fn create_merged_translation(
    audit_result: &AuditResult,
    lang_code: &str,
    output_dir: &Path,
) -> Result<()> {
    let reference_lang = &audit_result.reference_language;

    if !audit_result.translations.contains_key(reference_lang) {
        return Err(anyhow::anyhow!(
            "Reference language {} not found in translations",
            reference_lang
        ));
    }

    if !audit_result.translations.contains_key(lang_code) {
        return Err(anyhow::anyhow!(
            "Language {} not found in translations",
            lang_code
        ));
    }

    let reference_data = &audit_result.translations[reference_lang];
    let lang_data = &audit_result.translations[lang_code];

    // Start with a copy of the reference content
    let mut merged = reference_data.content.clone();

    // Replace values with translations where available
    for key in &lang_data.all_keys {
        if let Some(value) = get_value_by_path(&lang_data.content, key) {
            set_value_by_path(&mut merged, key, value.clone());
        }
    }

    // Mark missing translations
    let missing = get_missing_translations(audit_result, lang_code);
    for key in &missing {
        if let Some(value) = get_value_by_path(&merged, key) {
            let missing_value =
                Value::String(format!("MISSING: {}", value.as_str().unwrap_or("?")));
            set_value_by_path(&mut merged, key, missing_value);
        }
    }

    // Write the merged file
    let merged_path = output_dir.join(format!("{}_merged.json", lang_code));
    let json_str = serde_json::to_string_pretty(&merged)?;
    fs::write(&merged_path, json_str)?;

    println!(
        "Created merged translation file for {}.json: {:?}",
        lang_code, merged_path
    );

    Ok(())
}
