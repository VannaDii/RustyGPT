use i18nrs::yew::use_translation;
use yew::{Html, Properties, classes, function_component, html};
use yew_icons::{Icon, IconId};

/// Change type for stats card
#[derive(Clone, Copy, PartialEq)]
pub enum ChangeType {
    /// Positive change (green)
    Positive,
    /// Negative change (red)
    Negative,
    /// Neutral change (gray/blue)
    Neutral,
}

/// Stats card component properties
#[derive(Properties, PartialEq)]
pub struct StatsCardProps {
    /// Card title
    pub title: String,

    /// Value to display
    pub value: String,

    /// Optional change text (e.g., "15% increase")
    #[prop_or_default]
    pub change: Option<String>,

    /// Change type to determine styling
    #[prop_or(ChangeType::Neutral)]
    pub change_type: ChangeType,

    /// Optional icon
    #[prop_or_default]
    pub icon: Option<IconId>,

    /// Optional background color class for icon
    #[prop_or("bg-primary")]
    pub icon_bg_class: &'static str,
}

/// Stats card component
#[function_component(StatsCard)]
pub fn stats_card(props: &StatsCardProps) -> Html {
    let (_i18n, ..) = use_translation();

    // Determine color class based on change type
    let change_color_class = match props.change_type {
        ChangeType::Positive => "text-success",
        ChangeType::Negative => "text-error",
        ChangeType::Neutral => "text-info",
    };

    // Determine change icon based on change type
    let change_icon = match props.change_type {
        ChangeType::Positive => IconId::HeroiconsOutlineChartBar, // Use available icons
        ChangeType::Negative => IconId::HeroiconsOutlineChartBar,
        ChangeType::Neutral => IconId::HeroiconsOutlineChartBar,
    };

    html! {
        <div class="card bg-base-100 shadow-sm">
            <div class="card-body p-4">
                <div class="flex items-center justify-between">
                    <div>
                        <h2 class="text-lg font-semibold text-base-content">{&props.value}</h2>
                        <p class="text-sm text-base-content/70">{&props.title}</p>

                        if let Some(change) = &props.change {
                            <div class={classes!("flex", "items-center", "mt-1", "text-xs", "font-medium", change_color_class)}>
                                <Icon icon_id={change_icon} class="h-3 w-3 mr-1" />
                                <span>{change}</span>
                            </div>
                        }
                    </div>

                    if let Some(icon_id) = props.icon {
                        <div class={classes!("rounded-lg", "p-2", props.icon_bg_class, "bg-opacity-10")}>
                            <Icon icon_id={icon_id} class="h-6 w-6 text-base-content" />
                        </div>
                    }
                </div>
            </div>
        </div>
    }
}
