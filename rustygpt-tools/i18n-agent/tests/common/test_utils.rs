use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::collections::HashSet;

/// Creates a temporary directory with sample source files for testing
pub fn create_test_source_directory() -> anyhow::Result<TempDir> {
    let temp_dir = TempDir::new()?;

    // Create a sample routes.rs file
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

    // Create a file with static keys
    let static_keys_file = temp_dir.child("static_keys.rs");
    static_keys_file.write_str(
        r#"
        fn render_buttons() {
            i18n.t("common.button.submit");
            i18n.t("common.button.cancel");
            i18n.translate("common.button.reset");
        }
        "#,
    )?;

    // Create a file with dynamic keys
    let dynamic_keys_file = temp_dir.child("dynamic_keys.rs");
    dynamic_keys_file.write_str(
        r#"
        fn render_route_title(route: &str) {
            i18n.t(&format!("{}.title", route));
            i18n.t(&format!("{}.icon", route));
            i18n.t(&format!("custom.pattern.{}.value", id));
        }
        "#,
    )?;

    Ok(temp_dir)
}

/// Creates a temporary directory with sample translation files for testing
pub fn create_test_translation_directory() -> anyhow::Result<TempDir> {
    let temp_dir = TempDir::new()?;

    // Create English translation file (reference language)
    let en_file = temp_dir.child("en.json");
    en_file.write_str(
        r#"{
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
            "admin": {
                "routes": {
                    "title": "Dashboard",
                    "icon": "dashboard",
                    "users": {
                        "title": "Users",
                        "icon": "people"
                    },
                    "settings": {
                        "title": "Settings",
                        "icon": "settings"
                    }
                }
            },
            "unused": {
                "key": "Unused"
            }
        }"#,
    )?;

    // Create Spanish translation file (missing some keys)
    let es_file = temp_dir.child("es.json");
    es_file.write_str(
        r#"{
            "common": {
                "button": {
                    "submit": "Enviar",
                    "cancel": "Cancelar"
                }
            },
            "profile": {
                "title": "Perfil"
            },
            "admin": {
                "routes": {
                    "title": "Panel",
                    "icon": "dashboard",
                    "users": {
                        "title": "Usuarios",
                        "icon": "people"
                    }
                }
            },
            "unused": {
                "key": "No usado"
            }
        }"#,
    )?;

    // Create French translation file (with different missing keys)
    let fr_file = temp_dir.child("fr.json");
    fr_file.write_str(
        r#"{
            "common": {
                "button": {
                    "submit": "Soumettre",
                    "cancel": "Annuler"
                }
            },
            "profile": {
                "title": "Profil"
            },
            "admin": {
                "routes": {
                    "title": "Tableau de bord",
                    "icon": "dashboard"
                }
            }
        }"#,
    )?;

    // Create a malformed JSON file for testing error handling
    // Create an intentionally malformed JSON file for testing error handling
    let invalid_file = temp_dir.child("invalid.json");
    invalid_file.write_str(
        r#"{
            "malformed": {
                "json": "missing closing brace"
            }
        }"#,
    )?;

    Ok(temp_dir)
}

/// Creates a predefined set of keys in use for testing
pub fn create_test_keys_in_use() -> HashSet<String> {
    let mut keys_in_use = HashSet::new();

    // Static keys
    keys_in_use.insert("common.button.submit".to_string());
    keys_in_use.insert("common.button.cancel".to_string());
    keys_in_use.insert("common.button.reset".to_string());
    keys_in_use.insert("profile.title".to_string());

    // Admin route keys
    keys_in_use.insert("admin.routes.title".to_string());
    keys_in_use.insert("admin.routes.icon".to_string());
    keys_in_use.insert("admin.routes.users.title".to_string());
    keys_in_use.insert("admin.routes.users.icon".to_string());
    keys_in_use.insert("admin.routes.settings.title".to_string());
    keys_in_use.insert("admin.routes.settings.icon".to_string());

    keys_in_use
}
