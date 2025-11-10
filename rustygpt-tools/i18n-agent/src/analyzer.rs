use anyhow::Result;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

/// Represents the result of a translation audit
#[derive(Debug)]
pub struct AuditResult {
    /// Reference to the keys actually used in the codebase
    pub keys_in_use: HashSet<String>,

    /// Map of language code to translation file data
    pub translations: HashMap<String, TranslationData>,

    /// The reference language (usually "en")
    pub reference_language: String,
}

/// Data for a single translation file
#[derive(Debug)]
pub struct TranslationData {
    /// Path to the translation file
    pub file_path: PathBuf,

    /// All keys in the translation file
    pub all_keys: HashSet<String>,

    /// Keys in the translation file that are not used in the codebase
    pub unused_keys: HashSet<String>,

    /// The parsed JSON content of the translation file
    pub content: Value,
}

/// Audits translation files against the keys used in the codebase.
///
/// # Errors
///
/// Returns an error when the translations directory cannot be read, a file cannot
/// be read from disk, or a translation file contains invalid JSON.
pub fn audit_translations<S: BuildHasher>(
    trans_dir: &Path,
    keys_in_use: &HashSet<String, S>,
) -> Result<AuditResult> {
    let mut translations = HashMap::new();
    let keys_in_use_owned: HashSet<String> = keys_in_use.iter().cloned().collect();
    let reference_language = "en".to_string(); // Assuming English is the reference language

    // Find all JSON files in the translations directory
    for entry in fs::read_dir(trans_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let lang_code = stem.to_string();

            // Parse the JSON file
            let content = fs::read_to_string(&path)?;
            let json_value: Value = serde_json::from_str(&content)?;

            // Extract all keys from the JSON
            let all_keys = extract_keys_from_json(&json_value, "");

            // Determine unused keys
            let unused_keys: HashSet<_> =
                all_keys.difference(&keys_in_use_owned).cloned().collect();

            translations.insert(
                lang_code,
                TranslationData {
                    file_path: path,
                    all_keys,
                    unused_keys,
                    content: json_value,
                },
            );
        }
    }

    Ok(AuditResult {
        keys_in_use: keys_in_use_owned,
        translations,
        reference_language,
    })
}

/// Extracts all keys from a JSON object with their full path using an iterative approach
#[must_use]
pub fn extract_keys_from_json(json: &Value, initial_prefix: &str) -> HashSet<String> {
    let mut keys = HashSet::new();
    let mut stack = Vec::new();

    // Start with the root object and initial prefix
    if let Value::Object(map) = json {
        for (key, value) in map {
            let prefix = if initial_prefix.is_empty() {
                key.clone()
            } else {
                format!("{initial_prefix}.{key}")
            };
            stack.push((prefix, value));
        }
    }

    // Process the stack iteratively
    while let Some((prefix, value)) = stack.pop() {
        match value {
            Value::Object(obj) => {
                if obj.is_empty() {
                    // Empty object, treat as a leaf
                    keys.insert(prefix);
                } else {
                    // Non-empty object, add its children to the stack
                    for (key, child_value) in obj {
                        let new_prefix = format!("{prefix}.{key}");
                        stack.push((new_prefix, child_value));
                    }
                }
            }
            _ => {
                // Non-object value, add as a leaf key
                keys.insert(prefix);
            }
        }
    }

    keys
}

/// Gets missing translations for a language compared to the reference language
#[must_use]
pub fn get_missing_translations(audit_result: &AuditResult, lang_code: &str) -> HashSet<String> {
    let reference_lang = &audit_result.reference_language;

    if lang_code == reference_lang {
        return HashSet::new(); // Reference language has no missing translations by definition
    }

    if let (Some(ref_data), Some(lang_data)) = (
        audit_result.translations.get(reference_lang),
        audit_result.translations.get(lang_code),
    ) {
        // Find keys that are in the reference language but not in this language
        let used_ref_keys: HashSet<_> = ref_data
            .all_keys
            .intersection(&audit_result.keys_in_use)
            .cloned()
            .collect();

        used_ref_keys
            .difference(&lang_data.all_keys)
            .cloned()
            .collect()
    } else {
        HashSet::new()
    }
}

/// Calculates translation coverage percentage
#[must_use]
#[allow(clippy::cast_precision_loss)] // Translation key counts remain far below f64 precision limits.
pub fn calculate_coverage(audit_result: &AuditResult, lang_code: &str) -> f64 {
    let reference_lang = &audit_result.reference_language;

    if lang_code == reference_lang {
        return 100.0; // Reference language has 100% coverage by definition
    }

    if let (Some(ref_data), Some(lang_data)) = (
        audit_result.translations.get(reference_lang),
        audit_result.translations.get(lang_code),
    ) {
        let used_ref_keys: HashSet<_> = ref_data
            .all_keys
            .intersection(&audit_result.keys_in_use)
            .collect();

        let used_lang_keys: HashSet<_> = lang_data
            .all_keys
            .intersection(&audit_result.keys_in_use)
            .collect();

        let total_keys = used_ref_keys.len();
        if total_keys == 0 {
            return 100.0;
        }

        let translated_keys = used_lang_keys.len() as f64;
        let total_keys = total_keys as f64;
        (translated_keys / total_keys) * 100.0
    } else {
        0.0
    }
}

/// Removes a key from a JSON object
pub fn remove_key_from_json(json: &mut Value, key_path: &str) -> bool {
    let parts: Vec<&str> = key_path.split('.').collect();

    // Handle empty path
    if parts.is_empty() {
        return false;
    }

    // Navigate to the parent object of the key to remove
    let parent_path = &parts[0..parts.len() - 1];
    let key_to_remove = parts[parts.len() - 1];

    // If it's a top-level key
    if parent_path.is_empty() {
        if let Value::Object(map) = json {
            return map.remove(key_to_remove).is_some();
        }
        return false;
    }

    // Navigate to the parent object
    let mut current = json;
    for &part in parent_path {
        if let Value::Object(map) = current {
            if let Some(next) = map.get_mut(part) {
                current = next;
            } else {
                // Path doesn't exist
                return false;
            }
        } else {
            // Not an object, can't navigate further
            return false;
        }
    }

    // Remove the key from the parent object
    if let Value::Object(map) = current {
        let removed = map.remove(key_to_remove).is_some();

        // We don't need to clean up empty parent objects for this use case
        // as it's not critical for the translation files

        return removed;
    }

    false
}

/// Gets a nested value from a JSON object by key path (already iterative)
#[must_use]
pub fn get_value_by_path<'a>(json: &'a Value, key_path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = key_path.split('.').collect();
    let mut current = json;

    for part in parts {
        match current {
            Value::Object(map) => {
                if let Some(value) = map.get(part) {
                    current = value;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Sets a nested value in a JSON object by key path (already iterative)
pub fn set_value_by_path(json: &mut Value, key_path: &str, new_value: Value) -> bool {
    let parts: Vec<&str> = key_path.split('.').collect();
    let mut current = json;

    // Navigate to the parent object
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part, set the value
            if let Value::Object(map) = current {
                map.insert((*part).to_string(), new_value);
                return true;
            }
            return false;
        }

        // Navigate deeper
        if let Value::Object(map) = current {
            if !map.contains_key(*part) {
                // Create intermediate objects if they don't exist
                map.insert((*part).to_string(), Value::Object(Map::new()));
            }

            if let Some(next) = map.get_mut(*part) {
                current = next;
                continue;
            }
            return false;
        }
        return false;
    }

    false
}

#[cfg(test)]
#[allow(clippy::similar_names, clippy::unnecessary_wraps)] // Tests return Result for ergonomic use of the ? operator.
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use serde_json::json;

    #[test]
    fn test_extract_keys_from_json_flat() -> Result<()> {
        let json = json!({
            "common": {
                "button": {
                    "submit": "Submit",
                    "cancel": "Cancel"
                },
                "error": {
                    "required": "This field is required"
                }
            },
            "profile": {
                "title": "User Profile"
            }
        });

        let keys = extract_keys_from_json(&json, "");

        assert_eq!(keys.len(), 4);
        assert!(keys.contains("common.button.submit"));
        assert!(keys.contains("common.button.cancel"));
        assert!(keys.contains("common.error.required"));
        assert!(keys.contains("profile.title"));

        Ok(())
    }

    #[test]
    fn test_extract_keys_from_json_with_prefix() -> Result<()> {
        let json = json!({
            "button": {
                "submit": "Submit",
                "cancel": "Cancel"
            }
        });

        let keys = extract_keys_from_json(&json, "common");

        assert_eq!(keys.len(), 2);
        assert!(keys.contains("common.button.submit"));
        assert!(keys.contains("common.button.cancel"));

        Ok(())
    }

    #[test]
    fn test_extract_keys_from_json_empty_object() -> Result<()> {
        let json = json!({
            "empty": {}
        });

        let keys = extract_keys_from_json(&json, "");

        assert_eq!(keys.len(), 1);
        assert!(keys.contains("empty"));

        Ok(())
    }

    #[test]
    fn test_extract_keys_from_json_array_values() -> Result<()> {
        let json = json!({
            "items": {
                "list": ["item1", "item2"]
            }
        });

        let keys = extract_keys_from_json(&json, "");

        assert_eq!(keys.len(), 1);
        assert!(keys.contains("items.list"));

        Ok(())
    }

    #[test]
    fn test_extract_keys_from_json_non_object() -> Result<()> {
        let json = json!("not an object");

        let keys = extract_keys_from_json(&json, "");

        assert_eq!(keys.len(), 0);

        Ok(())
    }

    #[test]
    fn test_audit_translations() -> Result<()> {
        // Create a temporary directory for translation files
        let temp_dir = TempDir::new()?;

        // Create English translation file (reference language)
        let english_file = temp_dir.child("en.json");
        english_file.write_str(
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
                }
            }
            "#,
        )?;

        // Create Spanish translation file (missing some keys)
        let spanish_file = temp_dir.child("es.json");
        spanish_file.write_str(
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
                    "key": "Unused Key"
                }
            }
            "#,
        )?;

        // Define keys in use
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("common.button.submit".to_string());
        keys_in_use.insert("common.button.cancel".to_string());
        keys_in_use.insert("profile.title".to_string());
        // Note: common.button.reset is not in use

        // Test audit_translations
        let audit_result = audit_translations(temp_dir.path(), &keys_in_use)?;

        // Verify results
        assert_eq!(audit_result.reference_language, "en");
        assert_eq!(audit_result.translations.len(), 2);
        assert!(audit_result.translations.contains_key("en"));
        assert!(audit_result.translations.contains_key("es"));

        // Check English translation data
        let english_translation = &audit_result.translations["en"];
        assert_eq!(english_translation.all_keys.len(), 4);
        assert_eq!(english_translation.unused_keys.len(), 1);
        assert!(
            english_translation
                .unused_keys
                .contains("common.button.reset")
        );

        // Check Spanish translation data
        let spanish_translation = &audit_result.translations["es"];
        assert_eq!(spanish_translation.all_keys.len(), 4);
        assert_eq!(spanish_translation.unused_keys.len(), 1);
        assert!(spanish_translation.unused_keys.contains("unused.key"));

        Ok(())
    }

    #[test]
    fn test_get_missing_translations() -> Result<()> {
        // Create a simple audit result
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("common.button.submit".to_string());
        keys_in_use.insert("common.button.cancel".to_string());
        keys_in_use.insert("profile.title".to_string());

        let mut translations = HashMap::new();

        // English has all keys
        let mut english_all_keys = HashSet::new();
        english_all_keys.insert("common.button.submit".to_string());
        english_all_keys.insert("common.button.cancel".to_string());
        english_all_keys.insert("profile.title".to_string());
        english_all_keys.insert("unused.key".to_string());

        let english_data = TranslationData {
            file_path: PathBuf::from("en.json"),
            all_keys: english_all_keys,
            unused_keys: HashSet::from(["unused.key".to_string()]),
            content: json!({}),
        };
        translations.insert("en".to_string(), english_data);

        // Spanish is missing some keys
        let mut spanish_all_keys = HashSet::new();
        spanish_all_keys.insert("common.button.submit".to_string());
        spanish_all_keys.insert("profile.title".to_string());

        let spanish_data = TranslationData {
            file_path: PathBuf::from("es.json"),
            all_keys: spanish_all_keys,
            unused_keys: HashSet::new(),
            content: json!({}),
        };
        translations.insert("es".to_string(), spanish_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test get_missing_translations
        let missing_english = get_missing_translations(&audit_result, "en");
        let missing_spanish = get_missing_translations(&audit_result, "es");
        let missing_french = get_missing_translations(&audit_result, "fr"); // Non-existent language

        // Verify results
        assert_eq!(missing_english.len(), 0); // Reference language has no missing translations
        assert_eq!(missing_spanish.len(), 1);
        assert!(missing_spanish.contains("common.button.cancel"));
        assert_eq!(missing_french.len(), 0); // Non-existent language returns empty set

        Ok(())
    }

    #[test]
    fn test_calculate_coverage() -> Result<()> {
        // Create a simple audit result
        let mut keys_in_use = HashSet::new();
        keys_in_use.insert("common.button.submit".to_string());
        keys_in_use.insert("common.button.cancel".to_string());
        keys_in_use.insert("profile.title".to_string());
        keys_in_use.insert("profile.description".to_string());

        let mut translations = HashMap::new();

        // English has all keys
        let mut english_all_keys = HashSet::new();
        english_all_keys.insert("common.button.submit".to_string());
        english_all_keys.insert("common.button.cancel".to_string());
        english_all_keys.insert("profile.title".to_string());
        english_all_keys.insert("profile.description".to_string());

        let english_data = TranslationData {
            file_path: PathBuf::from("en.json"),
            all_keys: english_all_keys,
            unused_keys: HashSet::new(),
            content: json!({}),
        };
        translations.insert("en".to_string(), english_data);

        // Spanish has 50% coverage
        let mut spanish_all_keys = HashSet::new();
        spanish_all_keys.insert("common.button.submit".to_string());
        spanish_all_keys.insert("profile.title".to_string());

        let spanish_data = TranslationData {
            file_path: PathBuf::from("es.json"),
            all_keys: spanish_all_keys,
            unused_keys: HashSet::new(),
            content: json!({}),
        };
        translations.insert("es".to_string(), spanish_data);

        // French has 75% coverage
        let mut fr_all_keys = HashSet::new();
        fr_all_keys.insert("common.button.submit".to_string());
        fr_all_keys.insert("common.button.cancel".to_string());
        fr_all_keys.insert("profile.title".to_string());

        let fr_data = TranslationData {
            file_path: PathBuf::from("fr.json"),
            all_keys: fr_all_keys,
            unused_keys: HashSet::new(),
            content: json!({}),
        };
        translations.insert("fr".to_string(), fr_data);

        let audit_result = AuditResult {
            keys_in_use,
            translations,
            reference_language: "en".to_string(),
        };

        // Test calculate_coverage
        let english_coverage = calculate_coverage(&audit_result, "en");
        let spanish_coverage = calculate_coverage(&audit_result, "es");
        let french_coverage = calculate_coverage(&audit_result, "fr");
        let german_coverage = calculate_coverage(&audit_result, "de"); // Non-existent language

        // Verify results
        assert!(
            (english_coverage - 100.0).abs() < f64::EPSILON,
            "expected 100%, got {english_coverage}"
        );
        assert!(
            (spanish_coverage - 50.0).abs() < f64::EPSILON,
            "expected 50%, got {spanish_coverage}"
        );
        assert!(
            (french_coverage - 75.0).abs() < f64::EPSILON,
            "expected 75%, got {french_coverage}"
        );
        assert!(
            german_coverage.abs() < f64::EPSILON,
            "expected 0%, got {german_coverage}"
        );

        Ok(())
    }

    #[test]
    fn test_remove_key_from_json() -> Result<()> {
        // Test removing a top-level key
        let mut json = json!({
            "key1": "value1",
            "key2": "value2"
        });
        let removed = remove_key_from_json(&mut json, "key1");
        assert!(removed);
        assert_eq!(json, json!({"key2": "value2"}));

        // Test removing a nested key
        let mut json = json!({
            "parent": {
                "child1": "value1",
                "child2": "value2"
            }
        });
        let removed = remove_key_from_json(&mut json, "parent.child1");
        assert!(removed);
        assert_eq!(json, json!({"parent": {"child2": "value2"}}));

        // Test removing a deeply nested key
        let mut json = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "target": "value",
                        "keep": "keep"
                    }
                }
            }
        });
        let removed = remove_key_from_json(&mut json, "level1.level2.level3.target");
        assert!(removed);
        assert_eq!(
            json,
            json!({"level1": {"level2": {"level3": {"keep": "keep"}}}})
        );

        // Test removing a non-existent key
        let mut json = json!({"key": "value"});
        let removed = remove_key_from_json(&mut json, "nonexistent");
        assert!(!removed);
        assert_eq!(json, json!({"key": "value"}));

        // Test removing a key from a non-object
        let mut json = json!("not an object");
        let removed = remove_key_from_json(&mut json, "key");
        assert!(!removed);
        assert_eq!(json, json!("not an object"));

        // Test removing with an empty path
        let mut json = json!({"key": "value"});
        let removed = remove_key_from_json(&mut json, "");
        assert!(!removed);
        assert_eq!(json, json!({"key": "value"}));

        Ok(())
    }

    #[test]
    fn test_get_value_by_path() -> Result<()> {
        // Test getting a top-level value
        let json = json!({
            "key": "value",
            "nested": {
                "inner": "inner_value"
            }
        });

        let value = get_value_by_path(&json, "key");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), &json!("value"));

        // Test getting a nested value
        let value = get_value_by_path(&json, "nested.inner");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), &json!("inner_value"));

        // Test getting a non-existent value
        let value = get_value_by_path(&json, "nonexistent");
        assert!(value.is_none());

        // Test getting a value with an invalid path
        let value = get_value_by_path(&json, "key.invalid");
        assert!(value.is_none());

        // Test getting a value from a non-object
        let json = json!("not an object");
        let value = get_value_by_path(&json, "key");
        assert!(value.is_none());

        Ok(())
    }

    #[test]
    fn test_set_value_by_path() -> Result<()> {
        // Test setting a top-level value
        let mut json = json!({});
        let result = set_value_by_path(&mut json, "key", json!("value"));
        assert!(result);
        assert_eq!(json, json!({"key": "value"}));

        // Test setting a nested value
        let mut json = json!({"parent": {}});
        let result = set_value_by_path(&mut json, "parent.child", json!("value"));
        assert!(result);
        assert_eq!(json, json!({"parent": {"child": "value"}}));

        // Test setting a deeply nested value with auto-creation of intermediate objects
        let mut json = json!({});
        let result = set_value_by_path(&mut json, "level1.level2.level3", json!("value"));
        assert!(result);
        assert_eq!(json, json!({"level1": {"level2": {"level3": "value"}}}));

        // Test setting a value in a non-object
        let mut json = json!("not an object");
        let result = set_value_by_path(&mut json, "key", json!("value"));
        assert!(!result);
        assert_eq!(json, json!("not an object"));

        // Test overwriting an existing value
        let mut json = json!({"key": "old_value"});
        let result = set_value_by_path(&mut json, "key", json!("new_value"));
        assert!(result);
        assert_eq!(json, json!({"key": "new_value"}));

        // Test setting a value where an intermediate path is not an object
        let mut json = json!({"key": "value"});
        let result = set_value_by_path(&mut json, "key.child", json!("value"));
        assert!(!result);
        assert_eq!(json, json!({"key": "value"}));

        Ok(())
    }
}
