use crate::{
    app_state::AppState,
    handlers::{
        conversation::*,
        streaming::{SharedState, SseCoordinator},
    },
};
use axum::{
    extract::{Extension, Json, Path},
    http::StatusCode,
};
use shared::models::conversation::SendMessageRequest;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_conversation_returns_empty_list() {
        let app_state = Arc::new(AppState::default());
        let response = get_conversation(Extension(app_state)).await;

        // Should be ok and return empty list since no authentication/user context
        assert!(response.is_ok());
        let conversations = response.unwrap().0;
        assert_eq!(conversations.len(), 0);
    }

    #[tokio::test]
    async fn test_get_conversation_response_structure() {
        let app_state = Arc::new(AppState::default());
        let response = get_conversation(Extension(app_state)).await;

        // Should succeed and return empty conversations
        assert!(response.is_ok());
        let conversations = response.unwrap().0;
        assert_eq!(conversations.len(), 0);
    }

    #[tokio::test]
    async fn test_send_message_with_valid_data() {
        let app_state = Arc::new(AppState::default());
        let shared_state: SharedState = Arc::new(SseCoordinator::new(16, "evt_".into()));
        let conversation_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4();

        let request = SendMessageRequest {
            user_id: user_id.to_string(),
            content: "Test message content".to_string(),
        };

        let response = send_message(
            Extension(app_state),
            Extension(shared_state),
            Path(conversation_id),
            Json(request),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_send_message_with_invalid_conversation_id() {
        let app_state = Arc::new(AppState::default());
        let shared_state: SharedState = Arc::new(SseCoordinator::new(16, "evt_".into()));
        let invalid_conversation_id = "invalid-uuid".to_string();
        let user_id = Uuid::new_v4();

        let request = SendMessageRequest {
            user_id: user_id.to_string(),
            content: "Test message".to_string(),
        };

        let response = send_message(
            Extension(app_state),
            Extension(shared_state),
            Path(invalid_conversation_id),
            Json(request),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_send_message_with_invalid_user_id() {
        let app_state = Arc::new(AppState::default());
        let shared_state: SharedState = Arc::new(SseCoordinator::new(16, "evt_".into()));
        let conversation_id = Uuid::new_v4().to_string();

        let request = SendMessageRequest {
            user_id: "invalid-user-id".to_string(),
            content: "Test message".to_string(),
        };

        let response = send_message(
            Extension(app_state),
            Extension(shared_state),
            Path(conversation_id),
            Json(request),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_send_message_with_empty_content() {
        let app_state = Arc::new(AppState::default());
        let shared_state: SharedState = Arc::new(SseCoordinator::new(16, "evt_".into()));
        let conversation_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4();

        let request = SendMessageRequest {
            user_id: user_id.to_string(),
            content: "".to_string(),
        };

        let response = send_message(
            Extension(app_state),
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
        let app_state = Arc::new(AppState::default());
        let shared_state: SharedState = Arc::new(SseCoordinator::new(16, "evt_".into()));
        let conversation_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4();

        let request = SendMessageRequest {
            user_id: user_id.to_string(),
            content: "Content type test".to_string(),
        };

        let response = send_message(
            Extension(app_state),
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
    async fn test_get_conversation_with_database() {
        // Test that get_conversation works with app state containing no database
        let app_state = Arc::new(AppState::default());
        let response1 = get_conversation(Extension(app_state.clone())).await;
        let response2 = get_conversation(Extension(app_state)).await;

        // Both should succeed and return empty lists
        assert!(response1.is_ok());
        assert!(response2.is_ok());

        let conversations1 = response1.unwrap().0;
        let conversations2 = response2.unwrap().0;

        assert_eq!(conversations1.len(), 0);
        assert_eq!(conversations2.len(), 0);
    }

    #[test]
    fn test_verify_password_matching() {
        // Test password verification with matching password
        let password = "test_password";
        let stored_hash = "test_password"; // Simple string comparison for now

        assert!(super::verify_password(password, stored_hash));
    }

    #[test]
    fn test_verify_password_not_matching() {
        // Test password verification with non-matching password
        let password = "test_password";
        let stored_hash = "different_password";

        assert!(!super::verify_password(password, stored_hash));
    }

    #[test]
    fn test_verify_password_empty_strings() {
        // Test password verification with empty strings
        let password = "";
        let stored_hash = "";

        assert!(super::verify_password(password, stored_hash));
    }

    #[test]
    fn test_verify_password_case_sensitive() {
        // Test password verification is case sensitive
        let password = "TestPassword";
        let stored_hash = "testpassword";

        assert!(!super::verify_password(password, stored_hash));
    }
}
