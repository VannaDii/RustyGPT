use anyhow::Result;
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Scans the codebase for translation key usage and returns a set of unique keys
pub fn scan_codebase(src_dir: &Path) -> Result<HashSet<String>> {
    let mut keys = HashSet::new();

    // Regex patterns for finding translation keys
    let static_key_pattern = Regex::new(r#"i18n\.t\("([^"]+)"\)"#)?;
    let translate_pattern = Regex::new(r#"i18n\.translate\("([^"]+)"\)"#)?;
    let dynamic_key_pattern = Regex::new(r#"i18n\.t\(&format!\("([^"]+)", .+\)\)"#)?;

    // Walk through all .rs files in the source directory
    for entry in WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let file_path = entry.path();
        scan_file(
            file_path,
            &mut keys,
            &static_key_pattern,
            &translate_pattern,
            &dynamic_key_pattern,
        )?;
    }

    // Special handling for dynamic keys in header_nav_item.rs
    handle_dynamic_route_keys(&mut keys, src_dir)?;

    Ok(keys)
}

/// Scans a single file for translation key usage
pub fn scan_file(
    file_path: &Path,
    keys: &mut HashSet<String>,
    static_key_pattern: &Regex,
    translate_pattern: &Regex,
    dynamic_key_pattern: &Regex,
) -> Result<()> {
    let content = std::fs::read_to_string(file_path)?;

    // Find static keys (i18n.t("key"))
    for cap in static_key_pattern.captures_iter(&content) {
        if let Some(key) = cap.get(1) {
            keys.insert(key.as_str().to_string());
        }
    }

    // Find translate keys (i18n.translate("key"))
    for cap in translate_pattern.captures_iter(&content) {
        if let Some(key) = cap.get(1) {
            keys.insert(key.as_str().to_string());
        }
    }

    // Find dynamic keys with format patterns
    for cap in dynamic_key_pattern.captures_iter(&content) {
        if let Some(key_pattern) = cap.get(1) {
            // Store the pattern for later processing
            // We'll handle these in handle_dynamic_route_keys
            if key_pattern.as_str().contains("{}.title") || key_pattern.as_str().contains("{}.icon")
            {
                // This is a route-based key pattern, will be handled separately
            } else {
                // For other dynamic keys, we can't determine the exact keys
                // so we'll just log a warning
                log::warn!(
                    "Found dynamic key pattern that can't be statically analyzed: {}",
                    key_pattern.as_str()
                );
            }
        }
    }

    Ok(())
}

/// Special handling for dynamic keys in header_nav_item.rs that are based on routes
pub fn handle_dynamic_route_keys(keys: &mut HashSet<String>, src_dir: &Path) -> Result<()> {
    // Find the routes.rs file
    let routes_file = find_file(src_dir, "routes.rs")?;
    if routes_file.is_none() {
        log::warn!("Could not find routes.rs file for dynamic key analysis");
        return Ok(());
    }

    let routes_content = std::fs::read_to_string(routes_file.unwrap())?;

    // Extract AdminRoute enum variants
    let admin_route_pattern = Regex::new(r"enum\s+AdminRoute\s*\{([\s\S]*?)\}")?;
    if let Some(cap) = admin_route_pattern.captures(&routes_content) {
        if let Some(enum_content) = cap.get(1) {
            let variant_pattern = Regex::new(r##"#\[at\("([^"]+)"\)\]\s*([A-Za-z0-9_]+)"##)?;

            for var_cap in variant_pattern.captures_iter(enum_content.as_str()) {
                if let (Some(path), Some(_variant)) = (var_cap.get(1), var_cap.get(2)) {
                    let route_path = path.as_str().replace("/admin", "").replace("/", ".");
                    if route_path.is_empty() {
                        // Root admin route
                        keys.insert("admin.routes.title".to_string());
                        keys.insert("admin.routes.icon".to_string());
                    } else {
                        // Nested admin route
                        let key_base = format!("admin.routes{}", route_path);
                        keys.insert(format!("{}.title", key_base));
                        keys.insert(format!("{}.icon", key_base));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Helper function to find a file by name in a directory (recursively)
pub fn find_file(dir: &Path, filename: &str) -> Result<Option<PathBuf>> {
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if entry.file_name().to_string_lossy() == filename {
            return Ok(Some(entry.path().to_path_buf()));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use assert_fs::TempDir;

    #[test]
    fn test_scan_file_with_static_keys() -> Result<()> {
        // Create a temporary directory and file
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.child("test_file.rs");
        file_path.write_str(
            r#"
            fn some_function() {
                i18n.t("common.button.submit");
                i18n.t("common.button.cancel");
                let x = 5; // Some other code
                i18n.t("common.error.required");
            }
        "#,
        )?;

        // Create regex patterns
        let static_key_pattern = Regex::new(r#"i18n\.t\("([^"]+)"\)"#)?;
        let translate_pattern = Regex::new(r#"i18n\.translate\("([^"]+)"\)"#)?;
        let dynamic_key_pattern = Regex::new(r#"i18n\.t\(&format!\("([^"]+)", .+\)\)"#)?;

        // Test scan_file
        let mut keys = HashSet::new();
        scan_file(
            file_path.path(),
            &mut keys,
            &static_key_pattern,
            &translate_pattern,
            &dynamic_key_pattern,
        )?;

        // Verify results
        assert_eq!(keys.len(), 3);
        assert!(keys.contains("common.button.submit"));
        assert!(keys.contains("common.button.cancel"));
        assert!(keys.contains("common.error.required"));

        Ok(())
    }

    #[test]
    fn test_scan_file_with_translate_keys() -> Result<()> {
        // Create a temporary directory and file
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.child("test_file.rs");
        file_path.write_str(
            r#"
            fn some_function() {
                i18n.translate("profile.title");
                i18n.translate("profile.description");
            }
        "#,
        )?;

        // Create regex patterns
        let static_key_pattern = Regex::new(r#"i18n\.t\("([^"]+)"\)"#)?;
        let translate_pattern = Regex::new(r#"i18n\.translate\("([^"]+)"\)"#)?;
        let dynamic_key_pattern = Regex::new(r#"i18n\.t\(&format!\("([^"]+)", .+\)\)"#)?;

        // Test scan_file
        let mut keys = HashSet::new();
        scan_file(
            file_path.path(),
            &mut keys,
            &static_key_pattern,
            &translate_pattern,
            &dynamic_key_pattern,
        )?;

        // Verify results
        assert_eq!(keys.len(), 2);
        assert!(keys.contains("profile.title"));
        assert!(keys.contains("profile.description"));

        Ok(())
    }

    #[test]
    fn test_scan_file_with_dynamic_keys() -> Result<()> {
        // Create a temporary directory and file
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.child("test_file.rs");
        file_path.write_str(
            r#"
            fn some_function() {
                i18n.t(&format!("user.greeting", name));
                i18n.t(&format!("{}.title", route_name));
                i18n.t(&format!("{}.icon", route_name));
                i18n.t(&format!("custom.pattern.{}.value", id));
            }
        "#,
        )?;

        // Create regex patterns
        let static_key_pattern = Regex::new(r#"i18n\.t\("([^"]+)"\)"#)?;
        let translate_pattern = Regex::new(r#"i18n\.translate\("([^"]+)"\)"#)?;
        let dynamic_key_pattern = Regex::new(r#"i18n\.t\(&format!\("([^"]+)", .+\)\)"#)?;

        // Test scan_file
        let mut keys = HashSet::new();
        scan_file(
            file_path.path(),
            &mut keys,
            &static_key_pattern,
            &translate_pattern,
            &dynamic_key_pattern,
        )?;

        // Verify results - dynamic keys are not added directly by scan_file
        assert_eq!(keys.len(), 0);

        Ok(())
    }

    #[test]
    fn test_scan_file_with_mixed_keys() -> Result<()> {
        // Create a temporary directory and file
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.child("test_file.rs");
        file_path.write_str(
            r#"
            fn some_function() {
                i18n.t("static.key");
                i18n.translate("translate.key");
                i18n.t(&format!("{}.title", route_name));
                let x = "not a translation key";
            }
        "#,
        )?;

        // Create regex patterns
        let static_key_pattern = Regex::new(r#"i18n\.t\("([^"]+)"\)"#)?;
        let translate_pattern = Regex::new(r#"i18n\.translate\("([^"]+)"\)"#)?;
        let dynamic_key_pattern = Regex::new(r#"i18n\.t\(&format!\("([^"]+)", .+\)\)"#)?;

        // Test scan_file
        let mut keys = HashSet::new();
        scan_file(
            file_path.path(),
            &mut keys,
            &static_key_pattern,
            &translate_pattern,
            &dynamic_key_pattern,
        )?;

        // Verify results
        assert_eq!(keys.len(), 2);
        assert!(keys.contains("static.key"));
        assert!(keys.contains("translate.key"));

        Ok(())
    }

    #[test]
    fn test_find_file() -> Result<()> {
        // Create a temporary directory structure
        let temp_dir = TempDir::new()?;

        // Create a nested directory structure
        let subdir = temp_dir.child("subdir");
        subdir.create_dir_all()?;

        // Create some files
        let file1 = temp_dir.child("file1.txt");
        file1.write_str("content1")?;

        let file2 = subdir.child("file2.txt");
        file2.write_str("content2")?;

        let target_file = subdir.child("routes.rs");
        target_file.write_str("enum AdminRoute {}")?;

        // Test finding a file that exists in a subdirectory
        let result = find_file(temp_dir.path(), "routes.rs")?;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), target_file.path());

        // Test finding a file that doesn't exist
        let result = find_file(temp_dir.path(), "nonexistent.rs")?;
        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_handle_dynamic_route_keys() -> Result<()> {
        // Create a temporary directory
        let temp_dir = TempDir::new()?;

        // Create a routes.rs file with AdminRoute enum
        let routes_file = temp_dir.child("routes.rs");
        routes_file.write_str(
            r#"
            enum AdminRoute {
                #[at("/admin")]
                Dashboard,

                #[at("/admin/users")]
                Users,

                #[at("/admin/settings")]
                Settings,
            }
        "#,
        )?;

        // Test handle_dynamic_route_keys
        let mut keys = HashSet::new();
        handle_dynamic_route_keys(&mut keys, temp_dir.path())?;

        // Verify results
        assert_eq!(keys.len(), 6);
        assert!(keys.contains("admin.routes.title"));
        assert!(keys.contains("admin.routes.icon"));
        assert!(keys.contains("admin.routes.users.title"));
        assert!(keys.contains("admin.routes.users.icon"));
        assert!(keys.contains("admin.routes.settings.title"));
        assert!(keys.contains("admin.routes.settings.icon"));

        Ok(())
    }

    #[test]
    fn test_handle_dynamic_route_keys_no_routes_file() -> Result<()> {
        // Create a temporary directory without a routes.rs file
        let temp_dir = TempDir::new()?;

        // Test handle_dynamic_route_keys
        let mut keys = HashSet::new();
        handle_dynamic_route_keys(&mut keys, temp_dir.path())?;

        // Verify results - no keys should be added
        assert_eq!(keys.len(), 0);

        Ok(())
    }

    #[test]
    fn test_handle_dynamic_route_keys_invalid_routes_file() -> Result<()> {
        // Create a temporary directory
        let temp_dir = TempDir::new()?;

        // Create a routes.rs file without AdminRoute enum
        let routes_file = temp_dir.child("routes.rs");
        routes_file.write_str(
            r#"
            // No AdminRoute enum here
            struct SomethingElse {}
        "#,
        )?;

        // Test handle_dynamic_route_keys
        let mut keys = HashSet::new();
        handle_dynamic_route_keys(&mut keys, temp_dir.path())?;

        // Verify results - no keys should be added
        assert_eq!(keys.len(), 0);

        Ok(())
    }

    #[test]
    fn test_scan_codebase() -> Result<()> {
        // Create a temporary directory structure
        let temp_dir = TempDir::new()?;

        // Create a file with static keys
        let file1 = temp_dir.child("file1.rs");
        file1.write_str(
            r#"
            fn some_function() {
                i18n.t("common.button.submit");
                i18n.t("common.button.cancel");
            }
        "#,
        )?;

        // Create a file with translate keys
        let file2 = temp_dir.child("file2.rs");
        file2.write_str(
            r#"
            fn another_function() {
                i18n.translate("profile.title");
            }
        "#,
        )?;

        // Create a routes.rs file for dynamic keys
        let routes_file = temp_dir.child("routes.rs");
        routes_file.write_str(
            r#"
            enum AdminRoute {
                #[at("/admin")]
                Dashboard,

                #[at("/admin/users")]
                Users,
            }
        "#,
        )?;

        // Test scan_codebase
        let keys = scan_codebase(temp_dir.path())?;

        // Verify results
        assert_eq!(keys.len(), 7);
        assert!(keys.contains("common.button.submit"));
        assert!(keys.contains("common.button.cancel"));
        assert!(keys.contains("profile.title"));
        assert!(keys.contains("admin.routes.title"));
        assert!(keys.contains("admin.routes.icon"));
        assert!(keys.contains("admin.routes.users.title"));
        assert!(keys.contains("admin.routes.users.icon"));

        Ok(())
    }

    #[test]
    fn test_scan_codebase_with_non_rs_files() -> Result<()> {
        // Create a temporary directory structure
        let temp_dir = TempDir::new()?;

        // Create a .rs file with keys
        let rs_file = temp_dir.child("file.rs");
        rs_file.write_str(
            r#"
            fn some_function() {
                i18n.t("key.in.rs.file");
            }
        "#,
        )?;

        // Create a .txt file with keys that should be ignored
        let txt_file = temp_dir.child("file.txt");
        txt_file.write_str(
            r#"
            i18n.t("key.in.txt.file");
        "#,
        )?;

        // Test scan_codebase
        let keys = scan_codebase(temp_dir.path())?;

        // Verify results - only keys from .rs files should be included
        assert_eq!(keys.len(), 1);
        assert!(keys.contains("key.in.rs.file"));
        assert!(!keys.contains("key.in.txt.file"));

        Ok(())
    }
}
