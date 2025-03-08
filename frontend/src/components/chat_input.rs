use yew::{Callback, Html, Properties, function_component, html, use_state};

#[derive(Properties, PartialEq)]
pub struct ChatInputProps {
    pub on_send: Callback<String>,
}

#[function_component(ChatInput)]
pub fn chat_input(props: &ChatInputProps) -> Html {
    let input_value = use_state(|| String::new());

    let on_input = {
        let input_value = input_value.clone();
        Callback::from(move |e: yew::events::InputEvent| {
            let input: String = e.data().unwrap_or_default();
            input_value.set(input);
        })
    };

    let on_submit = {
        let input_value = input_value.clone();
        let on_send = props.on_send.clone();
        Callback::from(move |_| {
            if !input_value.is_empty() {
                on_send.emit(input_value.to_string());
                input_value.set(String::new());
            }
        })
    };

    html! {
        <div class="chat-input">
            <input type="text" value={(*input_value).clone()} oninput={on_input} />
            <button onclick={on_submit}>{ "Send" }</button>
        </div>
    }
}
