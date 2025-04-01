use std::sync::Arc;

use axum::{
    Router,
    extract::{Extension, Json, Path},
    http::{StatusCode, header},
    response::Response,
    routing::{get, post},
};
use chrono::Utc;
use shared::models::{
    Conversation, ErrorResponse, Message, Timestamp,
    conversation::{SendMessageRequest, SendMessageResponse},
};
use tokio::spawn;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    handlers::streaming::{SharedState, stream_partial_response},
};

#[utoipa::path(
    get,
    path = "/conversation",
    responses(
        (status = 200, description = "Conversations retrieved", body = Vec<Conversation>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Chat"
)]
pub async fn get_conversation() -> Json<Vec<Conversation>> {
    let mock_conversations = vec![Conversation {
        id: Uuid::new_v4(),
        title: "Sample Chat".into(),
        participant_ids: vec![Uuid::new_v4()],
        messages: vec![Message {
            id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            content: "Hello, world!".into(),
            timestamp: Timestamp(Utc::now()),
        }],
        last_updated: Timestamp(Utc::now()),
    }];
    Json(mock_conversations)
}

/// Send a message to a conversation with streaming response
#[utoipa::path(
    post,
    path = "/conversation/{conversation_id}/messages",
    responses(
        (status = 200, description = "Message received", body = SendMessageResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Chat"
)]
pub async fn send_message(
    Extension(state): Extension<SharedState>,
    Path(conversation_id): Path<String>,
    Json(request): Json<SendMessageRequest>,
) -> Response {
    // Parse UUIDs
    let conversation_id = match Uuid::parse_str(&conversation_id) {
        Ok(id) => id,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(axum::body::Body::from("Invalid conversation ID"))
                .unwrap();
        }
    };

    let user_id = match Uuid::parse_str(&request.user_id) {
        Ok(id) => id,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(axum::body::Body::from("Invalid user ID"))
                .unwrap();
        }
    };

    // Create a new message ID
    let message_id = Uuid::new_v4();

    // Simulate generating a response in chunks
    // In a real application, this would come from an AI model
    // Use the content from the request to personalize the response
    let content = &request.content;
    let response_chunks = vec![
        "I'm ".to_string(),
        "thinking ".to_string(),
        "about ".to_string(),
        "your ".to_string(),
        format!("question: '{}'. ", content),
        "Here's ".to_string(),
        "my ".to_string(),
        "response.".to_string(),
    ];

    // Spawn a task to stream the response
    let state_clone = state.clone();
    spawn(async move {
        stream_partial_response(
            state_clone,
            user_id,
            conversation_id,
            message_id,
            response_chunks,
        )
        .await;
    });

    // Return the message ID immediately
    let response_body = serde_json::to_string(&SendMessageResponse {
        message_id: message_id.to_string(),
    })
    .unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(response_body))
        .unwrap()
}

// Function to register the conversation routes
pub fn conversation_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/conversation", get(get_conversation))
        .route(
            "/conversation/{conversation_id}/messages",
            post(send_message),
        )
}
