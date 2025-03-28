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

    let mut html = String::new();

    // HTML header
    html.push_str(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Translation Audit Report</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            line-height: 1.6;
            margin: 0;
            padding: 20px;
            color: #333;
        }
        h1, h2, h3 {
            color: #2c3e50;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
        }
        .summary {
            background-color: #f8f9fa;
            padding: 15px;
            border-radius: 5px;
            margin-bottom: 20px;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            margin-bottom: 20px;
        }
        th, td {
            padding: 12px 15px;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }
        th {
            background-color: #f2f2f2;
        }
        tr:hover {
            background-color: #f5f5f5;
        }
        .progress-bar {
            height: 20px;
            background-color: #e9ecef;
            border-radius: 5px;
            overflow: hidden;
        }
        .progress {
            height: 100%;
            background-color: #4caf50;
            text-align: center;
            color: white;
            line-height: 20px;
        }
        .warning {
            color: #856404;
            background-color: #fff3cd;
            padding: 10px;
            border-radius: 5px;
            margin-bottom: 10px;
        }
        .keys-list {
            max-height: 200px;
            overflow-y: auto;
            border: 1px solid #ddd;
            padding: 10px;
            border-radius: 5px;
            background-color: #f8f9fa;
        }
        .keys-list ul {
            padding-left: 20px;
        }
        .tab {
            overflow: hidden;
            border: 1px solid #ccc;
            background-color: #f1f1f1;
            border-radius: 5px 5px 0 0;
        }
        .tab button {
            background-color: inherit;
            float: left;
            border: none;
            outline: none;
            cursor: pointer;
            padding: 14px 16px;
            transition: 0.3s;
        }
        .tab button:hover {
            background-color: #ddd;
        }
        .tab button.active {
            background-color: #ccc;
        }
        .tabcontent {
            display: none;
            padding: 6px 12px;
            border: 1px solid #ccc;
            border-top: none;
            border-radius: 0 0 5px 5px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Translation Audit Report</h1>
"#,
    );

    // Summary section
    html.push_str(
        r#"<div class="summary">
            <h2>Summary</h2>
            <p>Total unique keys in codebase: "#,
    );
    html.push_str(&audit_result.keys_in_use.len().to_string());
    html.push_str("</p>\n");

    // Files table
    html.push_str(
        r#"<h2>Translation Files</h2>
        <table>
            <thead>
                <tr>
                    <th>File</th>
                    <th>Total Keys</th>
                    <th>Used Keys</th>
                    <th>Unused Keys</th>
                    <th>Missing Translations</th>
                    <th>Coverage</th>
                </tr>
            </thead>
            <tbody>
"#,
    );

    for (lang_code, data) in &audit_result.translations {
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

        html.push_str("<tr>\n");
        html.push_str(&format!(
            "<td>{}.json {}</td>\n",
            lang_code,
            if is_reference { "(reference)" } else { "" }
        ));
        html.push_str(&format!("<td>{}</td>\n", total_keys));
        html.push_str(&format!("<td>{}</td>\n", used_keys));
        html.push_str(&format!("<td>{}</td>\n", unused_keys));
        html.push_str(&format!("<td>{}</td>\n", missing.len()));

        // Coverage with progress bar
        html.push_str("<td>\n");
        html.push_str("<div class=\"progress-bar\">\n");
        html.push_str(&format!(
            "<div class=\"progress\" style=\"width: {}%;\">{:.1}%</div>\n",
            coverage, coverage
        ));
        html.push_str("</div>\n");
        html.push_str("</td>\n");

        html.push_str("</tr>\n");
    }

    html.push_str("</tbody></table>\n");

    // Tabs for detailed information
    html.push_str(r#"<div class="tab">
            <button class="tablinks" onclick="openTab(event, 'UnusedKeys')" id="defaultOpen">Unused Keys</button>
            <button class="tablinks" onclick="openTab(event, 'MissingTranslations')">Missing Translations</button>
        </div>
"#);

    // Unused keys tab
    html.push_str(
        r#"<div id="UnusedKeys" class="tabcontent">
            <h3>Unused Keys by File</h3>
"#,
    );

    for (lang_code, data) in &audit_result.translations {
        html.push_str(&format!(
            "<h4>{}.json ({} unused keys)</h4>\n",
            lang_code,
            data.unused_keys.len()
        ));

        if !data.unused_keys.is_empty() {
            html.push_str("<div class=\"keys-list\">\n<ul>\n");

            let mut sorted_keys: Vec<_> = data.unused_keys.iter().collect();
            sorted_keys.sort();

            for key in sorted_keys {
                html.push_str(&format!("<li>{}</li>\n", key));
            }

            html.push_str("</ul>\n</div>\n");
        } else {
            html.push_str("<p>No unused keys found.</p>\n");
        }
    }

    html.push_str("</div>\n");

    // Missing translations tab
    html.push_str(
        r#"<div id="MissingTranslations" class="tabcontent">
            <h3>Missing Translations by File</h3>
"#,
    );

    for lang_code in audit_result.translations.keys() {
        if lang_code == &audit_result.reference_language {
            continue;
        }

        let missing = get_missing_translations(audit_result, lang_code);
        html.push_str(&format!(
            "<h4>{}.json ({} missing translations)</h4>\n",
            lang_code,
            missing.len()
        ));

        if !missing.is_empty() {
            html.push_str("<div class=\"keys-list\">\n<ul>\n");

            let mut sorted_keys: Vec<_> = missing.iter().collect();
            sorted_keys.sort();

            for key in sorted_keys {
                html.push_str(&format!("<li>{}</li>\n", key));
            }

            html.push_str("</ul>\n</div>\n");
        } else {
            html.push_str("<p>No missing translations found.</p>\n");
        }
    }

    html.push_str("</div>\n");

    // JavaScript for tabs
    html.push_str(
        r#"<script>
        function openTab(evt, tabName) {
            var i, tabcontent, tablinks;
            tabcontent = document.getElementsByClassName("tabcontent");
            for (i = 0; i < tabcontent.length; i++) {
                tabcontent[i].style.display = "none";
            }
            tablinks = document.getElementsByClassName("tablinks");
            for (i = 0; i < tablinks.length; i++) {
                tablinks[i].className = tablinks[i].className.replace(" active", "");
            }
            document.getElementById(tabName).style.display = "block";
            evt.currentTarget.className += " active";
        }

        // Get the element with id="defaultOpen" and click on it
        document.getElementById("defaultOpen").click();
    </script>
    </div>
</body>
</html>"#,
    );

    fs::write(&report_path, html)?;
    println!("HTML report generated: {:?}", report_path);

    Ok(())
}
