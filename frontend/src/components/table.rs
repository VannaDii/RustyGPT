use std::rc::Rc;
use yew::{
    Children, Classes, Html, Properties, classes, function_component, html, virtual_dom::VNode,
};

#[derive(Properties, PartialEq)]
pub struct TableProps {
    // Headers for the table (string or element)
    pub headers: Rc<Vec<Html>>,

    // Children for the table rows
    #[prop_or_default]
    pub children: Children,

    // Additional CSS classes
    #[prop_or_default]
    pub class: Classes,

    // Whether data is currently loading
    #[prop_or(false)]
    pub loading: bool,

    // Type/variant of table styling
    #[prop_or_default]
    pub variant: Option<TableVariant>,

    // Custom message to display when no data is available
    #[prop_or("No data available".to_string())]
    pub empty_message: String,

    // Custom loading message
    #[prop_or("Loading data...".to_string())]
    pub loading_message: String,

    // Number of skeleton rows to show when loading
    #[prop_or(5)]
    pub skeleton_rows: u8,
}

#[derive(Clone, PartialEq)]
pub enum TableVariant {
    Default,
    Zebra,
    Hover,
}

#[function_component(Table)]
pub fn table(props: &TableProps) -> Html {
    // Determine table variant classes
    let variant_class = match props.variant {
        Some(TableVariant::Zebra) => "table-zebra",
        Some(TableVariant::Hover) => "table-hover",
        _ => "", // Default variant has no additional class
    };

    // Generate table headers
    let headers = props
        .headers
        .iter()
        .map(|header| {
            html! {
                <th class="py-3">{header.clone()}</th>
            }
        })
        .collect::<Html>();

    // This function generates skeleton loading rows
    let generate_skeleton_rows = || {
        (0..props.skeleton_rows)
            .map(|_| {
                html! {
                    <tr class="animate-pulse">
                        {
                            (0..props.headers.len()).map(|_| {
                                html! {
                                    <td class="h-12">
                                        <div class="h-4 bg-base-300 rounded"></div>
                                    </td>
                                }
                            }).collect::<Html>()
                        }
                    </tr>
                }
            })
            .collect::<Html>()
    };

    // Check if we need to show "no data" message (if children are empty and not loading)
    let should_show_empty = !props.loading && props.children.is_empty();

    html! {
        <div class={classes!(
            "overflow-x-auto",
            "rounded-lg",
            "border",
            "border-base-300",
            "shadow-sm",
            props.class.clone()
        )}>
            <table class={classes!(
                "table",
                "w-full",
                variant_class,
            )}>
                <thead class="bg-base-200/50">
                    <tr>
                        {headers}
                    </tr>
                </thead>
                <tbody>
                    // Content rendering based on state
                    {get_table_body_content(props)}
                </tbody>
            </table>
        </div>
    }
}

// TableRow component for convenience
#[derive(Properties, PartialEq)]
pub struct TableRowProps {
    // Row content
    pub children: Children,

    // Custom onClick handler
    #[prop_or_default]
    pub onclick: Option<yew::Callback<web_sys::MouseEvent>>,

    // Whether the row should be highlighted
    #[prop_or(false)]
    pub highlight: bool,

    // Additional classes
    #[prop_or_default]
    pub class: Classes,
}

#[function_component(TableRow)]
pub fn table_row(props: &TableRowProps) -> Html {
    let row_classes = classes!(
        props.class.clone(),
        if props.highlight {
            "bg-base-200/50"
        } else {
            ""
        },
        if props.onclick.is_some() {
            "hover:bg-base-200 cursor-pointer"
        } else {
            ""
        },
    );

    html! {
        <tr
            class={row_classes}
            onclick={props.onclick.clone()}
        >
            {props.children.clone()}
        </tr>
    }
}

// Helper function to get the table body content based on the state
fn get_table_body_content(props: &TableProps) -> Html {
    // This function generates skeleton loading rows for showing during loading state
    let generate_skeleton_rows = || {
        (0..props.skeleton_rows)
            .map(|_| {
                html! {
                    <tr class="animate-pulse">
                        {
                            (0..props.headers.len()).map(|_| {
                                html! {
                                    <td class="h-12">
                                        <div class="h-4 bg-base-300 rounded"></div>
                                    </td>
                                }
                            }).collect::<Html>()
                        }
                    </tr>
                }
            })
            .collect::<Html>()
    };

    // Check if we need to show "no data" message
    let should_show_empty = !props.loading && props.children.is_empty();

    if props.loading {
        // Show loading skeleton
        html! {
            <>
                <tr class="text-center text-base-content/60">
                    <td colspan={props.headers.len().to_string()} class="py-8">
                        <div class="flex flex-col items-center justify-center">
                            <div class="loading loading-spinner loading-md mb-2"></div>
                            <p>{&props.loading_message}</p>
                        </div>
                    </td>
                </tr>
                {generate_skeleton_rows()}
            </>
        }
    } else if should_show_empty {
        // Show empty state
        html! {
            <tr class="text-center text-base-content/60">
                <td colspan={props.headers.len().to_string()} class="py-10">
                    <div class="flex flex-col items-center justify-center">
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 mb-2 text-base-content/30" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
                        </svg>
                        <p>{&props.empty_message}</p>
                    </div>
                </td>
            </tr>
        }
    } else {
        // Show the actual data by wrapping children in a fragment
        html! { <>{for props.children.iter()}</> }
    }
}

// TableCell component for convenience
#[derive(Properties, PartialEq)]
pub struct TableCellProps {
    // Cell content
    #[prop_or_default]
    pub children: Children,

    // Additional classes
    #[prop_or_default]
    pub class: Classes,

    // Colspan attribute
    #[prop_or(1)]
    pub colspan: u32,
}

#[function_component(TableCell)]
pub fn table_cell(props: &TableCellProps) -> Html {
    html! {
        <td
            class={props.class.clone()}
            colspan={props.colspan.to_string()}
        >
            {props.children.clone()}
        </td>
    }
}
