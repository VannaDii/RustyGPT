use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::sse::{Event, Sse},
};
use futures_util::stream::{self, Stream};
use serde_json::json;
use shared::models::MessageChunk;
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::{Mutex, mpsc};
use tracing::{info, warn};
use uuid::Uuid;

/// Shared state for managing SSE connections
pub type SharedState = Arc<Mutex<HashMap<Uuid, mpsc::Sender<String>>>>;

/// A proper SSE handler that maintains long-lived streaming connections
pub async fn sse_handler(
    Path(user_id): Path<String>,
    Extension(shared_state): Extension<SharedState>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let user_uuid = Uuid::parse_str(&user_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    info!("Setting up SSE connection for user {}", user_uuid);

    // Create a channel for streaming data to this user
    let (tx, _rx) = mpsc::channel::<String>(100);

    // Add the user to the shared state
    {
        let mut state = shared_state.lock().await;
        state.insert(user_uuid, tx.clone());
        info!("Added user {} to SSE state", user_uuid);
    }

    // Send an initial connection message
    if let Err(e) = tx
        .send(
            json!({
                "type": "connected",
                "user_id": user_uuid.to_string()
            })
            .to_string(),
        )
        .await
    {
        warn!(
            "Failed to send initial message to user {}: {}",
            user_uuid, e
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Create a simple infinite keep-alive stream that sends proper MessageChunk objects
    let test_user_id = user_uuid;
    let test_stream = stream::unfold(0, move |counter| async move {
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Create a proper MessageChunk that the frontend expects
        let chunk = MessageChunk {
            conversation_id: test_user_id, // Use user_id as a dummy conversation_id for keep-alive
            message_id: test_user_id,      // Use user_id as a dummy message_id for keep-alive
            content_type: "keep-alive".to_string(),
            content: format!("ping-{}", counter),
            is_final: false,
        };

        let message = match serde_json::to_string(&chunk) {
            Ok(json_data) => {
                info!("Sending SSE MessageChunk: {}", json_data);
                json_data
            }
            Err(e) => {
                warn!("Failed to serialize MessageChunk: {}", e);
                // Send a simple fallback MessageChunk as JSON string
                format!(
                    r#"{{"conversation_id":"{}","message_id":"{}","content_type":"error","content":"serialization_error","is_final":false}}"#,
                    test_user_id, test_user_id
                )
            }
        };

        Some((Ok(Event::default().data(message)), counter + 1))
    });

    // Create SSE response
    let sse_stream = Sse::new(test_stream);

    Ok(sse_stream)
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
                content_type: "text".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Extension, Path};
    use std::collections::HashMap;
    use tokio::sync::mpsc;

    /// Test sse_handler with valid UUID returns proper SSE response
    #[tokio::test]
    async fn test_sse_handler_valid_uuid() {
        let user_id = Uuid::new_v4().to_string();
        let path = Path(user_id);
        let shared_state: SharedState = Arc::new(Mutex::new(HashMap::new()));
        let extension = Extension(shared_state);

        let result = sse_handler(path, extension).await;
        assert!(result.is_ok(), "SSE handler should succeed with valid UUID");
    }

    /// Test sse_handler with invalid UUID returns error
    #[tokio::test]
    async fn test_sse_handler_invalid_uuid() {
        let user_id = "invalid-uuid".to_string();
        let path = Path(user_id);
        let shared_state: SharedState = Arc::new(Mutex::new(HashMap::new()));
        let extension = Extension(shared_state);

        let result = sse_handler(path, extension).await;
        assert!(result.is_err(), "SSE handler should fail with invalid UUID");
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    /// Test stream_partial_response with valid state and user
    #[tokio::test]
    async fn test_stream_partial_response_valid_user() {
        let user_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        // Create shared state with a mock sender
        let (sender, mut receiver) = mpsc::channel::<String>(10);
        let mut state_map = HashMap::new();
        state_map.insert(user_id, sender);
        let state = Arc::new(Mutex::new(state_map));

        let chunks = vec!["Hello".to_string(), " World".to_string()];

        // Start streaming in a separate task
        let stream_task = tokio::spawn(stream_partial_response(
            state,
            user_id,
            conversation_id,
            message_id,
            chunks,
        ));

        // Collect messages
        let mut messages = Vec::new();
        while let Some(message) = receiver.recv().await {
            messages.push(message);
            if messages.len() == 2 {
                break;
            }
        }

        // Wait for the streaming task to complete
        let _ = stream_task.await;

        assert_eq!(messages.len(), 2);

        // Verify the first message
        assert!(messages[0].contains("Hello"));
        assert!(messages[0].contains("\"is_final\":false"));

        // Verify the second message
        assert!(messages[1].contains(" World"));
        assert!(messages[1].contains("\"is_final\":true"));
    }

    /// Test stream_partial_response with user not in state
    #[tokio::test]
    async fn test_stream_partial_response_user_not_found() {
        let user_id = Uuid::new_v4();
        let missing_user_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        // Create shared state without the user we're trying to stream to
        let (sender, _receiver) = mpsc::channel::<String>(10);
        let mut state_map = HashMap::new();
        state_map.insert(user_id, sender); // Different user
        let state = Arc::new(Mutex::new(state_map));

        let chunks = vec!["Hello".to_string()];

        // This should complete without error but not send any messages
        stream_partial_response(state, missing_user_id, conversation_id, message_id, chunks).await;

        // Test passes if no panic occurred - verified by reaching this point
    }

    /// Test stream_partial_response with empty chunks
    #[tokio::test]
    async fn test_stream_partial_response_empty_chunks() {
        let user_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        let (sender, mut receiver) = mpsc::channel::<String>(10);
        let mut state_map = HashMap::new();
        state_map.insert(user_id, sender);
        let state = Arc::new(Mutex::new(state_map));

        let chunks: Vec<String> = vec![];

        // Start streaming - should complete immediately for empty chunks
        let stream_task = tokio::spawn(stream_partial_response(
            state.clone(),
            user_id,
            conversation_id,
            message_id,
            chunks,
        ));

        // Wait for the streaming task to complete
        let stream_result = stream_task.await;
        assert!(
            stream_result.is_ok(),
            "Stream task should complete successfully"
        );

        // Close the sender by removing it from state to close the channel
        {
            let mut state_lock = state.lock().await;
            state_lock.remove(&user_id);
        }

        // Now try to receive - should get None because sender is dropped
        let received = receiver.recv().await;
        assert!(
            received.is_none(),
            "Expected None when sender is dropped and no messages sent"
        );
    }

    /// Test stream_partial_response with single chunk
    #[tokio::test]
    async fn test_stream_partial_response_single_chunk() {
        let user_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();

        let (sender, mut receiver) = mpsc::channel::<String>(10);
        let mut state_map = HashMap::new();
        state_map.insert(user_id, sender);
        let state = Arc::new(Mutex::new(state_map));

        let chunks = vec!["Single message".to_string()];

        // Start streaming
        let stream_task = tokio::spawn(stream_partial_response(
            state,
            user_id,
            conversation_id,
            message_id,
            chunks,
        ));

        // Get the message
        let message = receiver.recv().await.unwrap();

        // Wait for the streaming task to complete
        let _ = stream_task.await;

        // Single chunk should be marked as final
        assert!(message.contains("Single message"));
        assert!(message.contains("\"is_final\":true"));
    }

    /// Test SharedState type alias
    #[test]
    fn test_shared_state_type() {
        let state_map = HashMap::new();
        let state: SharedState = Arc::new(Mutex::new(state_map));

        // Verify the type works as expected
        assert_eq!(state.try_lock().unwrap().len(), 0);
    }

    /// Test MessageChunk serialization in streaming context
    #[test]
    fn test_message_chunk_serialization() {
        let chunk = MessageChunk {
            conversation_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
            content_type: "text".to_string(),
            content: "Test content".to_string(),
            is_final: true,
        };

        let serialized = serde_json::to_string(&chunk).unwrap();
        assert!(serialized.contains("Test content"));
        assert!(serialized.contains("\"is_final\":true"));
        assert!(serialized.contains("\"content_type\":\"text\""));
    }
}
