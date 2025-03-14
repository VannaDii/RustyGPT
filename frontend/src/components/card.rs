use wasm_bindgen::prelude::*;
use yew::{Children, Classes, Html, Properties, classes, function_component, html};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Clone, PartialEq)]
pub enum CardVariant {
    Default,
    Primary,
    Secondary,
    Accent,
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Properties, PartialEq)]
pub struct CardProps {
    #[prop_or_default]
    pub children: Children,
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub title: Option<String>,
    #[prop_or_default]
    pub subtitle: Option<String>,
    #[prop_or_default]
    pub bordered: bool,
    #[prop_or_else(|| CardVariant::Default)]
    pub variant: CardVariant,
    #[prop_or_default]
    pub compact: bool,
    #[prop_or_default]
    pub image: Option<String>,
    #[prop_or_default]
    pub image_full: bool,
    #[prop_or_default]
    pub image_alt: Option<String>,
    #[prop_or_default]
    pub actions_slot: Option<Html>,
}

#[function_component(Card)]
pub fn card(props: &CardProps) -> Html {
    log("Rendering Card component");

    // Determine card variant class
    let variant_class = match props.variant {
        CardVariant::Default => "",
        CardVariant::Primary => "card-primary",
        CardVariant::Secondary => "card-secondary",
        CardVariant::Accent => "card-accent",
        CardVariant::Info => "card-info",
        CardVariant::Success => "card-success",
        CardVariant::Warning => "card-warning",
        CardVariant::Error => "card-error",
    };

    // Build classes
    let mut card_classes = classes!("card", "bg-base-100", "shadow-xl");

    // Add optional classes
    if !variant_class.is_empty() {
        card_classes.push(variant_class);
    }

    if props.bordered {
        card_classes.push("border");
    }

    if props.compact {
        card_classes.push("card-compact");
    }

    if props.image_full {
        card_classes.push("image-full");
    }

    // Add user-provided classes
    card_classes = classes!(card_classes, props.class.clone());

    html! {
        <div class={card_classes}>
            // Card Image
            if let Some(image_src) = &props.image {
                <figure>
                    <img
                        src={image_src.clone()}
                        alt={props.image_alt.clone().unwrap_or_default()}
                    />
                </figure>
            }

            // Card Body
            <div class="card-body">
                // Card Title
                if let Some(title) = &props.title {
                    <h2 class="card-title">{title}</h2>
                }

                // Card Subtitle
                if let Some(subtitle) = &props.subtitle {
                    <p class="text-base-content/70">{subtitle}</p>
                }

                // Card Content
                <div>
                    { for props.children.iter() }
                </div>

                // Card Actions
                if let Some(actions) = &props.actions_slot {
                    <div class="card-actions justify-end mt-4">
                        {actions.clone()}
                    </div>
                }
            </div>
        </div>
    }
}
