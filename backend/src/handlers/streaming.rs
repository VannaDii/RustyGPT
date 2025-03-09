use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::Response,
};
use serde_json::json;
use shared::models::MessageChunk;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{Mutex, mpsc},
    time::Duration,
};
use uuid::Uuid;

/// Shared state for managing SSE connections
pub type SharedState = Arc<Mutex<HashMap<Uuid, mpsc::Sender<String>>>>;

/// A simple handler that doesn't require state
pub async fn simple_sse_handler(Path(user_id): Path<String>) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(axum::body::Body::from(format!(
            "data: Connected to SSE stream for user {}\n\n",
            user_id
        )))
        .unwrap()
}

/// Handler for SSE connections with state
pub async fn sse_handler(
    Path(user_id): Path<String>,
    State(state): State<SharedState>,
) -> Response {
    // Parse user_id from string to Uuid
    let user_id = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            // Return error response for invalid UUID
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(axum::body::Body::from("Invalid user ID"))
                .unwrap();
        }
    };

    // Create a channel for this user
    let (tx, _rx) = mpsc::channel::<String>(100);

    // Store the sender in shared state
    {
        let mut state = state.lock().await;
        state.insert(user_id, tx.clone());
    }

    // Create a simple keep-alive task that sends a comment every 30 seconds
    // This helps keep the connection alive
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if tx.send(": keep-alive\n\n".to_string()).await.is_err() {
                break;
            }
        }
    });

    // Return a streaming response
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(axum::body::Body::from("data: Connected to SSE stream\n\n"))
        .unwrap()
}

/// Stream a partial response to a user
pub async fn stream_partial_response(
    state: SharedState,
    user_id: Uuid,
    conversation_id: Uuid,
    message_id: Uuid,
    chunks: Vec<String>,
) {
    let state = state.lock().await;
    if let Some(sender) = state.get(&user_id) {
        let chunks_len = chunks.len();

        for (i, chunk) in chunks.into_iter().enumerate() {
            let is_final = i == chunks_len - 1;

            let message_chunk = MessageChunk {
                conversation_id,
                message_id,
                content: chunk,
                is_final,
            };

            let event_data = serde_json::to_string(&message_chunk).unwrap_or_else(|_| {
                json!({
                    "error": "Failed to serialize message chunk"
                })
                .to_string()
            });

            let formatted_event = format!("data: {}\n\n", event_data);
            if sender.send(formatted_event).await.is_err() {
                break; // Client disconnected
            }

            // Simulate streaming delay
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}
