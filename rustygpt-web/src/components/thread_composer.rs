use web_sys::HtmlTextAreaElement;
use yew::{Callback, Html, Properties, TargetCast, classes, function_component, html};

#[derive(Properties, PartialEq, Clone)]
pub struct ThreadComposerProps {
    pub text: String,
    pub on_text_change: Callback<String>,
    pub on_submit: Callback<()>,
    #[prop_or(false)]
    pub disabled: bool,
    #[prop_or_default]
    pub placeholder: String,
    #[prop_or_default]
    pub submit_label: String,
    #[prop_or(false)]
    pub show_cancel: bool,
    #[prop_or_default]
    pub on_cancel: Callback<()>,
}

#[function_component(ThreadComposer)]
pub fn thread_composer(props: &ThreadComposerProps) -> Html {
    let on_change = {
        let on_text_change = props.on_text_change.clone();
        Callback::from(move |event: yew::events::InputEvent| {
            let target: HtmlTextAreaElement = event.target_unchecked_into();
            on_text_change.emit(target.value());
        })
    };

    let on_keydown = {
        let on_submit = props.on_submit.clone();
        let disabled = props.disabled;
        Callback::from(move |event: yew::events::KeyboardEvent| {
            if event.key() == "Enter" && !event.shift_key() && !disabled {
                event.prevent_default();
                on_submit.emit(());
            }
        })
    };

    let on_submit = {
        let on_submit = props.on_submit.clone();
        Callback::from(move |event: yew::events::SubmitEvent| {
            event.prevent_default();
            on_submit.emit(());
        })
    };

    let on_cancel = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |_| on_cancel.emit(()))
    };

    let submit_label = if props.submit_label.is_empty() {
        String::from("Send")
    } else {
        props.submit_label.clone()
    };

    html! {
        <form class="space-y-3" onsubmit={on_submit}>
            <textarea
                class={classes!("textarea", "textarea-bordered", "w-full", "min-h-[6rem]")}
                placeholder={props.placeholder.clone()}
                value={props.text.clone()}
                oninput={on_change}
                onkeydown={on_keydown}
                disabled={props.disabled}
            />
            <div class="flex items-center justify-between">
                { if props.show_cancel {
                    html! {
                        <button
                            class="btn btn-ghost"
                            type="button"
                            onclick={on_cancel}
                        >
                            {"Cancel"}
                        </button>
                    }
                } else {
                    html! {}
                }}
                <button
                    class="btn btn-primary"
                    type="submit"
                    disabled={props.disabled || props.text.trim().is_empty()}
                >
                    { submit_label }
                </button>
            </div>
        </form>
    }
}
