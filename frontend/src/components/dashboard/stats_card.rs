use yew::{Html, Properties, classes, function_component, html};
use yew_icons::{Icon, IconId};

/// Type of change indicator in a stats card
#[derive(Clone, PartialEq)]
pub enum ChangeType {
    Positive,
    Negative,
    Neutral,
}

/// Props for the StatsCard component
#[derive(Properties, PartialEq)]
pub struct StatsCardProps {
    /// Title of the stats card
    pub title: String,

    /// Main value to display
    pub value: String,

    /// Optional change value (e.g., "+12%")
    #[prop_or_default]
    pub change: Option<String>,

    /// Type of change (affects color)
    #[prop_or(ChangeType::Neutral)]
    pub change_type: ChangeType,

    /// Icon to display
    #[prop_or_default]
    pub icon: Option<IconId>,

    /// Icon background color class
    #[prop_or("bg-primary")]
    pub icon_bg_class: &'static str,
}

/// A card displaying statistics with optional icon and trend indicator
#[function_component(StatsCard)]
pub fn stats_card(props: &StatsCardProps) -> Html {
    // Determine change indicator color based on the change type
    let change_class = match props.change_type {
        ChangeType::Positive => "text-success",
        ChangeType::Negative => "text-error",
        ChangeType::Neutral => "text-base-content/70",
    };

    // Determine change icon based on the change type
    let change_icon = match props.change_type {
        ChangeType::Positive => IconId::HeroiconsOutlineArrowTrendingUp,
        ChangeType::Negative => IconId::HeroiconsOutlineArrowTrendingDown,
        ChangeType::Neutral => IconId::HeroiconsOutlineMinus,
    };

    html! {
        <div class="card bg-base-100 shadow-sm">
            <div class="card-body p-4">
                <div class="flex items-center justify-between">
                    <div>
                        <h3 class="text-base-content/70 text-sm font-medium">{&props.title}</h3>
                        <p class="text-2xl font-bold mt-1">{&props.value}</p>

                        // Change indicator
                        if let Some(change) = &props.change {
                            <div class={classes!("flex", "items-center", "gap-1", "mt-1", change_class)}>
                                <Icon icon_id={change_icon} class="h-4 w-4" />
                                <span class="text-sm">{change}</span>
                            </div>
                        }
                    </div>

                    // Icon
                    if let Some(icon) = props.icon {
                        <div class={classes!("p-3", "rounded-lg", props.icon_bg_class, "text-base-100")}>
                            <Icon icon_id={icon} class="h-6 w-6" />
                        </div>
                    }
                </div>
            </div>
        </div>
    }
}
