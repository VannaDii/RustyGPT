use yew::{Html, function_component, html};

#[function_component(LeftSidebar)]
pub fn left_sidebar() -> Html {
    html! {
      <div class="drawer-side  z-30  ">
        <label htmlFor="left-sidebar-drawer" class="drawer-overlay"></label>
        <ul class="menu  pt-2 w-80 bg-base-100 min-h-full   text-base-content">
          <li class="mb-2 font-semibold text-xl"><i class="fas fa-robot text-primary"></i></li>
        </ul>
      </div>
    }
}
