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
fn scan_file(
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
fn handle_dynamic_route_keys(keys: &mut HashSet<String>, src_dir: &Path) -> Result<()> {
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
fn find_file(dir: &Path, filename: &str) -> Result<Option<PathBuf>> {
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
