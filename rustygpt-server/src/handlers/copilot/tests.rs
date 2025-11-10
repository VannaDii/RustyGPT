use super::*;
use axum::{extract::Extension, http::StatusCode};
use axum_test::TestServer;
use chrono::Utc;
use futures::stream;
use shared::{
    config::server::{Config, Profile},
    llms::errors::LLMError,
    llms::types::{FinishReason, LLMConfig},
    models::ChatCompletionResponse,
};
use std::{collections::HashMap, sync::Arc};

use crate::{
    handlers::streaming::StreamHub, middleware::request_context::RequestContext,
    routes::copilot::create_router_copilot, services::assistant_service::AssistantRuntime,
};
use serde_json::json;

struct StubAssistant {
    model: String,
    chunks: Vec<StreamingResponse>,
    config: LLMConfig,
}

impl StubAssistant {
    fn new(model: &str, chunks: Vec<StreamingResponse>) -> Self {
        Self {
            model: model.to_string(),
            chunks,
            config: LLMConfig {
                model_path: "stub.gguf".into(),
                max_tokens: Some(128),
                temperature: Some(0.7),
                top_p: Some(1.0),
                top_k: None,
                repetition_penalty: None,
                n_threads: None,
                n_gpu_layers: None,
                context_size: None,
                batch_size: None,
                additional_params: HashMap::new(),
            },
        }
    }
}

#[async_trait::async_trait]
impl AssistantRuntime for StubAssistant {
    async fn stream_reply(
        &self,
        _request: LLMRequest,
    ) -> Result<AssistantStreamingSession, AssistantError> {
        let stream = stream::iter(
            self.chunks
                .clone()
                .into_iter()
                .map(Ok::<StreamingResponse, LLMError>),
        );

        Ok(AssistantStreamingSession::from_stream(Box::pin(stream), 4))
    }

    fn persist_stream_chunks(&self) -> bool {
        false
    }

    fn default_model_name(&self) -> &str {
        &self.model
    }

    fn default_chat_config(&self) -> Result<LLMConfig, AssistantError> {
        Ok(self.config.clone())
    }
}

fn stub_chunks() -> Vec<StreamingResponse> {
    vec![
        StreamingResponse {
            request_id: Uuid::new_v4(),
            text_delta: "Hello".to_string(),
            is_final: false,
            current_text: Some("Hello".to_string()),
            finish_reason: None,
            usage: TokenUsage::new(4, 1),
            timestamp: Utc::now(),
        },
        StreamingResponse {
            request_id: Uuid::new_v4(),
            text_delta: " world".to_string(),
            is_final: true,
            current_text: Some("Hello world".to_string()),
            finish_reason: Some(FinishReason::EndOfText),
            usage: TokenUsage::new(4, 2),
            timestamp: Utc::now(),
        },
    ]
}

fn test_app(assistant: Arc<dyn AssistantRuntime>) -> TestServer {
    let config = Arc::new(Config::default_for_profile(Profile::Test));
    let hub: SharedStreamHub = Arc::new(StreamHub::new(32, None, None));
    let context = RequestContext {
        request_id: "req".into(),
        session: None,
    };

    let app_state = Arc::new(AppState {
        assistant: Some(assistant),
        ..AppState::default()
    });

    let app = create_router_copilot()
        .layer(Extension(app_state.clone()))
        .layer(Extension(config))
        .layer(Extension(context))
        .layer(Extension(hub))
        .with_state(app_state);

    TestServer::new(app).expect("test server")
}

#[tokio::test]
async fn get_models_uses_configuration() {
    let config = Arc::new(Config::default_for_profile(Profile::Test));
    let response = super::get_models(Extension(config.clone())).await;
    assert!(!response.models.is_empty());
    assert_eq!(response.models[0].object, OBJECT_MODEL);
}

#[tokio::test]
async fn post_chat_completions_returns_final_message() {
    let assistant: Arc<dyn AssistantRuntime> =
        Arc::new(StubAssistant::new("stub-model", stub_chunks()));
    let server = test_app(assistant);

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "stub-model",
            "messages": [
                { "role": "user", "content": "Hello" }
            ]
        }))
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: ChatCompletionResponse = response.json();
    assert_eq!(body.choices.len(), 1);
    assert_eq!(body.choices[0].message.content, "Hello world");
    assert_eq!(body.choices[0].finish_reason.as_deref(), Some("stop"));
    assert_eq!(body.usage.as_ref().unwrap().prompt_tokens, 4);
}

#[tokio::test]
async fn post_chat_completions_streams_sse() {
    let assistant: Arc<dyn AssistantRuntime> =
        Arc::new(StubAssistant::new("stub-model", stub_chunks()));
    let server = test_app(assistant);

    let response = server
        .post("/v1/chat/completions")
        .json(&json!({
            "model": "stub-model",
            "stream": true,
            "messages": [
                { "role": "user", "content": "Hello" }
            ]
        }))
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("\"content\":\"Hello\""));
    assert!(body.contains("[DONE]"));
}
