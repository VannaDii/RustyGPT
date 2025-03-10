use axum::{
    extract::Path,
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
