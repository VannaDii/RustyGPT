use i18nrs::yew::use_translation;
use wasm_bindgen::prelude::*;
use yew::{Callback, Html, classes, function_component, html, use_state};

use crate::components::{Column, DataTable, Modal, ModalSize, Row};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Represents a transaction status
#[derive(Clone, PartialEq)]
enum TransactionStatus {
    Completed,
    Pending,
    Failed,
}

impl TransactionStatus {
    /// Get the translation key for a status
    fn translation_key(&self) -> &'static str {
        match self {
            TransactionStatus::Completed => "transactions.status_completed",
            TransactionStatus::Pending => "transactions.status_pending",
            TransactionStatus::Failed => "transactions.status_failed",
        }
    }

    /// Get the display class for a status
    fn status_class(&self) -> &'static str {
        match self {
            TransactionStatus::Completed => "badge-success",
            TransactionStatus::Pending => "badge-warning",
            TransactionStatus::Failed => "badge-error",
        }
    }

    /// Parse from string
    fn from_str(s: &str) -> Self {
        match s {
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            _ => Self::Pending, // Default to Pending
        }
    }
}

/// Transaction page component
#[function_component(TransactionsPage)]
pub fn transactions_page() -> Html {
    let (i18n, _) = use_translation();

    // State for modal
    let selected_transaction = use_state(|| None::<Row>);
    let show_modal = use_state(|| false);

    // Transactions data
    let transactions = vec![
        Row {
            id: "tx-001".to_string(),
            data: {
                let mut map = std::collections::HashMap::new();
                map.insert("id".to_string(), "TX-001".to_string());
                map.insert("customer".to_string(), "Alice Johnson".to_string());
                map.insert("amount".to_string(), "$156.00".to_string());
                map.insert("status".to_string(), "completed".to_string());
                map.insert("date".to_string(), "2025-03-12".to_string());
                map
            },
        },
        Row {
            id: "tx-002".to_string(),
            data: {
                let mut map = std::collections::HashMap::new();
                map.insert("id".to_string(), "TX-002".to_string());
                map.insert("customer".to_string(), "Bob Smith".to_string());
                map.insert("amount".to_string(), "$42.50".to_string());
                map.insert("status".to_string(), "pending".to_string());
                map.insert("date".to_string(), "2025-03-13".to_string());
                map
            },
        },
        Row {
            id: "tx-003".to_string(),
            data: {
                let mut map = std::collections::HashMap::new();
                map.insert("id".to_string(), "TX-003".to_string());
                map.insert("customer".to_string(), "Carol Wu".to_string());
                map.insert("amount".to_string(), "$199.99".to_string());
                map.insert("status".to_string(), "completed".to_string());
                map.insert("date".to_string(), "2025-03-14".to_string());
                map
            },
        },
        Row {
            id: "tx-004".to_string(),
            data: {
                let mut map = std::collections::HashMap::new();
                map.insert("id".to_string(), "TX-004".to_string());
                map.insert("customer".to_string(), "Dave Lopez".to_string());
                map.insert("amount".to_string(), "$87.75".to_string());
                map.insert("status".to_string(), "failed".to_string());
                map.insert("date".to_string(), "2025-03-14".to_string());
                map
            },
        },
        Row {
            id: "tx-005".to_string(),
            data: {
                let mut map = std::collections::HashMap::new();
                map.insert("id".to_string(), "TX-005".to_string());
                map.insert("customer".to_string(), "Eve Patel".to_string());
                map.insert("amount".to_string(), "$312.00".to_string());
                map.insert("status".to_string(), "pending".to_string());
                map.insert("date".to_string(), "2025-03-15".to_string());
                map
            },
        },
    ];

    // Create sample transaction data - use regular columns that don't require render functions
    let columns = vec![
        Column {
            id: "id".to_string(),
            label: i18n.t("transactions.id").to_string(),
            render: None,
            sortable: true,
        },
        Column {
            id: "customer".to_string(),
            label: i18n.t("transactions.customer").to_string(),
            render: None,
            sortable: true,
        },
        Column {
            id: "amount".to_string(),
            label: i18n.t("transactions.amount").to_string(),
            render: None,
            sortable: true,
        },
        Column {
            id: "status".to_string(),
            label: i18n.t("transactions.status").to_string(),
            render: None, // We'll handle status rendering differently
            sortable: true,
        },
        Column {
            id: "date".to_string(),
            label: i18n.t("transactions.date").to_string(),
            render: None,
            sortable: true,
        },
    ];

    // Table row click callback
    let row_click = {
        let selected_transaction = selected_transaction.clone();
        let show_modal = show_modal.clone();

        Callback::from(move |row: Row| {
            selected_transaction.set(Some(row));
            show_modal.set(true);
        })
    };

    // Close modal callback
    let close_modal = {
        let show_modal = show_modal.clone();

        Callback::from(move |_| {
            show_modal.set(false);
        })
    };

    // Get a static ID string for the modal title
    let tx_id = selected_transaction.as_ref().map_or("".to_string(), |tx| {
        tx.data.get("id").unwrap_or(&"".to_string()).clone()
    });

    // Render a status badge - i18n value is cloned
    let i18n_for_badge = i18n.clone();
    let render_status_badge = move |status_text: &str| -> Html {
        let status = TransactionStatus::from_str(status_text);

        html! {
            <span class={classes!("badge", status.status_class())}>
                {i18n_for_badge.t(status.translation_key())}
            </span>
        }
    };

    html! {
        <div class="p-4 space-y-6">
            <h1 class="text-2xl font-bold mb-6">{i18n.t("transactions.id")}</h1>

            <div class="card bg-base-100 shadow-sm">
                <div class="card-body">
                    <DataTable
                        columns={columns}
                        rows={transactions}
                        hover={true}
                        rows_per_page={10}
                        on_row_click={row_click}
                    />
                </div>
            </div>

            // Transaction details modal
            <Modal
                title={format!("{} {}", i18n.t("transactions.id"), tx_id)}
                is_open={*show_modal}
                on_close={close_modal}
                size={ModalSize::Medium}
            >
                {
                    if let Some(transaction) = selected_transaction.as_ref() {
                        // Create a String that will live long enough
                        let status_value = transaction.data.get("status")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "pending".to_string());

                        html! {
                            <div class="space-y-4">
                                <div class="grid grid-cols-2 gap-4">
                                    <div>
                                        <h3 class="font-semibold text-sm text-base-content/70">
                                            {i18n.t("transactions.id")}
                                        </h3>
                                        <p>{transaction.data.get("id").unwrap_or(&"".to_string())}</p>
                                    </div>
                                    <div>
                                        <h3 class="font-semibold text-sm text-base-content/70">
                                            {i18n.t("transactions.customer")}
                                        </h3>
                                        <p>{transaction.data.get("customer").unwrap_or(&"".to_string())}</p>
                                    </div>
                                    <div>
                                        <h3 class="font-semibold text-sm text-base-content/70">
                                            {i18n.t("transactions.amount")}
                                        </h3>
                                        <p>{transaction.data.get("amount").unwrap_or(&"".to_string())}</p>
                                    </div>
                                    <div>
                                        <h3 class="font-semibold text-sm text-base-content/70">
                                            {i18n.t("transactions.date")}
                                        </h3>
                                        <p>{transaction.data.get("date").unwrap_or(&"".to_string())}</p>
                                    </div>
                                    <div>
                                        <h3 class="font-semibold text-sm text-base-content/70">
                                            {i18n.t("transactions.status")}
                                        </h3>
                                        {render_status_badge(&status_value)}
                                    </div>
                                </div>
                            </div>
                        }
                    } else {
                        html! {
                            <div class="text-center py-4 text-base-content/70">
                                {i18n.t("table.no_data")}
                            </div>
                        }
                    }
                }
            </Modal>
        </div>
    }
}
