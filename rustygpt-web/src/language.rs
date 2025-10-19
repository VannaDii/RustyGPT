use std::collections::HashMap;

/// Information about a supported language
#[derive(PartialEq, Eq, Clone)]
pub struct LanguageInfo {
    pub code: &'static str,
    pub flag: &'static str,
    pub translation: &'static str,
    pub native_name: &'static str,
}

/// Get information about a supported language
pub fn get_language_info(code: &str) -> Option<LanguageInfo> {
    supported_languages().get(code).cloned()
}

/// Get a map of supported languages
pub fn supported_languages() -> HashMap<&'static str, LanguageInfo> {
    HashMap::from([
        (
            "en",
            LanguageInfo {
                code: "en",
                flag: "🇬🇧",
                translation: include_str!("../translations/en.json"),
                native_name: "English",
            },
        ),
        (
            "es",
            LanguageInfo {
                code: "es",
                flag: "🇪🇸",
                translation: include_str!("../translations/es.json"),
                native_name: "Español",
            },
        ),
        (
            "de",
            LanguageInfo {
                code: "de",
                flag: "🇩🇪",
                translation: include_str!("../translations/de.json"),
                native_name: "Deutsch",
            },
        ),
        (
            "fr",
            LanguageInfo {
                code: "fr",
                flag: "🇫🇷",
                translation: include_str!("../translations/fr.json"),
                native_name: "Français",
            },
        ),
    ])
}
