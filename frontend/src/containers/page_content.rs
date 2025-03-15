use yew::{Children, Classes, Html, Properties, classes, function_component, html};

#[derive(Properties, PartialEq)]
pub struct PageContentProps {
    // Main content
    #[prop_or_default]
    pub children: Children,

    // Additional classes for the page container
    #[prop_or_default]
    pub class: Classes,

    // Whether to add padding to the content
    #[prop_or(true)]
    pub padding: bool,

    // Whether to use a card-like container with shadow and border
    #[prop_or(true)]
    pub container: bool,
}

/// Page content container component
/// Provides consistent styling for page content across the app
#[function_component(PageContent)]
pub fn page_content(props: &PageContentProps) -> Html {
    // Generate the appropriate classes based on props
    let container_classes = classes!(
        if props.container {
            classes!(
                "bg-base-100",
                "rounded-box",
                "shadow-sm",
                "border",
                "border-base-300"
            )
        } else {
            Classes::new()
        },
        if props.padding { "p-4 md:p-6" } else { "" },
        props.class.clone()
    );

    html! {
        <div class={container_classes}>
            {props.children.clone()}
        </div>
    }
}
