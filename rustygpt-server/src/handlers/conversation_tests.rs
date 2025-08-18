use crate::handlers::{conversation::*, streaming::SharedState};
use axum::{
    extract::{Extension, Json, Path},
    http::StatusCode,
};
use shared::models::conversation::SendMessageRequest;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_conversation_returns_mock_data() {
        let response = get_conversation().await;

        assert_eq!(response.len(), 1);
        assert_eq!(response[0].title, "Sample Chat");
        assert_eq!(response[0].participant_ids.len(), 1);
        assert_eq!(response[0].messages.len(), 1);
        assert_eq!(response[0].messages[0].content, "Hello, world!");
    }

    #[tokio::test]
    async fn test_get_conversation_response_structure() {
        let response = get_conversation().await;
        let conversation = &response[0];

        // Verify all required fields are present and valid
        assert!(!conversation.id.to_string().is_empty());
        assert!(!conversation.title.is_empty());
        assert!(!conversation.participant_ids.is_empty());
        assert!(!conversation.messages.is_empty());

        let message = &conversation.messages[0];
        assert!(!message.id.to_string().is_empty());
        assert!(!message.sender_id.to_string().is_empty());
        assert!(!message.conversation_id.to_string().is_empty());
        assert!(!message.content.is_empty());
    }

    #[tokio::test]
    async fn test_send_message_with_valid_data() {
        let shared_state: SharedState = Arc::new(Mutex::new(HashMap::new()));
        let conversation_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4();

        let request = SendMessageRequest {
            user_id: user_id.to_string(),
            content: "Test message content".to_string(),
        };

        let response = send_message(
            Extension(shared_state),
            Path(conversation_id),
            Json(request),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_send_message_with_invalid_conversation_id() {
        let shared_state: SharedState = Arc::new(Mutex::new(HashMap::new()));
        let invalid_conversation_id = "invalid-uuid".to_string();
        let user_id = Uuid::new_v4();

        let request = SendMessageRequest {
            user_id: user_id.to_string(),
            content: "Test message".to_string(),
        };

        let response = send_message(
            Extension(shared_state),
            Path(invalid_conversation_id),
            Json(request),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_send_message_with_invalid_user_id() {
        let shared_state: SharedState = Arc::new(Mutex::new(HashMap::new()));
        let conversation_id = Uuid::new_v4().to_string();

        let request = SendMessageRequest {
            user_id: "invalid-user-id".to_string(),
            content: "Test message".to_string(),
        };

        let response = send_message(
            Extension(shared_state),
            Path(conversation_id),
            Json(request),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_send_message_with_empty_content() {
        let shared_state: SharedState = Arc::new(Mutex::new(HashMap::new()));
        let conversation_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4();

        let request = SendMessageRequest {
            user_id: user_id.to_string(),
            content: "".to_string(),
        };

        let response = send_message(
            Extension(shared_state),
            Path(conversation_id),
            Json(request),
        )
        .await;

        // Should still accept empty content (may be valid for some use cases)
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_conversation_routes_creation() {
        let router = conversation_routes();

        // Test that the router was created successfully
        assert!(!format!("{:?}", router).is_empty());
    }

    #[tokio::test]
    async fn test_send_message_response_content_type() {
        let shared_state: SharedState = Arc::new(Mutex::new(HashMap::new()));
        let conversation_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4();

        let request = SendMessageRequest {
            user_id: user_id.to_string(),
            content: "Content type test".to_string(),
        };

        let response = send_message(
            Extension(shared_state),
            Path(conversation_id),
            Json(request),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        // Check that content-type header is set
        let content_type = response.headers().get("content-type");
        assert!(content_type.is_some());
        assert_eq!(content_type.unwrap(), "application/json");
    }

    #[tokio::test]
    async fn test_get_conversation_generates_unique_ids() {
        let response1 = get_conversation().await;
        let response2 = get_conversation().await;

        // Each call should generate unique IDs
        assert_ne!(response1[0].id, response2[0].id);
        assert_ne!(response1[0].messages[0].id, response2[0].messages[0].id);
        assert_ne!(
            response1[0].participant_ids[0],
            response2[0].participant_ids[0]
        );
    }
}
