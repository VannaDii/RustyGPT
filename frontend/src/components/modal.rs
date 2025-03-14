use web_sys::MouseEvent;
use yew::{Callback, Children, Classes, Html, Properties, classes, function_component, html};

#[derive(Properties, PartialEq)]
pub struct ModalProps {
    // Whether the modal is currently visible
    pub show: bool,

    // Callback when the modal is requested to be closed (via backdrop or X button)
    #[prop_or_default]
    pub on_close: Option<Callback<MouseEvent>>,

    // Title displayed at the top of the modal
    #[prop_or_default]
    pub title: Option<String>,

    // Modal content
    #[prop_or_default]
    pub children: Children,

    // Footer content (typically buttons)
    #[prop_or_default]
    pub footer: Option<Html>,

    // Additional CSS classes for customization
    #[prop_or_default]
    pub class: Classes,

    // Custom width class (default is max-w-lg)
    #[prop_or(Classes::from("max-w-lg"))]
    pub width_class: Classes,

    // Whether to show the close button in the top-right
    #[prop_or(true)]
    pub show_close_button: bool,

    // Custom icon/content to display before the title
    #[prop_or_default]
    pub icon: Option<Html>,
}

#[function_component(Modal)]
pub fn modal(props: &ModalProps) -> Html {
    if !props.show {
        return html! {};
    }

    // Prevent event bubbling to parent elements
    let stop_propagation = Callback::from(|e: MouseEvent| {
        e.stop_propagation();
    });

    let on_backdrop_click = {
        let on_close = props.on_close.clone();

        Callback::from(move |e: MouseEvent| {
            if let Some(on_close) = on_close.clone() {
                on_close.emit(e);
            }
        })
    };

    let on_close_click = {
        let on_close = props.on_close.clone();

        Callback::from(move |e: MouseEvent| {
            if let Some(on_close) = on_close.clone() {
                on_close.emit(e);
            }
        })
    };

    html! {
        <div
            class="fixed inset-0 z-50 flex items-center justify-center bg-base-300/50 p-4 backdrop-blur-sm transition-all duration-300"
            onclick={on_backdrop_click}
        >
            <div
                class={classes!(
                    "modal-box",
                    "bg-base-100",
                    "rounded-lg",
                    "shadow-xl",
                    "border",
                    "border-base-300",
                    "flex",
                    "flex-col",
                    "relative",
                    "overflow-hidden",
                    "max-h-[90vh]",
                    props.width_class.clone(),
                    props.class.clone()
                )}
                onclick={stop_propagation}
            >
                // Modal Header
                <div class="flex items-center justify-between p-4 border-b border-base-300">
                    <div class="flex items-center gap-3">
                        {
                            if let Some(icon) = &props.icon {
                                html! { <div class="text-primary">{icon.clone()}</div> }
                            } else {
                                html! {}
                            }
                        }

                        {
                            if let Some(title) = &props.title {
                                html! { <h3 class="font-bold text-lg">{title}</h3> }
                            } else {
                                html! {}
                            }
                        }
                    </div>

                    {
                        if props.show_close_button {
                            html! {
                                <button
                                    class="btn btn-ghost btn-circle btn-sm"
                                    onclick={on_close_click}
                                    aria-label="Close"
                                >
                                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                                        <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
                                    </svg>
                                </button>
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>

                // Modal Body
                <div class="p-4 overflow-y-auto">
                    {props.children.clone()}
                </div>

                // Modal Footer (if provided)
                {
                    if let Some(footer) = &props.footer {
                        html! {
                            <div class="p-4 border-t border-base-300 flex justify-end gap-2">
                                {footer.clone()}
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
            </div>
        </div>
    }
}
