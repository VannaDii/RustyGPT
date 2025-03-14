use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::{
    Callback, Children, Classes, Html, Properties, classes, function_component, html, use_state,
};
use yew_icons::{Icon, IconId};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Column definition for DataTable
#[derive(Clone, PartialEq)]
pub struct Column {
    /// Column ID
    pub id: String,

    /// Column header text
    pub label: String,

    /// Optional renderer function
    pub render: Option<fn(&str, &Row) -> Html>,

    /// Is column sortable
    pub sortable: bool,
}

/// Row data for DataTable
#[derive(Clone, PartialEq)]
pub struct Row {
    /// Unique row ID
    pub id: String,

    /// Map of column_id -> cell_value
    pub data: std::collections::HashMap<String, String>,
}

/// Sort direction
#[derive(Clone, Copy, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Table properties
#[derive(Properties, PartialEq)]
pub struct DataTableProps {
    /// Table columns
    pub columns: Vec<Column>,

    /// Table data rows
    pub rows: Vec<Row>,

    /// Optional class for the table
    #[prop_or_default]
    pub class: Classes,

    /// Optional loading state
    #[prop_or(false)]
    pub loading: bool,

    /// Optional zebra striping
    #[prop_or(false)]
    pub zebra: bool,

    /// Optional hover effect
    #[prop_or(true)]
    pub hover: bool,

    /// Optional rows per page
    #[prop_or(10)]
    pub rows_per_page: usize,

    /// Optional default sort column
    #[prop_or_default]
    pub default_sort_column: Option<String>,

    /// Optional default sort direction
    #[prop_or(SortDirection::Ascending)]
    pub default_sort_direction: SortDirection,

    /// Optional on-row-click callback
    #[prop_or_default]
    pub on_row_click: Option<Callback<Row>>,
}

#[function_component(DataTable)]
pub fn data_table(props: &DataTableProps) -> Html {
    let (_i18n, ..) = use_translation();

    // Initialize state for sorting
    let sort_column = use_state(|| props.default_sort_column.clone());
    let sort_direction = use_state(|| props.default_sort_direction);

    // Initialize state for pagination
    let current_page = use_state(|| 0);

    // Handle sort change
    let handle_sort = {
        let sort_column = sort_column.clone();
        let sort_direction = sort_direction.clone();

        Callback::from(move |column_id: String| {
            if Some(column_id.clone()) == *sort_column {
                // Toggle direction if same column
                sort_direction.set(match *sort_direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                });
            } else {
                // Set new column and default direction
                sort_column.set(Some(column_id));
                sort_direction.set(SortDirection::Ascending);
            }
        })
    };

    // Sort rows if needed
    let sorted_rows = {
        let mut rows = props.rows.clone();

        if let Some(column_id) = sort_column.as_ref() {
            rows.sort_by(|a, b| {
                let a_val = a.data.get(column_id).cloned().unwrap_or_default();
                let b_val = b.data.get(column_id).cloned().unwrap_or_default();

                match *sort_direction {
                    SortDirection::Ascending => a_val.cmp(&b_val),
                    SortDirection::Descending => b_val.cmp(&a_val),
                }
            });
        }

        rows
    };

    // Paginate rows
    let total_pages = (sorted_rows.len() as f64 / props.rows_per_page as f64).ceil() as usize;
    let start_idx = *current_page * props.rows_per_page;
    let end_idx = (start_idx + props.rows_per_page).min(sorted_rows.len());
    let visible_rows = &sorted_rows[start_idx..end_idx];

    // Handle page change
    let handle_page_change = {
        let current_page = current_page.clone();

        Callback::from(move |page: usize| {
            current_page.set(page);
        })
    };

    // Render the table
    html! {
        <div>
            <div class="overflow-x-auto">
                <table class={classes!(
                    "table",
                    "w-full",
                    if props.zebra { "table-zebra" } else { "" },
                    if props.hover { "table-hover" } else { "" },
                    props.class.clone()
                )}>
                    <thead>
                        <tr>
                            {
                                props.columns.iter().map(|column| {
                                    let column_id = column.id.clone();
                                    let is_sorted = sort_column.as_ref() == Some(&column.id);
                                    let sort_callback = if column.sortable {
                                        let handle_sort = handle_sort.clone();
                                        let column_id = column_id.clone();

                                        Some(Callback::from(move |_| {
                                            handle_sort.emit(column_id.clone());
                                        }))
                                    } else {
                                        None
                                    };

                                    html! {
                                        <th
                                            key={column_id.clone()}
                                            class={classes!(
                                                if column.sortable { "cursor-pointer select-none" } else { "" }
                                            )}
                                            onclick={sort_callback}
                                        >
                                            <div class="flex items-center gap-1">
                                                <span>{column.label.clone()}</span>

                                                if is_sorted {
                                                    <Icon
                                                        icon_id={
                                                            match *sort_direction {
                                                                SortDirection::Ascending => IconId::HeroiconsOutlineChevronUp,
                                                                SortDirection::Descending => IconId::HeroiconsOutlineChevronDown,
                                                            }
                                                        }
                                                        class="h-4 w-4"
                                                    />
                                                }
                                            </div>
                                        </th>
                                    }
                                }).collect::<Html>()
                            }
                        </tr>
                    </thead>
                    <tbody>
                        if props.loading {
                            // Skeleton loading rows
                            { (0..5).map(|i| html! {
                                <tr key={i}>
                                    {
                                        props.columns.iter().map(|column| {
                                            html! {
                                                <td key={column.id.clone()}>
                                                    <div class="h-4 bg-base-300 animate-pulse rounded"></div>
                                                </td>
                                            }
                                        }).collect::<Html>()
                                    }
                                </tr>
                            }).collect::<Html>() }
                        } else if visible_rows.is_empty() {
                            // Empty state
                            <tr>
                                <td colspan={props.columns.len().to_string()} class="text-center py-4">
                                    <p class="text-base-content/70">{_i18n.t("table.no_data")}</p>
                                </td>
                            </tr>
                        } else {
                            // Data rows
                            {
                                visible_rows.iter().map(|row| {
                                    let row_clone = row.clone();
                                    let row_click = if let Some(on_click) = &props.on_row_click {
                                        let callback = on_click.clone();
                                        let row = row.clone();

                                        Some(Callback::from(move |_| {
                                            callback.emit(row.clone());
                                        }))
                                    } else {
                                        None
                                    };

                                    html! {
                                        <tr
                                            key={row.id.clone()}
                                            class={classes!(
                                                if props.on_row_click.is_some() { "cursor-pointer" } else { "" }
                                            )}
                                            onclick={row_click}
                                        >
                                            {
                                                props.columns.iter().map(|column| {
                                                    let cell_value = row.data.get(&column.id).cloned().unwrap_or_default();

                                                    html! {
                                                        <td key={column.id.clone()}>
                                                            if let Some(render_fn) = column.render {
                                                                { render_fn(&cell_value, &row_clone) }
                                                            } else {
                                                                { cell_value }
                                                            }
                                                        </td>
                                                    }
                                                }).collect::<Html>()
                                            }
                                        </tr>
                                    }
                                }).collect::<Html>()
                            }
                        }
                    </tbody>
                </table>
            </div>

            // Pagination
            if total_pages > 1 {
                <div class="flex justify-center mt-4">
                    <div class="join">
                        <button
                            class="join-item btn btn-sm"
                            disabled={*current_page == 0}
                            onclick={
                                let handle_page_change = handle_page_change.clone();
                                Callback::from(move |_| {
                                    handle_page_change.emit(0);
                                })
                            }
                        >
                            <Icon icon_id={IconId::HeroiconsOutlineChevronDoubleLeft} class="h-4 w-4" />
                        </button>

                        <button
                            class="join-item btn btn-sm"
                            disabled={*current_page == 0}
                            onclick={
                                let handle_page_change = handle_page_change.clone();
                                let current_page = *current_page;
                                Callback::from(move |_| {
                                    if current_page > 0 {
                                        handle_page_change.emit(current_page - 1);
                                    }
                                })
                            }
                        >
                            <Icon icon_id={IconId::HeroiconsOutlineChevronLeft} class="h-4 w-4" />
                        </button>

                        {
                            // Page buttons
                            (0..total_pages).map(|page| {
                                let is_current = page == *current_page;
                                let handle_page_change = handle_page_change.clone();

                                html! {
                                    <button
                                        key={page}
                                        class={classes!(
                                            "join-item", "btn", "btn-sm",
                                            if is_current { "btn-active" } else { "" }
                                        )}
                                        onclick={
                                            let handle_page_change = handle_page_change.clone();
                                            Callback::from(move |_| {
                                                handle_page_change.emit(page);
                                            })
                                        }
                                    >
                                        {page + 1}
                                    </button>
                                }
                            }).collect::<Html>()
                        }

                        <button
                            class="join-item btn btn-sm"
                            disabled={*current_page >= total_pages - 1}
                            onclick={
                                let handle_page_change = handle_page_change.clone();
                                let current_page = *current_page;
                                Callback::from(move |_| {
                                    handle_page_change.emit(current_page + 1);
                                })
                            }
                        >
                            <Icon icon_id={IconId::HeroiconsOutlineChevronRight} class="h-4 w-4" />
                        </button>

                        <button
                            class="join-item btn btn-sm"
                            disabled={*current_page >= total_pages - 1}
                            onclick={
                                let handle_page_change = handle_page_change.clone();
                                Callback::from(move |_| {
                                    handle_page_change.emit(total_pages - 1);
                                })
                            }
                        >
                            <Icon icon_id={IconId::HeroiconsOutlineChevronDoubleRight} class="h-4 w-4" />
                        </button>
                    </div>
                </div>
            }
        </div>
    }
}
