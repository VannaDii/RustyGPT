use std::sync::Arc;

use axum::{
    Router,
    extract::{Extension, Json, Path},
    http::{StatusCode, header},
    response::Response,
    routing::{get, post},
};
use shared::models::{
    Conversation, CreateConversationRequest, CreateMessageRequest, ErrorResponse, Message,
    MessageType,
    conversation::{SendMessageRequest, SendMessageResponse},
    user::AuthenticateRequest,
};
use tokio::spawn;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    handlers::streaming::{SharedState, stream_partial_response},
    services::{
        MessageService, conversation_service::ConversationService, user_service::UserService,
    },
};

/// Simple password verification function
/// In production, use a proper password hashing library like argon2 or bcrypt
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    // For now, doing simple string comparison
    // TODO: Replace with proper password hashing verification
    password == stored_hash
}

#[utoipa::path(
    get,
    path = "/conversation",
    responses(
        (status = 200, description = "Conversations retrieved", body = Vec<Conversation>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Chat"
)]
pub async fn get_conversation(
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<Json<Vec<Conversation>>, (StatusCode, Json<ErrorResponse>)> {
    // For now, use a hardcoded test user ID since we don't have authentication
    // In a real app, you'd extract the user ID from authentication middleware
    let test_user_id =
        Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("Valid hardcoded UUID");

    if let Some(pool) = app_state.pool.as_ref() {
        let conversation_service = ConversationService::new(pool.clone());

        match conversation_service
            .list_user_conversations(test_user_id)
            .await
        {
            Ok(conversations) => Ok(Json(conversations)),
            Err(e) => {
                eprintln!("Failed to fetch conversations: {}", e);
                Ok(Json(vec![]))
            }
        }
    } else {
        // No database connection, return empty list
        Ok(Json(vec![]))
    }
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
    Extension(app_state): Extension<Arc<AppState>>,
    Extension(stream_state): Extension<SharedState>,
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

    // If we have a database connection, save the user message
    if let Some(pool) = app_state.pool.as_ref() {
        let message_service = MessageService::new(pool.clone());

        let create_request = CreateMessageRequest {
            conversation_id,
            sender_id: user_id,
            content: request.content.clone(),
            message_type: MessageType::User,
        };

        if let Err(e) = message_service.create_message(create_request).await {
            tracing::error!("Failed to save user message: {}", e);
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::from("Failed to save message"))
                .unwrap();
        }
    }

    // Create a new message ID for the assistant response
    let message_id = Uuid::new_v4();

    // Simulate generating a response in chunks
    // In a real application, this would come from an AI model
    // Use the content from the request to personalize the response
    let content = request.content.clone();
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
    let stream_state_clone = stream_state.clone();
    let app_state_clone = app_state.clone();
    spawn(async move {
        stream_partial_response(
            stream_state_clone,
            user_id,
            conversation_id,
            message_id,
            response_chunks,
        )
        .await;

        // After streaming, save the complete assistant message to the database
        if let Some(pool) = app_state_clone.pool.as_ref() {
            let message_service = MessageService::new(pool.clone());

            let assistant_message = CreateMessageRequest {
                conversation_id,
                sender_id: user_id, // Assistant messages still need a sender_id for the schema
                content: format!(
                    "I'm thinking about your question: '{}'. Here's my response.",
                    content
                ),
                message_type: MessageType::Assistant,
            };

            if let Err(e) = message_service.create_message(assistant_message).await {
                tracing::error!("Failed to save assistant message: {}", e);
            }
        }
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

/// Create a new conversation
#[utoipa::path(
    post,
    path = "/conversation",
    request_body = CreateConversationRequest,
    responses(
        (status = 201, description = "Conversation created", body = Conversation),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Chat"
)]
pub async fn create_conversation(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<CreateConversationRequest>,
) -> Result<Json<Conversation>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pool) = app_state.pool.as_ref() {
        let conversation_service = ConversationService::new(pool.clone());

        match conversation_service.create_conversation(request).await {
            Ok(conversation_id) => {
                // Return a minimal conversation representation
                // In a real app, you'd probably fetch the full conversation details
                let conversation = Conversation {
                    id: conversation_id,
                    title: "New Conversation".to_string(),
                    participant_ids: vec![],
                    messages: vec![],
                    last_updated: shared::models::timestamp::Timestamp(chrono::Utc::now()),
                };
                Ok(Json(conversation))
            }
            Err(e) => {
                eprintln!("Failed to create conversation: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "Failed to create conversation".to_string(),
                        details: None,
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "Database not available".to_string(),
                details: None,
            }),
        ))
    }
}

/// Get a specific conversation by ID
#[utoipa::path(
    get,
    path = "/conversation/{conversation_id}",
    responses(
        (status = 200, description = "Conversation retrieved", body = Conversation),
        (status = 400, description = "Invalid conversation ID", body = ErrorResponse),
        (status = 404, description = "Conversation not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Chat"
)]
pub async fn get_conversation_by_id(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Conversation>, (StatusCode, Json<ErrorResponse>)> {
    // Parse conversation ID
    let conversation_id = match Uuid::parse_str(&conversation_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Invalid conversation ID".to_string(),
                    details: None,
                }),
            ));
        }
    };

    // Use hardcoded test user ID for now (same as in other handlers)
    let test_user_id =
        Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("Valid hardcoded UUID");

    if let Some(pool) = app_state.pool.as_ref() {
        let conversation_service = ConversationService::new(pool.clone());

        match conversation_service
            .get_conversation(conversation_id, test_user_id)
            .await
        {
            Ok(conversation) => Ok(Json(conversation)),
            Err(e) => {
                eprintln!("Failed to fetch conversation: {}", e);
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        message: "Conversation not found".to_string(),
                        details: None,
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "Database not available".to_string(),
                details: None,
            }),
        ))
    }
}

/// Add a participant to a conversation
#[utoipa::path(
    post,
    path = "/conversation/{conversation_id}/participants/{user_id}",
    responses(
        (status = 200, description = "Participant added successfully"),
        (status = 400, description = "Invalid conversation or user ID", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Chat"
)]
pub async fn add_participant(
    Extension(app_state): Extension<Arc<AppState>>,
    Path((conversation_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Parse conversation ID
    let conversation_id = match Uuid::parse_str(&conversation_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Invalid conversation ID".to_string(),
                    details: None,
                }),
            ));
        }
    };

    // Parse user ID
    let user_id = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Invalid user ID".to_string(),
                    details: None,
                }),
            ));
        }
    };

    if let Some(pool) = app_state.pool.as_ref() {
        let conversation_service = ConversationService::new(pool.clone());

        match conversation_service
            .add_participant(conversation_id, user_id)
            .await
        {
            Ok(()) => Ok(StatusCode::OK),
            Err(e) => {
                eprintln!("Failed to add participant: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "Failed to add participant".to_string(),
                        details: None,
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "Database not available".to_string(),
                details: None,
            }),
        ))
    }
}

/// Get messages for a specific conversation
#[utoipa::path(
    get,
    path = "/conversation/{conversation_id}/messages",
    responses(
        (status = 200, description = "Messages retrieved", body = Vec<Message>),
        (status = 400, description = "Invalid conversation ID", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Chat"
)]
pub async fn get_conversation_messages(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Vec<shared::models::Message>>, (StatusCode, Json<ErrorResponse>)> {
    // Parse conversation ID
    let conversation_id = match Uuid::parse_str(&conversation_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Invalid conversation ID".to_string(),
                    details: None,
                }),
            ));
        }
    };

    // Use hardcoded test user ID for now (same as in get_conversation)
    let test_user_id =
        Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("Valid hardcoded UUID");

    if let Some(pool) = app_state.pool.as_ref() {
        let message_service = MessageService::new(pool.clone());

        match message_service
            .get_conversation_messages(conversation_id, test_user_id)
            .await
        {
            Ok(messages) => Ok(Json(messages)),
            Err(e) => {
                eprintln!("Failed to fetch messages: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "Failed to fetch messages".to_string(),
                        details: None,
                    }),
                ))
            }
        }
    } else {
        // No database connection, return empty list
        Ok(Json(vec![]))
    }
}

/// Get a specific message by ID
#[utoipa::path(
    get,
    path = "/message/{message_id}",
    responses(
        (status = 200, description = "Message retrieved", body = Message),
        (status = 400, description = "Invalid message ID", body = ErrorResponse),
        (status = 404, description = "Message not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Chat"
)]
pub async fn get_message(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(message_id): Path<String>,
) -> Result<Json<shared::models::Message>, (StatusCode, Json<ErrorResponse>)> {
    // Parse message ID
    let message_id = match Uuid::parse_str(&message_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Invalid message ID".to_string(),
                    details: None,
                }),
            ));
        }
    };

    // Use hardcoded test user ID for now (same as in get_conversation)
    let test_user_id =
        Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("Valid hardcoded UUID");

    if let Some(pool) = app_state.pool.as_ref() {
        let message_service = MessageService::new(pool.clone());

        match message_service.get_message(message_id, test_user_id).await {
            Ok(message) => Ok(Json(message)),
            Err(e) => {
                eprintln!("Failed to fetch message: {}", e);
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        message: "Message not found".to_string(),
                        details: None,
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "Database not available".to_string(),
                details: None,
            }),
        ))
    }
}

/// Register a new user with email and password (traditional auth)
#[utoipa::path(
    post,
    path = "/user/register",
    request_body = serde_json::Value,
    responses(
        (status = 201, description = "User registered successfully", body = String),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "User"
)]
pub async fn register_user(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let username = payload["username"].as_str().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "Username is required".to_string(),
                details: None,
            }),
        )
    })?;

    let email = payload["email"].as_str().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "Email is required".to_string(),
                details: None,
            }),
        )
    })?;

    let password_hash = payload["password_hash"].as_str().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "Password hash is required".to_string(),
                details: None,
            }),
        )
    })?;

    if let Some(pool) = app_state.pool.as_ref() {
        let user_service = UserService::new(pool.clone());

        match user_service
            .register_user(username, email, password_hash)
            .await
        {
            Ok(user_id) => Ok(Json(user_id.to_string())),
            Err(e) => {
                eprintln!("Failed to register user: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "Failed to register user".to_string(),
                        details: None,
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "Database not available".to_string(),
                details: None,
            }),
        ))
    }
}

/// Authenticate a user by username or email (unified auth)
#[utoipa::path(
    post,
    path = "/authenticate",
    request_body = AuthenticateRequest,
    responses(
        (status = 200, description = "User authenticated", body = String),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Authentication failed", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "User"
)]
pub async fn authenticate_user_unified(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<AuthenticateRequest>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pool) = app_state.pool.as_ref() {
        let user_service = UserService::new(pool.clone());

        match user_service
            .authenticate_user_unified(&request.username_or_email)
            .await
        {
            Ok(Some((user_id, _username, _email, password_hash))) => {
                // Verify the password hash against request.password
                // For now, we'll do a simple comparison - in production, use proper password hashing
                if verify_password(&request.password, &password_hash) {
                    Ok(Json(user_id.to_string()))
                } else {
                    Err((
                        StatusCode::UNAUTHORIZED,
                        Json(ErrorResponse {
                            message: "Authentication failed".to_string(),
                            details: None,
                        }),
                    ))
                }
            }
            Ok(None) => Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    message: "Authentication failed".to_string(),
                    details: None,
                }),
            )),
            Err(e) => {
                eprintln!("Failed to authenticate user: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "Failed to authenticate user".to_string(),
                        details: None,
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "Database not available".to_string(),
                details: None,
            }),
        ))
    }
}

/// Get user details by ID
#[utoipa::path(
    get,
    path = "/user/{user_id}",
    responses(
        (status = 200, description = "User details retrieved", body = shared::models::User),
        (status = 400, description = "Invalid user ID", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "User"
)]
pub async fn get_user_by_id(
    Extension(app_state): Extension<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<shared::models::User>, (StatusCode, Json<ErrorResponse>)> {
    // Parse user ID
    let user_id = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    message: "Invalid user ID".to_string(),
                    details: None,
                }),
            ));
        }
    };

    if let Some(pool) = app_state.pool.as_ref() {
        let user_service = UserService::new(pool.clone());

        match user_service.get_user_by_id(user_id).await {
            Ok(Some(user)) => Ok(Json(user)),
            Ok(None) => Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    message: "User not found".to_string(),
                    details: None,
                }),
            )),
            Err(e) => {
                eprintln!("Failed to get user: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        message: "Failed to get user".to_string(),
                        details: None,
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "Database not available".to_string(),
                details: None,
            }),
        ))
    }
}

// Function to register the conversation routes
pub fn conversation_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/conversation",
            get(get_conversation).post(create_conversation),
        )
        .route(
            "/conversation/{conversation_id}",
            get(get_conversation_by_id),
        )
        .route(
            "/conversation/{conversation_id}/messages",
            post(send_message).get(get_conversation_messages),
        )
        .route(
            "/conversation/{conversation_id}/participants/{user_id}",
            post(add_participant),
        )
        .route("/message/{message_id}", get(get_message))
        .route("/user/register", post(register_user))
        .route("/authenticate", post(authenticate_user_unified))
        .route("/user/{user_id}", get(get_user_by_id))
}
