use wasm_bindgen::prelude::*;
use web_sys::MouseEvent;
use yew::{Callback, Children, Classes, Html, Properties, classes, function_component, html};
use yew_icons::{Icon, IconId};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Clone, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Accent,
    Info,
    Success,
    Warning,
    Error,
    Ghost,
    Link,
}

#[derive(Clone, PartialEq)]
pub enum ButtonSize {
    Large,
    Medium,
    Small,
    Tiny,
}

#[derive(Properties, PartialEq)]
pub struct ButtonProps {
    #[prop_or_default]
    pub children: Children,
    #[prop_or_else(|| ButtonVariant::Primary)]
    pub variant: ButtonVariant,
    #[prop_or_else(|| ButtonSize::Medium)]
    pub size: ButtonSize,
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub icon: Option<IconId>,
    #[prop_or_default]
    pub icon_position: Option<String>,
    #[prop_or_default]
    pub onclick: Callback<MouseEvent>,
    #[prop_or_default]
    pub disabled: bool,
    #[prop_or_default]
    pub loading: bool,
    #[prop_or_default]
    pub outlined: bool,
    #[prop_or_default]
    pub circle: bool,
    #[prop_or_default]
    pub square: bool,
    #[prop_or_default]
    pub block: bool,
}

#[function_component(Button)]
pub fn button(props: &ButtonProps) -> Html {
    log("Rendering Button component");

    let variant_class = match props.variant {
        ButtonVariant::Primary => "btn-primary",
        ButtonVariant::Secondary => "btn-secondary",
        ButtonVariant::Accent => "btn-accent",
        ButtonVariant::Info => "btn-info",
        ButtonVariant::Success => "btn-success",
        ButtonVariant::Warning => "btn-warning",
        ButtonVariant::Error => "btn-error",
        ButtonVariant::Ghost => "btn-ghost",
        ButtonVariant::Link => "btn-link",
    };

    let size_class = match props.size {
        ButtonSize::Large => "btn-lg",
        ButtonSize::Medium => "",
        ButtonSize::Small => "btn-sm",
        ButtonSize::Tiny => "btn-xs",
    };

    let shape_class = if props.circle {
        "btn-circle"
    } else if props.square {
        "btn-square"
    } else {
        ""
    };

    // Build dynamic classes
    let mut button_classes = classes!("btn", variant_class);

    // Add optional classes
    if !size_class.is_empty() {
        button_classes.push(size_class);
    }

    if !shape_class.is_empty() {
        button_classes.push(shape_class);
    }

    if props.outlined {
        button_classes.push("btn-outline");
    }

    if props.block {
        button_classes.push("btn-block");
    }

    if props.loading {
        button_classes.push("loading");
    }

    // Add any user-provided classes
    button_classes = classes!(button_classes, props.class.clone());

    // Icon layout
    let icon_position = props
        .icon_position
        .clone()
        .unwrap_or_else(|| "left".to_string());

    html! {
        <button
            class={button_classes}
            onclick={props.onclick.clone()}
            disabled={props.disabled || props.loading}
        >
            if props.icon.is_some() && icon_position == "left" {
                <Icon icon_id={props.icon.unwrap()} class="h-5 w-5 mr-2" />
            }
            { for props.children.iter() }
            if props.icon.is_some() && icon_position == "right" {
                <Icon icon_id={props.icon.unwrap()} class="h-5 w-5 ml-2" />
            }
        </button>
    }
}
