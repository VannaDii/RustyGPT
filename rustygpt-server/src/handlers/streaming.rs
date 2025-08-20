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
    use axum::extract::Path;
    use tokio::sync::mpsc;

    /// Test simple_sse_handler returns proper SSE response
    #[tokio::test]
    async fn test_simple_sse_handler() {
        let user_id = "test_user_123".to_string();
        let path = Path(user_id.clone());

        let response = simple_sse_handler(path).await;

        assert_eq!(response.status(), StatusCode::OK);

        // Check headers
        let headers = response.headers();
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "text/event-stream"
        );
        assert_eq!(headers.get(header::CACHE_CONTROL).unwrap(), "no-cache");
        assert_eq!(headers.get(header::CONNECTION).unwrap(), "keep-alive");
    }

    /// Test simple_sse_handler with empty user ID
    #[tokio::test]
    async fn test_simple_sse_handler_empty_user_id() {
        let user_id = "".to_string();
        let path = Path(user_id);

        let response = simple_sse_handler(path).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/event-stream"
        );
    }

    /// Test simple_sse_handler with special characters in user ID
    #[tokio::test]
    async fn test_simple_sse_handler_special_chars() {
        let user_id = "user@test.com".to_string();
        let path = Path(user_id);

        let response = simple_sse_handler(path).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/event-stream"
        );
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
