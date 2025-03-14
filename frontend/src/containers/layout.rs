use crate::containers::{
    left_sidebar::LeftSidebar, page_content::PageContent, right_sidebar::RightSidebar,
};
use yew::{Html, function_component, html};

#[function_component(Layout)]
pub fn app() -> Html {
    html! {
    <>
      <div class="drawer  lg:drawer-open">
          <input id="left-sidebar-drawer" type="checkbox" class="drawer-toggle" />
          <PageContent />
          <LeftSidebar />
      </div>
      <RightSidebar />
    </>
    }
}
