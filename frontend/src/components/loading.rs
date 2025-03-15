use yew::{Html, function_component, html};

#[function_component(Loading)]
pub fn loading() -> Html {
    html! {
        <div class="flex flex-col items-center justify-center h-full animate-fadeIn">
            <div class="bg-base-200 p-6 rounded-lg shadow-md flex flex-col items-center">
                <div class="text-xl font-medium flex items-center gap-2">
                    <i class="fas fa-robot text-primary"></i>
                    <span>{"RustyGPT"}</span>
                </div>
                <div class="mt-3 flex items-center">
                    <span>{"Loading"}</span>
                    <span class="typing-dot"></span>
                    <span class="typing-dot"></span>
                    <span class="typing-dot"></span>
                </div>
            </div>
        </div>
    }
}
