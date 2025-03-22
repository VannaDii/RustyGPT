use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use web_sys::window;
use yew::{
    Callback, Children, Classes, Html, Properties, classes, function_component, html,
    use_effect_with, use_state,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Size variants for the modal
#[derive(Clone, Copy, PartialEq)]
pub enum ModalSize {
    /// Small modal (max-w-sm)
    Small,
    /// Medium modal (max-w-md) - Default
    Medium,
    /// Large modal (max-w-lg)
    Large,
    /// Extra large modal (max-w-xl)
    ExtraLarge,
    /// Full width modal (max-w-full)
    Full,
}

/// Modal dialog properties
#[derive(Properties, PartialEq)]
pub struct ModalProps {
    /// Modal title
    pub title: String,

    /// Modal content
    pub children: Children,

    /// Whether the modal is open
    pub is_open: bool,

    /// Callback when closing the modal
    pub on_close: Callback<()>,

    /// Optional footer content
    #[prop_or_default]
    pub footer: Option<Html>,

    /// Optional size variant
    #[prop_or(ModalSize::Medium)]
    pub size: ModalSize,

    /// Optional class for the modal
    #[prop_or_default]
    pub class: Classes,

    /// Whether to close on backdrop click
    #[prop_or(true)]
    pub close_on_backdrop: bool,

    /// Whether to close on escape key
    #[prop_or(true)]
    pub close_on_escape: bool,
}

/// Modal dialog component
#[function_component(Modal)]
pub fn modal(props: &ModalProps) -> Html {
    let (_i18n, ..) = use_translation();
    let is_mounted = use_state(|| false);

    // Effect for body style
    {
        let is_open = props.is_open;
        use_effect_with(is_open, move |&is_open| {
            // Set body overflow based on modal state
            if let Some(window) = window() {
                if let Some(document) = window.document() {
                    if let Some(body) = document.body() {
                        let style_value = if is_open { "hidden" } else { "auto" };
                        let _ = body.style().set_property("overflow", style_value);
                    }
                }
            }

            // Return a cleanup function that doesn't use any captured variables
            || {
                // Cleanup will be handled by the next effect call
            }
        });
    }

    // Separate effect for escape key handling
    {
        let is_open = props.is_open;
        let close_on_escape = props.close_on_escape;
        let on_close = props.on_close.clone();

        use_effect_with(
            (is_open, close_on_escape),
            move |(is_open, close_on_escape)| {
                let handler_opt = if *is_open && *close_on_escape {
                    let on_close = on_close.clone();
                    let handler = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                        if event.key() == "Escape" {
                            on_close.emit(());
                        }
                    }) as Box<dyn FnMut(_)>);

                    if let Some(window) = window() {
                        window
                            .add_event_listener_with_callback(
                                "keydown",
                                handler.as_ref().unchecked_ref(),
                            )
                            .unwrap_or_default();
                    }

                    Some(handler)
                } else {
                    None
                };

                // Return cleanup function
                move || {
                    if let Some(handler) = handler_opt {
                        if let Some(window) = window() {
                            window
                                .remove_event_listener_with_callback(
                                    "keydown",
                                    handler.as_ref().unchecked_ref(),
                                )
                                .unwrap_or_default();
                        }
                        handler.forget();
                    }
                }
            },
        );
    }

    // After first render, set mounted to true for animations
    {
        let is_mounted = is_mounted.clone();
        use_effect_with((), move |_| {
            is_mounted.set(true);
            || {}
        });
    }

    // Handle backdrop click
    let backdrop_click = {
        let on_close = props.on_close.clone();
        let close_on_backdrop = props.close_on_backdrop;

        Callback::from(move |_| {
            if close_on_backdrop {
                on_close.emit(());
            }
        })
    };

    // Prevent click propagation for modal content
    let modal_click = Callback::from(|e: yew::events::MouseEvent| {
        e.stop_propagation();
    });

    // Handle close button click
    let close_click = {
        let on_close = props.on_close.clone();

        Callback::from(move |_| {
            on_close.emit(());
        })
    };

    // Size class mapping
    let size_class = match props.size {
        ModalSize::Small => "max-w-sm",
        ModalSize::Medium => "max-w-md",
        ModalSize::Large => "max-w-lg",
        ModalSize::ExtraLarge => "max-w-xl",
        ModalSize::Full => "max-w-full",
    };

    // If modal is not open, render nothing
    if !props.is_open {
        return html! { <></> };
    }

    html! {
        <div
            class={classes!(
                "fixed",
                "inset-0",
                "z-50",
                "flex",
                "items-center",
                "justify-center",
                "transition-opacity",
                "duration-300",
                if *is_mounted { "opacity-100" } else { "opacity-0" },
            )}
            onclick={backdrop_click}
        >
            // Backdrop with blur effect
            <div
                class="absolute inset-0 bg-black/50 backdrop-blur-sm"
                aria-hidden="true"
            ></div>

            // Modal dialog
            <div
                class={classes!(
                    "modal-box",
                    "relative",
                    "bg-base-100",
                    "rounded-lg",
                    "shadow-lg",
                    "w-full",
                    size_class,
                    "z-10",
                    "transition-all",
                    "duration-300",
                    "transform",
                    if *is_mounted { "scale-100" } else { "scale-95" },
                    props.class.clone(),
                )}
                onclick={modal_click}
            >
                // Header
                <div class="flex items-center justify-between border-b border-base-200 p-4">
                    <h3 class="text-lg font-semibold">{&props.title}</h3>
                    <button
                        class="btn btn-sm btn-ghost btn-square"
                        onclick={close_click}
                        aria-label={_i18n.t("modal.close")}
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                // Body
                <div class="p-4">
                    {props.children.clone()}
                </div>

                // Footer (optional)
                if let Some(footer) = &props.footer {
                    <div class="border-t border-base-200 p-4">
                        {footer.clone()}
                    </div>
                }
            </div>
        </div>
    }
}
