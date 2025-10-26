//! Routes for Copilot-compatible endpoints.

use crate::{
    app_state::AppState,
    handlers::copilot::{get_models, post_chat_completions},
};
use axum::{Router, routing::get, routing::post};
use std::sync::Arc;

/// Configures the Copilot API routes.
///
/// # Returns
/// A [`Router`](axum::Router) with the Copilot API routes.
pub fn create_router_copilot() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/models", get(get_models))
        .route("/v1/chat/completions", post(post_chat_completions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::Extension, http::StatusCode};
    use axum_test::TestServer;
    use chrono::Utc;
    use futures::stream;
    use serde_json::json;
    use shared::{
        config::server::{Config, Profile},
        llms::errors::LLMError,
        llms::types::{FinishReason, LLMConfig, LLMRequest, StreamingResponse, TokenUsage},
    };
    use std::{collections::HashMap, sync::Arc};
    use uuid::Uuid;

    use crate::{
        handlers::streaming::{SharedStreamHub, StreamHub},
        middleware::request_context::RequestContext,
        services::assistant_service::{
            AssistantError, AssistantRuntime, AssistantStreamingSession,
        },
    };

    struct StubAssistant {
        chunks: Vec<StreamingResponse>,
        config: LLMConfig,
    }

    impl StubAssistant {
        fn new(chunks: Vec<StreamingResponse>) -> Self {
            Self {
                chunks,
                config: LLMConfig {
                    model_path: "stub.gguf".into(),
                    max_tokens: Some(64),
                    temperature: Some(0.5),
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
                    .map(|chunk| Ok::<StreamingResponse, LLMError>(chunk)),
            );

            Ok(AssistantStreamingSession::from_stream(Box::pin(stream), 4))
        }

        fn persist_stream_chunks(&self) -> bool {
            false
        }

        fn default_model_name(&self) -> &str {
            "stub-model"
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

    fn test_server() -> TestServer {
        let assistant: Arc<dyn AssistantRuntime> = Arc::new(StubAssistant::new(stub_chunks()));
        let config = Arc::new(Config::default_for_profile(Profile::Test));
        let hub: SharedStreamHub = Arc::new(StreamHub::new(16, None, None));
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

        TestServer::new(app).expect("router server")
    }

    #[tokio::test]
    async fn test_create_router_copilot() {
        let server = test_server();

        let response = server.get("/v1/models").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let response = server
            .post("/v1/chat/completions")
            .json(&json!({
                "model": "stub-model",
                "messages": [
                    { "role": "user", "content": "Hello!" }
                ]
            }))
            .await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }
}
