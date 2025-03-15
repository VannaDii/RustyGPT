use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::{Html, function_component, html};
use yew_icons::{Icon, IconId};

use crate::components::{
    ChangeType, Chart, ChartProps, ChartType, Column, DataTable, Row, StatsCard,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Dashboard page component
#[function_component(DashboardPage)]
pub fn dashboard_page() -> Html {
    let (i18n, _) = use_translation();

    // Create sample table data
    let columns = vec![
        Column {
            id: "type".to_string(),
            label: i18n.t("dashboard.type").to_string(),
            render: None,
            sortable: true,
        },
        Column {
            id: "description".to_string(),
            label: i18n.t("dashboard.description").to_string(),
            render: None,
            sortable: true,
        },
        Column {
            id: "date".to_string(),
            label: i18n.t("dashboard.date").to_string(),
            render: None,
            sortable: true,
        },
    ];

    let rows = vec![
        Row {
            id: "1".to_string(),
            data: {
                let mut map = std::collections::HashMap::new();
                map.insert("type".to_string(), "User Registration".to_string());
                map.insert("description".to_string(), "New user signed up".to_string());
                map.insert("date".to_string(), "2025-03-12".to_string());
                map
            },
        },
        Row {
            id: "2".to_string(),
            data: {
                let mut map = std::collections::HashMap::new();
                map.insert("type".to_string(), "Payment".to_string());
                map.insert(
                    "description".to_string(),
                    "Payment of $199.99 processed".to_string(),
                );
                map.insert("date".to_string(), "2025-03-11".to_string());
                map
            },
        },
        Row {
            id: "3".to_string(),
            data: {
                let mut map = std::collections::HashMap::new();
                map.insert("type".to_string(), "API Access".to_string());
                map.insert("description".to_string(), "API key created".to_string());
                map.insert("date".to_string(), "2025-03-10".to_string());
                map
            },
        },
    ];

    html! {
        <div class="p-4 space-y-6">
            <h1 class="text-2xl font-bold mb-6">{i18n.t("dashboard.page_title")}</h1>

            // Stats cards
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                <StatsCard
                    title={i18n.t("dashboard.total_users")}
                    value="3,721"
                    change={Some(format!("15% {}", i18n.t("dashboard.increase")))}
                    change_type={ChangeType::Positive}
                    icon={Some(IconId::HeroiconsOutlineUsers)}
                    icon_bg_class="bg-primary"
                />

                <StatsCard
                    title={i18n.t("dashboard.revenue")}
                    value="$45,231"
                    change={Some(format!("12% {}", i18n.t("dashboard.increase")))}
                    change_type={ChangeType::Positive}
                    icon={Some(IconId::HeroiconsOutlineCurrencyDollar)}
                    icon_bg_class="bg-success"
                />

                <StatsCard
                    title={i18n.t("dashboard.active_users")}
                    value="2,315"
                    change={Some(format!("321 {}", i18n.t("dashboard.users_this_month")))}
                    change_type={ChangeType::Neutral}
                    icon={Some(IconId::HeroiconsOutlineUserCircle)}
                    icon_bg_class="bg-info"
                />

                <StatsCard
                    title={i18n.t("dashboard.conversion_rate")}
                    value="3.24%"
                    change={Some(format!("4% {}", i18n.t("dashboard.decrease")))}
                    change_type={ChangeType::Negative}
                    icon={Some(IconId::HeroiconsOutlineChartBar)}
                    icon_bg_class="bg-warning"
                />
            </div>

            // Charts
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 mt-6">
                <Chart
                    title={i18n.t("dashboard.performance_overview")}
                    chart_type={ChartType::Line}
                    height={250}
                />

                <Chart
                    title={i18n.t("dashboard.revenue")}
                    chart_type={ChartType::Bar}
                    height={250}
                />
            </div>

            // Table
            <div class="mt-6">
                <div class="card bg-base-100 shadow-sm">
                    <div class="card-body">
                        <h2 class="card-title">{i18n.t("dashboard.recent_activity")}</h2>
                        <DataTable
                            columns={columns}
                            rows={rows}
                            hover={true}
                            rows_per_page={5}
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}
