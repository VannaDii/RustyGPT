use web_sys::window;
use yew::{Html, function_component, html, use_effect_with, use_node_ref};
use yew_hooks::use_location;

use crate::containers::header::Header;

#[function_component(PageContent)]
pub fn page_content() -> Html {
    let main_content_ref = use_node_ref();
    let location = use_location();

    use_effect_with(location.pathname.clone(), |_| {
        window().unwrap().scroll_to_with_x_and_y(0.0, 0.0);
    });

    html! {
        <div class="drawer-content flex flex-col ">
            <Header/>
            <main class="flex-1 overflow-y-auto md:pt-4 pt-4 px-6  bg-base-200" ref={main_content_ref}>
                <div class="h-16"></div>
            </main>
        </div>
    }
}
