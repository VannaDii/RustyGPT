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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use assert_fs::TempDir;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;

    use crate::analyzer::{AuditResult, TranslationData};

    #[test]
    fn test_create_backups() -> Result<()> {
        // Create a temporary directory for translation files
        let temp_dir = TempDir::new()?;

        // Create some translation files
        let en_file = temp_dir.child("en.json");
        en_file.write_str(r#"{"key": "value"}"#)?;

        let es_file = temp_dir.child("es.json");
        es_file.write_str(r#"{"key": "valor"}"#)?;

        // Create a non-JSON file that should be ignored
        let txt_file = temp_dir.child("notes.txt");
        txt_file.write_str("Some notes")?;

        // Test create_backups
        create_backups(temp_dir.path())?;

        // Verify backups were created
        let backup_dir = temp_dir.path().join("backups");
        assert!(backup_dir.exists());

        // There should be one timestamped directory inside the backups directory
        let entries = fs::read_dir(&backup_dir)?;
        let timestamped_dirs: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        assert_eq!(timestamped_dirs.len(), 1);

        // Inside the timestamped directory, there should be two JSON files
        let timestamped_dir = &timestamped_dirs[0].path();
        let backup_files: Vec<_> = fs::read_dir(timestamped_dir)?
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(backup_files.len(), 2);

        // Verify the backup files have the correct names
        let backup_filenames: Vec<_> = backup_files
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(backup_filenames.contains(&"en.json".to_string()));
        assert!(backup_filenames.contains(&"es.json".to_string()));

        Ok(())
    }

    #[test]
    fn test_clean_translation_files() -> Result<()> {
        // Create a temporary directory for translation files
        let temp_dir = TempDir::new()?;

        // Create English translation file
        let en_file = temp_dir.child("en.json");
        en_file.write_str(
            r#"
            {
                "common": {
                    "button": {
                        "submit": "Submit",
                        "cancel": "Cancel",
                        "reset": "Reset"
                    }
                },
                "profile": {
                    "title": "Profile"
                },
                "unused": {
                    "key": "Unused"
                }
            }
            "#,
        )?;

        // Create Spanish translation file
        let es_file = temp_dir.child("es.json");
        es_file.write_str(
            r#"
            {
                "common": {
                    "button": {
                        "submit": "Enviar",
                        "cancel": "Cancelar"
                    }
                },
                "profile": {
                    "title": "Perfil"
                },
                "unused": {
                    "key": "No usado"
                }
            }
            "#,
        )?;

        // Create a mock AuditResult
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("common.button.submit".to_string());
        keys_in_use.insert("common.button.cancel".to_string());
        keys_in_use.insert("profile.title".to_string());

        let mut translations = HashMap::new();

        // English translation data
        let en_content: Value = serde_json::from_str(
            r#"
            {
                "common": {
                    "button": {
                        "submit": "Submit",
                        "cancel": "Cancel",
                        "reset": "Reset"
                    }
                },
                "profile": {
                    "title": "Profile"
                },
                "unused": {
                    "key": "Unused"
                }
            }
            "#,
        )?;

        let mut en_all_keys = HashSet::new();
        en_all_keys.insert("common.button.submit".to_string());
        en_all_keys.insert("common.button.cancel".to_string());
        en_all_keys.insert("common.button.reset".to_string());
        en_all_keys.insert("profile.title".to_string());
        en_all_keys.insert("unused.key".to_string());

        let mut en_unused_keys = HashSet::new();
        en_unused_keys.insert("common.button.reset".to_string());
        en_unused_keys.insert("unused.key".to_string());

        let en_data = TranslationData {
            file_path: en_file.path().to_path_buf(),
            all_keys: en_all_keys,
            unused_keys: en_unused_keys,
            content: en_content,
        };
        translations.insert("en".to_string(), en_data);

        // Spanish translation data
        let es_content: Value = serde_json::from_str(
            r#"
            {
                "common": {
                    "button": {
                        "submit": "Enviar",
                        "cancel": "Cancelar"
                    }
                },
                "profile": {
                    "title": "Perfil"
                },
                "unused": {
                    "key": "No usado"
                }
            }
            "#,
        )?;

        let mut es_all_keys = HashSet::new();
        es_all_keys.insert("common.button.submit".to_string());
        es_all_keys.insert("common.button.cancel".to_string());
        es_all_keys.insert("profile.title".to_string());
        es_all_keys.insert("unused.key".to_string());

        let mut es_unused_keys = HashSet::new();
        es_unused_keys.insert("unused.key".to_string());

        let es_data = TranslationData {
            file_path: es_file.path().to_path_buf(),
            all_keys: es_all_keys,
            unused_keys: es_unused_keys,
            content: es_content,
        };
        translations.insert("es".to_string(), es_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test clean_translation_files
        clean_translation_files(temp_dir.path(), &audit_result)?;

        // Verify the files were cleaned
        let en_content = fs::read_to_string(en_file.path())?;
        let en_json: Value = serde_json::from_str(&en_content)?;

        // Check that unused keys were removed from English file
        assert!(get_value_by_path(&en_json, "common.button.submit").is_some());
        assert!(get_value_by_path(&en_json, "common.button.cancel").is_some());
        assert!(get_value_by_path(&en_json, "profile.title").is_some());
        assert!(get_value_by_path(&en_json, "common.button.reset").is_none());
        assert!(get_value_by_path(&en_json, "unused.key").is_none());

        // Check that unused keys were removed from Spanish file
        let es_content = fs::read_to_string(es_file.path())?;
        let es_json: Value = serde_json::from_str(&es_content)?;

        assert!(get_value_by_path(&es_json, "common.button.submit").is_some());
        assert!(get_value_by_path(&es_json, "common.button.cancel").is_some());
        assert!(get_value_by_path(&es_json, "profile.title").is_some());
        assert!(get_value_by_path(&es_json, "unused.key").is_none());

        Ok(())
    }

    #[test]
    fn test_create_translation_templates() -> Result<()> {
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

        // French translation data (missing profile title)
        let fr_content = json!({
            "common": {
                "button": {
                    "submit": "Soumettre",
                    "cancel": "Annuler"
                }
            }
        });

        let mut fr_all_keys = HashSet::new();
        fr_all_keys.insert("common.button.submit".to_string());
        fr_all_keys.insert("common.button.cancel".to_string());

        let fr_data = TranslationData {
            file_path: PathBuf::from("fr.json"),
            all_keys: fr_all_keys,
            unused_keys: HashSet::new(),
            content: fr_content,
        };
        translations.insert("fr".to_string(), fr_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test create_translation_templates
        create_translation_templates(&audit_result, temp_dir.path())?;

        // Verify template files were created
        let es_template_path = temp_dir.path().join("es_missing.json");
        assert!(es_template_path.exists());

        let fr_template_path = temp_dir.path().join("fr_missing.json");
        assert!(fr_template_path.exists());

        // Check content of Spanish template
        let es_template_content = fs::read_to_string(&es_template_path)?;
        let es_template: Value = serde_json::from_str(&es_template_content)?;

        let es_cancel_value = get_value_by_path(&es_template, "common.button.cancel");
        assert!(es_cancel_value.is_some());
        assert_eq!(es_cancel_value.unwrap().as_str().unwrap(), "TODO: Cancel");

        // Check content of French template
        let fr_template_content = fs::read_to_string(&fr_template_path)?;
        let fr_template: Value = serde_json::from_str(&fr_template_content)?;

        let fr_title_value = get_value_by_path(&fr_template, "profile.title");
        assert!(fr_title_value.is_some());
        assert_eq!(fr_title_value.unwrap().as_str().unwrap(), "TODO: Profile");

        Ok(())
    }

    #[test]
    fn test_create_merged_translation() -> Result<()> {
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

        // Test create_merged_translation
        create_merged_translation(&audit_result, "es", temp_dir.path())?;

        // Verify merged file was created
        let es_merged_path = temp_dir.path().join("es_merged.json");
        assert!(es_merged_path.exists());

        // Check content of merged file
        let es_merged_content = fs::read_to_string(&es_merged_path)?;
        let es_merged: Value = serde_json::from_str(&es_merged_content)?;

        // Existing translations should be preserved
        let submit_value = get_value_by_path(&es_merged, "common.button.submit");
        assert!(submit_value.is_some());
        assert_eq!(submit_value.unwrap().as_str().unwrap(), "Enviar");

        let title_value = get_value_by_path(&es_merged, "profile.title");
        assert!(title_value.is_some());
        assert_eq!(title_value.unwrap().as_str().unwrap(), "Perfil");

        // Missing translations should be marked
        let cancel_value = get_value_by_path(&es_merged, "common.button.cancel");
        assert!(cancel_value.is_some());
        assert!(cancel_value
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("MISSING:"));

        Ok(())
    }

    #[test]
    fn test_create_merged_translation_nonexistent_reference() -> Result<()> {
        // Create a mock AuditResult with no reference language
        let audit_result = AuditResult {
            keys_in_use: HashSet::new(),
            translations: HashMap::new(),
            reference_language: "en".to_string(),
        };

        // Test create_merged_translation with non-existent reference language
        let temp_dir = TempDir::new()?;
        let result = create_merged_translation(&audit_result, "es", temp_dir.path());

        // Verify the function returns an error
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Reference language en not found"));

        Ok(())
    }

    #[test]
    fn test_create_merged_translation_nonexistent_language() -> Result<()> {
        // Create a mock AuditResult with only reference language
        let mut translations = HashMap::new();

        // English (reference) translation data
        let en_content = json!({
            "common": {
                "button": {
                    "submit": "Submit"
                }
            }
        });

        let en_data = TranslationData {
            file_path: PathBuf::from("en.json"),
            all_keys: HashSet::from(["common.button.submit".to_string()]),
            unused_keys: HashSet::new(),
            content: en_content,
        };
        translations.insert("en".to_string(), en_data);

        let audit_result = AuditResult {
            keys_in_use: HashSet::new(),
            translations,
            reference_language: "en".to_string(),
        };

        // Test create_merged_translation with non-existent language
        let temp_dir = TempDir::new()?;
        let result = create_merged_translation(&audit_result, "es", temp_dir.path());

        // Verify the function returns an error
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Language es not found"));

        Ok(())
    }

    #[test]
    fn test_create_translation_templates_nonexistent_reference() -> Result<()> {
        // Create a mock AuditResult with no reference language
        let audit_result = AuditResult {
            keys_in_use: HashSet::new(),
            translations: HashMap::new(),
            reference_language: "en".to_string(),
        };

        // Test create_translation_templates with non-existent reference language
        let temp_dir = TempDir::new()?;
        let result = create_translation_templates(&audit_result, temp_dir.path());

        // Verify the function returns an error
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Reference language en not found"));

        Ok(())
    }
}
