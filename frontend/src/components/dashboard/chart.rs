use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::{Html, Properties, classes, function_component, html};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Chart types
#[derive(Clone, PartialEq)]
pub enum ChartType {
    /// Line chart
    Line,
    /// Bar chart
    Bar,
    /// Pie chart
    Pie,
    /// Doughnut chart
    Doughnut,
    /// Area chart
    Area,
}

/// Chart component properties
#[derive(Properties, PartialEq)]
pub struct ChartProps {
    /// Chart title
    pub title: String,

    /// Chart type
    #[prop_or(ChartType::Line)]
    pub chart_type: ChartType,

    /// Chart height in pixels
    #[prop_or(300)]
    pub height: u32,

    /// Chart placeholder (until we have real chart implementation)
    #[prop_or(true)]
    pub is_placeholder: bool,
}

/// Chart component (currently a placeholder to match DaisyUI template)
#[function_component(Chart)]
pub fn chart(props: &ChartProps) -> Html {
    let (_i18n, ..) = use_translation();

    // Determine chart icon based on type
    let chart_icon = match props.chart_type {
        ChartType::Line => "📈",
        ChartType::Bar => "📊",
        ChartType::Pie => "🥧",
        ChartType::Doughnut => "🍩",
        ChartType::Area => "📉",
    };

    // If we're using a placeholder, render a simple placeholder
    // In a real implementation, this would use a charting library
    html! {
        <div class="card bg-base-100 shadow-sm">
            <div class="card-body">
                <h2 class="card-title">{&props.title}</h2>

                if props.is_placeholder {
                    <div
                        class={classes!(
                            "flex",
                            "flex-col",
                            "items-center",
                            "justify-center",
                            "border",
                            "border-dashed",
                            "border-base-300",
                            "rounded-lg",
                            "bg-base-200/30",
                            "p-6",
                        )}
                        style={format!("height: {}px;", props.height)}
                    >
                        <div class="text-4xl mb-2">{chart_icon}</div>
                        <p class="text-base-content/70 text-center">
                            {_i18n.t("dashboard.chart_placeholder")}
                            <br />
                            {_i18n.t("dashboard.chart_description")}
                        </p>
                    </div>
                } else {
                    // This would be replaced with actual chart rendering
                    <div class="bg-base-200 rounded-lg" style={format!("height: {}px;", props.height)}></div>
                }
            </div>
        </div>
    }
}
