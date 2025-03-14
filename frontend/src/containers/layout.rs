use crate::containers::header::Header;
use crate::containers::left_sidebar::LeftSidebar;
use crate::containers::page_content::PageContent;
use crate::models::sidebar_store::SidebarStore;
use crate::routes::Routes;
use wasm_bindgen::prelude::*;
use web_sys::window;
use yew::{
    Children, Html, Properties, classes, function_component, html, use_effect_with, use_state,
};
use yewdux::prelude::use_store;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Properties, PartialEq)]
pub struct LayoutProps {
    pub children: Children,
    #[prop_or_default]
    pub current_route: Option<Routes>,
}

#[function_component(Layout)]
pub fn layout(props: &LayoutProps) -> Html {
    // Setup sidebar state

    let (sidebar_state, _) = use_store::<SidebarStore>();
    let is_sidebar_collapsed = sidebar_state.state.is_collapsed;

    // Adds data-theme attribute to html tag for theme support
    use_effect_with((), |_| {
        if let Some(window) = window() {
            if let Some(document) = window.document() {
                if let Some(html_element) = document.document_element() {
                    html_element
                        .set_attribute("data-theme", "dark")
                        .unwrap_or_default();
                }
            }
        }
        || {}
    });

    html! {
        <div class="min-h-screen bg-base-100 drawer lg:drawer-open">
            // Mobile drawer control
            <input id="left-sidebar-drawer" type="checkbox" class="drawer-toggle" />

            <div class="drawer-content flex flex-col">
                // Header at the top
                <Header current_route={props.current_route.clone()} />

                // Main content area with auto-adjusting width
                <main class={classes!(
                    "flex-grow",
                    "p-4",
                    "transition-all",
                    "duration-300",
                    "lg:ml-0", // No margin on large screens as drawer is open
                    if is_sidebar_collapsed { "lg:ml-20" } else { "lg:ml-80" }
                )}>
                    <PageContent>
                        {props.children.clone()}
                    </PageContent>
                </main>

                // Footer
                <footer class="footer footer-center p-4 border-t border-base-300 text-base-content">
                    <div>
                        <p>{"© 2025 RustyGPT · Powered by Yew and DaisyUI"}</p>
                    </div>
                </footer>
            </div>

            // Left sidebar
            <LeftSidebar />
        </div>
    }
}
