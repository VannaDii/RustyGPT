use crate::components::language_selector::LanguageSelector;
use yew::{Callback, Html, UseStateHandle, function_component, html, use_context, use_state};

#[function_component(Layout)]
pub fn app() -> Html {
    html! {
      <>
      <div class="drawer  lg:drawer-open">
          <input id="left-sidebar-drawer" type="checkbox" class="drawer-toggle" />
      </div>
      <LanguageSelector />
      </>
    }
}
