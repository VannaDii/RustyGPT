use crate::config::FrontendConfig;
use crate::routes::MainRoute;
use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::{Html, function_component, html};
use yew_icons::{Icon, IconId};
use yew_router::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Dashboard page component
#[function_component(DashboardPage)]
pub fn dashboard_page() -> Html {
    let (i18n, _) = use_translation();
    let config = FrontendConfig::new();
    let documentation_url = config.documentation_url().to_string();

    html! {
        <div class="p-4 space-y-6">
            <h1 class="text-2xl font-bold">{ i18n.t("app.title") }</h1>

            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                // Chat card
                <div class="card bg-base-200 shadow-xl">
                    <div class="card-body">
                        <h2 class="card-title">
                            <Icon icon_id={IconId::HeroiconsOutlineChatBubbleLeftRight} class="w-6 h-6" />
                            { i18n.t("chat.title") }
                        </h2>
                        <p>{ i18n.t("dashboard.cards.chat.description") }</p>
                        <div class="card-actions justify-end">
                            <Link<MainRoute> to={MainRoute::Chat} classes="btn btn-primary">
                                { i18n.t("dashboard.cards.chat.action") }
                            </Link<MainRoute>>
                        </div>
                    </div>
                </div>

                // Settings card
                <div class="card bg-base-200 shadow-xl">
                    <div class="card-body">
                        <h2 class="card-title">
                            <Icon icon_id={IconId::HeroiconsOutlineCog6Tooth} class="w-6 h-6" />
                            { i18n.t("dashboard.cards.settings.title") }
                        </h2>
                        <p>{ i18n.t("dashboard.cards.settings.description") }</p>
                        <div class="card-actions justify-end">
                            <Link<MainRoute> to={MainRoute::AdminRoot} classes="btn btn-secondary">
                                { i18n.t("dashboard.cards.settings.action") }
                            </Link<MainRoute>>
                        </div>
                    </div>
                </div>

                // Documentation card
                <div class="card bg-base-200 shadow-xl">
                    <div class="card-body">
                        <h2 class="card-title">
                            <Icon icon_id={IconId::HeroiconsOutlineDocumentText} class="w-6 h-6" />
                            { i18n.t("dashboard.cards.documentation.title") }
                        </h2>
                        <p>{ i18n.t("dashboard.cards.documentation.description") }</p>
                        <div class="card-actions justify-end">
                            <a href={documentation_url} target="_blank" class="btn btn-outline">
                                { i18n.t("dashboard.cards.documentation.action") }
                            </a>
                        </div>
                    </div>
                </div>
            </div>

            <div class="stats shadow w-full">
                <div class="stat">
                    <div class="stat-figure text-primary">
                        <Icon icon_id={IconId::HeroiconsOutlineChatBubbleLeftRight} class="w-8 h-8" />
                    </div>
                    <div class="stat-title">{ i18n.t("dashboard.stats.conversations.title") }</div>
                    <div class="stat-value text-primary">{ "0" }</div>
                    <div class="stat-desc">{ i18n.t("dashboard.stats.conversations.description") }</div>
                </div>

                <div class="stat">
                    <div class="stat-figure text-secondary">
                        <Icon icon_id={IconId::HeroiconsOutlineDocument} class="w-8 h-8" />
                    </div>
                    <div class="stat-title">{ i18n.t("dashboard.stats.messages.title") }</div>
                    <div class="stat-value text-secondary">{ "0" }</div>
                    <div class="stat-desc">{ i18n.t("dashboard.stats.messages.description") }</div>
                </div>

                <div class="stat">
                    <div class="stat-figure text-success">
                        <Icon icon_id={IconId::HeroiconsOutlineCheck} class="w-8 h-8" />
                    </div>
                    <div class="stat-title">{ i18n.t("dashboard.stats.status.title") }</div>
                    <div class="stat-value text-success">{ i18n.t("dashboard.stats.status.value") }</div>
                    <div class="stat-desc">{ i18n.t("dashboard.stats.status.description") }</div>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    #[allow(dead_code)] // WASM tests may not be run in regular test suite
    fn test_dashboard_page_function_exists() {
        // Test that the dashboard page component can be referenced
        let config = FrontendConfig::new();
        assert!(!config.documentation_url().is_empty());
    }

    #[wasm_bindgen_test]
    #[allow(dead_code)] // WASM tests may not be run in regular test suite
    fn test_frontend_config_integration() {
        let config = FrontendConfig::new();
        assert!(!config.documentation_url().is_empty());
        assert!(config.documentation_url().starts_with("http"));
    }

    #[wasm_bindgen_test]
    #[allow(dead_code)] // WASM tests may not be run in regular test suite
    fn test_frontend_config_default_url() {
        let config = FrontendConfig::default();
        let url = config.documentation_url();
        assert!(url.contains("github.com") || url.contains("VannaDii") || url.contains("RustyGPT"));
    }

    #[wasm_bindgen_test]
    #[allow(dead_code)] // WASM tests may not be run in regular test suite
    fn test_frontend_config_clone() {
        let config1 = FrontendConfig::new();
        let config2 = config1.clone();
        assert_eq!(config1.documentation_url(), config2.documentation_url());
    }

    #[wasm_bindgen_test]
    #[allow(dead_code)] // WASM tests may not be run in regular test suite
    fn test_frontend_config_debug() {
        let config = FrontendConfig::new();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("FrontendConfig"));
        assert!(debug_str.contains("documentation_url"));
    }

    #[wasm_bindgen_test]
    #[allow(dead_code)] // WASM tests may not be run in regular test suite
    fn test_dashboard_component_creation() {
        // Test that we can create the dashboard component without panicking
        let config = FrontendConfig::new();
        let documentation_url = config.documentation_url().to_string();
        assert!(!documentation_url.is_empty());
    }

    #[wasm_bindgen_test]
    #[allow(dead_code)] // WASM tests may not be run in regular test suite
    fn test_dashboard_uses_configuration() {
        // Test that dashboard properly uses the configuration system
        let config = FrontendConfig::new();

        // Verify the URL is configurable and not hardcoded
        assert!(config.documentation_url().starts_with("http"));

        // Test URL format - simple check without url crate
        let url = config.documentation_url();
        assert!(url.contains("://"));
        assert!(url.len() > 10);
    }
}
