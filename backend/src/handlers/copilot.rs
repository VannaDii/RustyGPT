use axum::Json;
use shared::models::{
    ChatChoice, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Model, ModelsResponse,
};

/// Handler for `/v1/models`.
///
/// # Returns
/// A list of available models.
pub async fn get_models() -> Json<ModelsResponse> {
    let models = vec![
        Model {
            id: "gpt-4".to_string(),
            name: "GPT-4".to_string(),
            model_type: "chat".to_string(),
        },
        Model {
            id: "gpt-3.5".to_string(),
            name: "GPT-3.5".to_string(),
            model_type: "chat".to_string(),
        },
    ];
    Json(ModelsResponse { models })
}

/// Handler for `/v1/chat/completions`.
///
/// # Arguments
/// * `payload` - The chat completion request payload.
///
/// # Returns
/// A chat completion response with generated choices.
pub async fn post_chat_completions(
    Json(payload): Json<ChatCompletionRequest>,
) -> Json<ChatCompletionResponse> {
    let choices = payload
        .messages
        .iter()
        .enumerate()
        .map(|(index, message)| ChatChoice {
            index,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: format!("Echo: {}", message.content),
            },
        })
        .collect();

    Json(ChatCompletionResponse { choices })
}
