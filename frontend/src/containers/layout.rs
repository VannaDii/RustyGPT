use crate::containers::header::Header;
use crate::containers::page_content::PageContent;
use crate::routes::AppRoute;
use wasm_bindgen::prelude::*;
use web_sys::window;
use yew::{Children, Html, Properties, classes, function_component, html, use_effect_with};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Properties, PartialEq)]
pub struct LayoutProps {
    pub children: Children,
    #[prop_or_default]
    pub current_route: Option<AppRoute>,
    #[prop_or_default]
    pub header_routes: Option<Vec<AppRoute>>,
}

#[function_component(Layout)]
pub fn layout(props: &LayoutProps) -> Html {
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
    let header_routes = props.header_routes.clone();

    html! {
    <>
        <Header {header_routes} current_route={props.current_route.clone()} />
        <div class="min-h-screen bg-base-100 drawer lg:drawer-open">
            <div class="drawer-content flex flex-col">
                <main class={classes!(
                    "flex-grow",
                    "p-4",
                    "transition-all",
                    "duration-300",
                    "lg:ml-0"
                )}>
                    <PageContent>
                        {props.children.clone()}
                    </PageContent>
                </main>
                <footer class="footer footer-center p-4 border-t border-base-300 text-base-content">
                    <div>
                        <p>{"© 2025 RustyGPT · Powered by Rust, Yew and DaisyUI"}</p>
                    </div>
                </footer>
            </div>
        </div>
    </>
    }
}
