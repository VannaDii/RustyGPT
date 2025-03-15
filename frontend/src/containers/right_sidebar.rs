use crate::models::right_sidebar::{RightSidebarAction, RightSidebarBodyType, RightSidebarState};
use yew::{Html, function_component, html};
use yewdux::{use_dispatch, use_selector};

#[function_component(RightSidebar)]
pub fn right_sidebar() -> Html {
    let dispatch = use_dispatch::<RightSidebarState>();
    let state = use_selector(|state: &RightSidebarState| state.clone());
    let close = dispatch.apply_callback(|_| RightSidebarAction::CloseSidebar);

    let open_styles_container = if state.is_open {
        " transition-opacity opacity-100 duration-500 translate-x-0"
    } else {
        " transition-all delay-500 opacity-0 translate-x-full"
    };
    let open_styles_section = if state.is_open {
        " translate-x-0"
    } else {
        " translate-x-full"
    };

    html! {
      <div class={"fixed overflow-hidden z-20 bg-gray-900 bg-opacity-25 inset-0 transform ease-in-out".to_owned() + open_styles_container}>
        <section class={"w-80 md:w-96  right-0 absolute bg-base-100 h-full shadow-xl delay-400 duration-500 ease-in-out transition-all transform".to_owned() + open_styles_section}>
            <div class="relative  pb-5 flex flex-col  h-full">
                <div class="navbar flex pl-4 pr-4 shadow-md ">
                    <button class="float-left btn btn-circle btn-outline btn-sm" onclick={&close}>{"X"}</button>
                    <span class="ml-2 font-bold text-xl">{state.header.clone()}</span>
                </div>
                <div class="overflow-y-scroll pl-4 pr-4">
                    <div class="flex flex-col w-full">
                    {
                        match state.body_type {
                            RightSidebarBodyType::Notifications => html! {<div>{"Notifications"}</div>},
                            RightSidebarBodyType::Events => html! {<div>{"Events"}</div>},
                            RightSidebarBodyType::Default => html! {<div></div>},
                        }
                    }
                    </div>
                </div>
                </div>
        </section>
        <section class="w-screen h-full cursor-pointer" onclick={&close} ></section>
    </div>
    }
}
