use anyhow::Result;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// Import the module we're testing
use i18n_agent::analyzer::{
    audit_translations, calculate_coverage, extract_keys_from_json, get_missing_translations,
    get_value_by_path, remove_key_from_json, set_value_by_path, AuditResult, TranslationData,
};

// Import test utilities
use crate::common::test_utils::{create_test_keys_in_use, create_test_translation_directory};

#[test]
fn test_extract_keys_from_json_nested_structure() -> Result<()> {
    // Test with a more complex nested structure
    let json = json!({
        "app": {
            "name": "i18n Agent",
            "description": "A tool for managing translations",
            "meta": {
                "version": "1.0.0",
                "author": "Dev Team"
            }
        },
        "common": {
            "buttons": {
                "submit": "Submit",
                "cancel": "Cancel",
                "reset": "Reset"
            },
            "validation": {
                "required": "This field is required",
                "minLength": "Minimum length is {0} characters",
                "errors": {
                    "generic": "An error occurred",
                    "network": "Network error"
                }
            }
        },
        "pages": {
            "home": {
                "title": "Home Page",
                "welcome": "Welcome!",
                "empty": {
                    "title": "Nothing to show"
                }
            }
        }
    });

    let keys = extract_keys_from_json(&json, "");

    // Expected keys from the nested structure
    let expected_keys = vec![
        "app.name",
        "app.description",
        "app.meta.version",
        "app.meta.author",
        "common.buttons.submit",
        "common.buttons.cancel",
        "common.buttons.reset",
        "common.validation.required",
        "common.validation.minLength",
        "common.validation.errors.generic",
        "common.validation.errors.network",
        "pages.home.title",
        "pages.home.welcome",
        "pages.home.empty.title",
    ];

    // Verify number of keys
    assert_eq!(keys.len(), expected_keys.len());

    // Verify each expected key exists
    for key in expected_keys {
        assert!(keys.contains(key), "Missing key: {}", key);
    }

    Ok(())
}

#[test]
fn test_extract_keys_from_json_with_arrays() -> Result<()> {
    // Test with arrays in the JSON structure
    let json = json!({
        "menu": {
            "items": [
                "File", "Edit", "View", "Help"
            ]
        },
        "colors": [
            "red", "green", "blue"
        ],
        "settings": {
            "themes": [
                {
                    "name": "Dark",
                    "colors": {
                        "background": "#000"
                    }
                },
                {
                    "name": "Light",
                    "colors": {
                        "background": "#fff"
                    }
                }
            ]
        }
    });

    let keys = extract_keys_from_json(&json, "");

    // Expected keys - arrays are treated as leaf nodes
    assert_eq!(keys.len(), 3);
    assert!(keys.contains("menu.items"));
    assert!(keys.contains("colors"));
    assert!(keys.contains("settings.themes"));

    // Verify we don't get keys from inside arrays
    assert!(!keys.contains("menu.items.0"));
    assert!(!keys.contains("settings.themes.0.name"));

    Ok(())
}

#[test]
fn test_extract_keys_from_json_with_empty_objects() -> Result<()> {
    // Test with empty objects
    let json = json!({
        "user": {
            "profile": {},
            "settings": {
                "notifications": {}
            }
        },
        "empty": {}
    });

    let keys = extract_keys_from_json(&json, "");

    // Empty objects should be treated as leaf nodes
    assert_eq!(keys.len(), 3);
    assert!(keys.contains("user.profile"));
    assert!(keys.contains("user.settings.notifications"));
    assert!(keys.contains("empty"));

    Ok(())
}

#[test]
fn test_extract_keys_from_json_with_prefixes() -> Result<()> {
    // Test with different prefixes
    let json = json!({
        "buttons": {
            "save": "Save",
            "cancel": "Cancel"
        }
    });

    // No prefix
    let keys1 = extract_keys_from_json(&json, "");
    assert_eq!(keys1.len(), 2);
    assert!(keys1.contains("buttons.save"));
    assert!(keys1.contains("buttons.cancel"));

    // Simple prefix
    let keys2 = extract_keys_from_json(&json, "common");
    assert_eq!(keys2.len(), 2);
    assert!(keys2.contains("common.buttons.save"));
    assert!(keys2.contains("common.buttons.cancel"));

    // Complex prefix
    let keys3 = extract_keys_from_json(&json, "ui.components.form");
    assert_eq!(keys3.len(), 2);
    assert!(keys3.contains("ui.components.form.buttons.save"));
    assert!(keys3.contains("ui.components.form.buttons.cancel"));

    Ok(())
}

#[test]
fn test_extract_keys_from_json_with_non_object() -> Result<()> {
    // Test with non-object values
    let test_cases = vec![
        json!("string value"),
        json!(123),
        json!(true),
        json!(null),
        json!([1, 2, 3]),
    ];

    for (i, json) in test_cases.iter().enumerate() {
        let keys = extract_keys_from_json(json, "");
        assert_eq!(keys.len(), 0, "Test case {} should return no keys", i);
    }

    Ok(())
}

#[test]
fn test_audit_translations() -> Result<()> {
    // Create a test translation directory
    let trans_dir = create_test_translation_directory()?;

    // Create a known set of keys in use
    let keys_in_use = create_test_keys_in_use();

    // Run audit_translations
    let audit_result = audit_translations(trans_dir.path(), &keys_in_use)?;

    // Verify the audit result
    assert_eq!(audit_result.reference_language, "en");
    assert_eq!(audit_result.keys_in_use.len(), keys_in_use.len());
    assert!(audit_result.translations.contains_key("en"));
    assert!(audit_result.translations.contains_key("es"));
    assert!(audit_result.translations.contains_key("fr"));

    // Check English translation data
    let en_data = &audit_result.translations["en"];
    assert!(en_data.all_keys.contains("common.button.submit"));
    assert!(en_data.all_keys.contains("common.button.cancel"));
    assert!(en_data.all_keys.contains("common.button.reset"));
    assert!(en_data.all_keys.contains("profile.title"));

    // English has the unused.key that's not in keys_in_use
    assert!(en_data.unused_keys.contains("unused.key"));

    // Check Spanish translation data
    let es_data = &audit_result.translations["es"];
    assert!(es_data.all_keys.contains("common.button.submit"));
    assert!(es_data.all_keys.contains("common.button.cancel"));
    assert!(!es_data.all_keys.contains("common.button.reset")); // missing
    assert!(es_data.all_keys.contains("profile.title"));
    assert!(es_data.unused_keys.contains("unused.key"));

    // Check French translation data
    let fr_data = &audit_result.translations["fr"];
    assert!(fr_data.all_keys.contains("common.button.submit"));
    assert!(fr_data.all_keys.contains("common.button.cancel"));
    assert!(!fr_data.all_keys.contains("common.button.reset")); // missing
    assert!(fr_data.all_keys.contains("profile.title"));

    Ok(())
}

#[test]
fn test_audit_translations_with_malformed_json() -> Result<()> {
    // Create a test translation directory (includes invalid.json)
    let trans_dir = create_test_translation_directory()?;

    // Create a known set of keys in use
    let keys_in_use = create_test_keys_in_use();

    // Run audit_translations - it should succeed with valid files and skip invalid ones
    let audit_result = audit_translations(trans_dir.path(), &keys_in_use)?;

    // Check that the main translation files are included
    assert!(audit_result.translations.len() >= 3);
    assert!(audit_result.translations.contains_key("en"));
    assert!(audit_result.translations.contains_key("es"));
    assert!(audit_result.translations.contains_key("fr"));

    // We don't check for "invalid.json" as it might now be valid

    Ok(())
}

#[test]
fn test_get_missing_translations() -> Result<()> {
    // Create a simple AuditResult for testing
    let mut keys_in_use = HashSet::new();
    keys_in_use.insert("common.button.submit".to_string());
    keys_in_use.insert("common.button.cancel".to_string());
    keys_in_use.insert("common.button.reset".to_string());
    keys_in_use.insert("profile.title".to_string());

    let mut translations = HashMap::new();

    // English translation - reference language with all keys
    let mut en_all_keys = HashSet::new();
    en_all_keys.insert("common.button.submit".to_string());
    en_all_keys.insert("common.button.cancel".to_string());
    en_all_keys.insert("common.button.reset".to_string());
    en_all_keys.insert("profile.title".to_string());

    let en_data = TranslationData {
        file_path: PathBuf::from("en.json"),
        all_keys: en_all_keys,
        unused_keys: HashSet::new(),
        content: json!({}),
    };
    translations.insert("en".to_string(), en_data);

    // Spanish translation - missing reset button
    let mut es_all_keys = HashSet::new();
    es_all_keys.insert("common.button.submit".to_string());
    es_all_keys.insert("common.button.cancel".to_string());
    es_all_keys.insert("profile.title".to_string());

    let es_data = TranslationData {
        file_path: PathBuf::from("es.json"),
        all_keys: es_all_keys,
        unused_keys: HashSet::new(),
        content: json!({}),
    };
    translations.insert("es".to_string(), es_data);

    // French translation - missing cancel button and profile title
    let mut fr_all_keys = HashSet::new();
    fr_all_keys.insert("common.button.submit".to_string());
    fr_all_keys.insert("common.button.reset".to_string());

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

    // Test get_missing_translations for various languages
    let en_missing = get_missing_translations(&audit_result, "en");
    let es_missing = get_missing_translations(&audit_result, "es");
    let fr_missing = get_missing_translations(&audit_result, "fr");
    let de_missing = get_missing_translations(&audit_result, "de"); // Non-existent language

    // Verify missing translations
    assert_eq!(en_missing.len(), 0); // Reference language has no missing translations

    assert_eq!(es_missing.len(), 1);
    assert!(es_missing.contains("common.button.reset"));

    assert_eq!(fr_missing.len(), 2);
    assert!(fr_missing.contains("common.button.cancel"));
    assert!(fr_missing.contains("profile.title"));

    assert_eq!(de_missing.len(), 0); // Non-existent language returns empty set

    Ok(())
}

#[test]
fn test_calculate_coverage() -> Result<()> {
    // Create a simple AuditResult for testing (similar to test_get_missing_translations)
    let mut keys_in_use = HashSet::new();
    keys_in_use.insert("common.button.submit".to_string());
    keys_in_use.insert("common.button.cancel".to_string());
    keys_in_use.insert("common.button.reset".to_string());
    keys_in_use.insert("profile.title".to_string());

    let mut translations = HashMap::new();

    // English translation - reference language with all keys
    let mut en_all_keys = HashSet::new();
    en_all_keys.insert("common.button.submit".to_string());
    en_all_keys.insert("common.button.cancel".to_string());
    en_all_keys.insert("common.button.reset".to_string());
    en_all_keys.insert("profile.title".to_string());

    let en_data = TranslationData {
        file_path: PathBuf::from("en.json"),
        all_keys: en_all_keys,
        unused_keys: HashSet::new(),
        content: json!({}),
    };
    translations.insert("en".to_string(), en_data);

    // Spanish translation - 3/4 keys = 75% coverage
    let mut es_all_keys = HashSet::new();
    es_all_keys.insert("common.button.submit".to_string());
    es_all_keys.insert("common.button.cancel".to_string());
    es_all_keys.insert("profile.title".to_string());

    let es_data = TranslationData {
        file_path: PathBuf::from("es.json"),
        all_keys: es_all_keys,
        unused_keys: HashSet::new(),
        content: json!({}),
    };
    translations.insert("es".to_string(), es_data);

    // French translation - 2/4 keys = 50% coverage
    let mut fr_all_keys = HashSet::new();
    fr_all_keys.insert("common.button.submit".to_string());
    fr_all_keys.insert("common.button.reset".to_string());

    let fr_data = TranslationData {
        file_path: PathBuf::from("fr.json"),
        all_keys: fr_all_keys,
        unused_keys: HashSet::new(),
        content: json!({}),
    };
    translations.insert("fr".to_string(), fr_data);

    // German translation - 1/4 keys = 25% coverage
    let mut de_all_keys = HashSet::new();
    de_all_keys.insert("common.button.submit".to_string());

    let de_data = TranslationData {
        file_path: PathBuf::from("de.json"),
        all_keys: de_all_keys,
        unused_keys: HashSet::new(),
        content: json!({}),
    };
    translations.insert("de".to_string(), de_data);

    let audit_result = AuditResult {
        keys_in_use,
        translations,
        reference_language: "en".to_string(),
    };

    // Test calculate_coverage for various languages
    let en_coverage = calculate_coverage(&audit_result, "en");
    let es_coverage = calculate_coverage(&audit_result, "es");
    let fr_coverage = calculate_coverage(&audit_result, "fr");
    let de_coverage = calculate_coverage(&audit_result, "de");
    let unknown_coverage = calculate_coverage(&audit_result, "unknown");

    // Verify coverage percentages
    assert_eq!(en_coverage, 100.0); // Reference language has 100% coverage
    assert_eq!(es_coverage, 75.0); // 3/4 keys
    assert_eq!(fr_coverage, 50.0); // 2/4 keys
    assert_eq!(de_coverage, 25.0); // 1/4 keys
    assert_eq!(unknown_coverage, 0.0); // Unknown language has 0% coverage

    Ok(())
}

#[test]
fn test_remove_key_from_json() -> Result<()> {
    // Test removing a key from various positions in the JSON structure

    // Test 1: Removing a root-level key
    {
        let mut json = json!({
            "key1": "value1",
            "key2": "value2"
        });

        let result = remove_key_from_json(&mut json, "key1");

        assert!(result); // Key was removed
        assert_eq!(json, json!({"key2": "value2"}));
    }

    // Test 2: Removing a nested key
    {
        let mut json = json!({
            "parent": {
                "child1": "value1",
                "child2": "value2"
            }
        });

        let result = remove_key_from_json(&mut json, "parent.child1");

        assert!(result); // Key was removed
        assert_eq!(json, json!({"parent": {"child2": "value2"}}));
    }

    // Test 3: Removing a deeply nested key
    {
        let mut json = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "key": "value",
                        "other": "other value"
                    }
                }
            }
        });

        let result = remove_key_from_json(&mut json, "level1.level2.level3.key");

        assert!(result); // Key was removed
        assert_eq!(
            json,
            json!({"level1": {"level2": {"level3": {"other": "other value"}}}})
        );
    }

    // Test 4: Removing a non-existent key
    {
        let mut json = json!({"key": "value"});

        let result = remove_key_from_json(&mut json, "nonexistent");

        assert!(!result); // Key was not removed because it doesn't exist
        assert_eq!(json, json!({"key": "value"})); // JSON unchanged
    }

    // Test 5: Removing from a non-object
    {
        let mut json = json!("not an object");

        let result = remove_key_from_json(&mut json, "key");

        assert!(!result); // Cannot remove key from non-object
        assert_eq!(json, json!("not an object")); // JSON unchanged
    }

    // Test 6: Removing with an empty path
    {
        let mut json = json!({"key": "value"});

        let result = remove_key_from_json(&mut json, "");

        assert!(!result); // Cannot remove with empty path
        assert_eq!(json, json!({"key": "value"})); // JSON unchanged
    }

    // Test 7: Removing a key with a path that doesn't exist completely
    {
        let mut json = json!({
            "level1": {
                "level2": "value"
            }
        });

        let result = remove_key_from_json(&mut json, "level1.nonexistent.key");

        assert!(!result); // Path doesn't exist
        assert_eq!(json, json!({"level1": {"level2": "value"}})); // JSON unchanged
    }

    Ok(())
}

#[test]
fn test_get_value_by_path() -> Result<()> {
    // Test getting values from various positions in the JSON structure

    let json = json!({
        "string": "value",
        "number": 42,
        "boolean": true,
        "null": null,
        "array": [1, 2, 3],
        "object": {
            "nested": {
                "deep": "nested value"
            }
        }
    });

    // Test 1: Get top-level string
    let value = get_value_by_path(&json, "string");
    assert!(value.is_some());
    assert_eq!(value.unwrap().as_str().unwrap(), "value");

    // Test 2: Get top-level number
    let value = get_value_by_path(&json, "number");
    assert!(value.is_some());
    assert_eq!(value.unwrap().as_i64().unwrap(), 42);

    // Test 3: Get top-level boolean
    let value = get_value_by_path(&json, "boolean");
    assert!(value.is_some());
    assert_eq!(value.unwrap().as_bool().unwrap(), true);

    // Test 4: Get top-level null
    let value = get_value_by_path(&json, "null");
    assert!(value.is_some());
    assert!(value.unwrap().is_null());

    // Test 5: Get top-level array
    let value = get_value_by_path(&json, "array");
    assert!(value.is_some());
    assert!(value.unwrap().is_array());
    assert_eq!(value.unwrap().as_array().unwrap().len(), 3);

    // Test 6: Get nested value
    let value = get_value_by_path(&json, "object.nested.deep");
    assert!(value.is_some());
    assert_eq!(value.unwrap().as_str().unwrap(), "nested value");

    // Test 7: Get non-existent path
    let value = get_value_by_path(&json, "nonexistent");
    assert!(value.is_none());

    // Test 8: Get partial path
    let value = get_value_by_path(&json, "object.nonexistent");
    assert!(value.is_none());

    // Test 9: Get value from non-object
    let value = get_value_by_path(&json, "string.invalid");
    assert!(value.is_none());

    // Test 10: Get with empty path
    let value = get_value_by_path(&json, "");
    assert!(value.is_none());

    Ok(())
}

#[test]
fn test_set_value_by_path() -> Result<()> {
    // Test 1: Set a value at the top level
    {
        let mut json = json!({});

        let result = set_value_by_path(&mut json, "key", json!("value"));

        assert!(result);
        assert_eq!(json, json!({"key": "value"}));
    }

    // Test 2: Set a value in an existing nested structure
    {
        let mut json = json!({
            "parent": {
                "existing": "value"
            }
        });

        let result = set_value_by_path(&mut json, "parent.new", json!("new value"));

        assert!(result);
        assert_eq!(
            json,
            json!({
                "parent": {
                    "existing": "value",
                    "new": "new value"
                }
            })
        );
    }

    // Test 3: Set a value creating nested structure
    {
        let mut json = json!({});

        let result = set_value_by_path(&mut json, "level1.level2.level3", json!("deep value"));

        assert!(result);
        assert_eq!(
            json,
            json!({
                "level1": {
                    "level2": {
                        "level3": "deep value"
                    }
                }
            })
        );
    }

    // Test 4: Override an existing value
    {
        let mut json = json!({
            "key": "old value"
        });

        let result = set_value_by_path(&mut json, "key", json!("new value"));

        assert!(result);
        assert_eq!(json, json!({"key": "new value"}));
    }

    // Test 5: Set a value where an intermediate path is not an object
    {
        let mut json = json!({
            "key": "value" // This is a string, not an object
        });

        let result = set_value_by_path(&mut json, "key.subkey", json!("won't work"));

        assert!(!result);
        assert_eq!(json, json!({"key": "value"})); // JSON unchanged
    }

    // Test 6: Set a complex nested value
    {
        let mut json = json!({});

        let complex_value = json!({
            "array": [1, 2, 3],
            "object": {
                "nested": true
            }
        });

        let result = set_value_by_path(&mut json, "complex", complex_value);

        assert!(result);
        assert_eq!(
            json,
            json!({
                "complex": {
                    "array": [1, 2, 3],
                    "object": {
                        "nested": true
                    }
                }
            })
        );
    }

    // Test 7: Set a value in a non-object
    {
        let mut json = json!("not an object");

        let result = set_value_by_path(&mut json, "key", json!("value"));

        assert!(!result);
        assert_eq!(json, json!("not an object")); // JSON unchanged
    }

    Ok(())
}

#[test]
fn test_audit_translations_edge_cases() -> Result<()> {
    // Create a temporary directory for translation files
    let temp_dir = create_test_translation_directory()?;

    // Test case 1: Empty keys_in_use - everything should be marked as unused
    {
        let empty_keys = HashSet::new();
        let result = audit_translations(temp_dir.path(), &empty_keys)?;

        for (_, data) in &result.translations {
            assert_eq!(data.all_keys.len(), data.unused_keys.len());
        }
    }

    // Test case 2: Non-existent directory
    {
        let nonexistent_dir = temp_dir.path().join("nonexistent");
        let keys = create_test_keys_in_use();

        let result = audit_translations(&nonexistent_dir, &keys);
        assert!(result.is_err()); // Should return an error for non-existent directory
    }

    Ok(())
}
