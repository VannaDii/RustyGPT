use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

use shared::{
    config::server::Config,
    llms::{
        llama_cpp::{LlamaCppModel, LlamaCppProvider},
        traits::{LLMModel, LLMProvider, StreamingResponseStream},
        types::{LLMConfig, LLMRequest},
    },
};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum AssistantError {
    #[error("llm configuration error: {0}")]
    Config(String),
    #[error("llm provider error: {0}")]
    Provider(String),
    #[error("llm execution error: {0}")]
    Inference(String),
}

pub struct AssistantStreamingSession {
    pub stream: StreamingResponseStream,
    pub prompt_tokens: i64,
    _model_guard: Option<Arc<LlamaCppModel>>,
    _metrics_guard: Option<SessionMetricsGuard>,
}

impl AssistantStreamingSession {
    #[cfg(test)]
    pub(crate) fn from_stream(stream: StreamingResponseStream, prompt_tokens: i64) -> Self {
        Self {
            stream,
            prompt_tokens,
            _model_guard: None,
            _metrics_guard: None,
        }
    }

    fn with_guards(
        stream: StreamingResponseStream,
        prompt_tokens: i64,
        model_guard: Arc<LlamaCppModel>,
        metrics_guard: SessionMetricsGuard,
    ) -> Self {
        Self {
            stream,
            prompt_tokens,
            _model_guard: Some(model_guard),
            _metrics_guard: Some(metrics_guard),
        }
    }
}

#[async_trait]
pub trait AssistantRuntime: Send + Sync {
    async fn stream_reply(
        &self,
        request: LLMRequest,
    ) -> Result<AssistantStreamingSession, AssistantError>;

    fn persist_stream_chunks(&self) -> bool;

    fn default_model_name(&self) -> &str;

    fn default_chat_config(&self) -> Result<LLMConfig, AssistantError>;
}

#[derive(Clone)]
pub struct AssistantService {
    config: Arc<Config>,
    models: Arc<RwLock<HashMap<String, Arc<LlamaCppModel>>>>,
    metrics: Arc<AssistantMetrics>,
}

impl AssistantService {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            models: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(AssistantMetrics::default()),
        }
    }

    pub async fn stream_reply(
        &self,
        request: LLMRequest,
    ) -> Result<AssistantStreamingSession, AssistantError> {
        let (model_name, provider_type, llm_config) = self.resolve_model_choice(&request)?;
        let cache_key = format!("{provider_type}::{model_name}");
        let model = self
            .ensure_model(&cache_key, &provider_type, llm_config)
            .await?;

        let prompt_tokens = model
            .count_tokens(&request.prompt)
            .await
            .map_err(|err| AssistantError::Inference(err.to_string()))?
            as i64;

        let stream = model
            .generate_stream(request.clone())
            .await
            .map_err(|err| AssistantError::Inference(err.to_string()))?;

        let metrics_guard =
            SessionMetricsGuard::new(self.metrics.clone(), provider_type, model_name.clone());

        Ok(AssistantStreamingSession::with_guards(
            stream,
            prompt_tokens,
            model,
            metrics_guard,
        ))
    }

    fn resolve_model_choice(
        &self,
        request: &LLMRequest,
    ) -> Result<(String, String, LLMConfig), AssistantError> {
        let model_name = request
            .metadata
            .get("model")
            .and_then(|value| value.as_str())
            .unwrap_or(&self.config.llm.default_chat_model)
            .to_string();

        let model_config = self
            .config
            .llm
            .get_model_config(&model_name)
            .ok_or_else(|| AssistantError::Config(format!("unknown LLM model '{model_name}'")))?;

        let provider_name = request
            .metadata
            .get("provider")
            .and_then(|value| value.as_str())
            .unwrap_or(&model_config.provider)
            .to_string();

        let provider_config = self
            .config
            .llm
            .get_provider_config(&provider_name)
            .ok_or_else(|| AssistantError::Config(format!("unknown provider '{provider_name}'")))?;

        let llm_config = self
            .config
            .llm
            .to_llm_config(&model_name)
            .map_err(AssistantError::Config)?;

        Ok((
            model_name,
            provider_config.provider_type.clone().to_lowercase(),
            llm_config,
        ))
    }

    async fn ensure_model(
        &self,
        cache_key: &str,
        provider_type: &str,
        llm_config: LLMConfig,
    ) -> Result<Arc<LlamaCppModel>, AssistantError> {
        {
            let guard = self.models.read().await;
            if let Some(model) = guard.get(cache_key) {
                metrics::counter!(
                    "llm_model_cache_hits_total",
                    "provider" => provider_type.to_string(),
                    "model" => cache_key.to_string()
                )
                .increment(1);
                return Ok(model.clone());
            }
        }

        let mut guard = self.models.write().await;
        if let Some(model) = guard.get(cache_key) {
            metrics::counter!(
                "llm_model_cache_hits_total",
                "provider" => provider_type.to_string(),
                "model" => cache_key.to_string()
            )
            .increment(1);
            return Ok(model.clone());
        }

        let load_started = Instant::now();
        let model = match provider_type {
            "llama_cpp" => load_llama_model(llm_config.clone()).await?,
            other => {
                return Err(AssistantError::Config(format!(
                    "unsupported LLM provider '{other}'"
                )));
            }
        };
        let elapsed = load_started.elapsed().as_secs_f64();
        metrics::histogram!(
            "llm_model_load_seconds",
            "provider" => provider_type.to_string(),
            "model" => cache_key.to_string()
        )
        .record(elapsed);

        let arc = Arc::new(model);
        guard.insert(cache_key.to_string(), arc.clone());
        drop(guard);
        Ok(arc)
    }

    pub fn persist_stream_chunks(&self) -> bool {
        self.config.llm.global_settings.persist_stream_chunks
    }

    pub fn default_model_name(&self) -> &str {
        &self.config.llm.default_chat_model
    }

    pub fn default_chat_config(&self) -> Result<LLMConfig, AssistantError> {
        self.config
            .llm
            .get_default_chat_config()
            .map_err(AssistantError::Config)
    }
}

#[async_trait]
impl AssistantRuntime for AssistantService {
    async fn stream_reply(
        &self,
        request: LLMRequest,
    ) -> Result<AssistantStreamingSession, AssistantError> {
        Self::stream_reply(self, request).await
    }

    fn persist_stream_chunks(&self) -> bool {
        Self::persist_stream_chunks(self)
    }

    fn default_model_name(&self) -> &str {
        Self::default_model_name(self)
    }

    fn default_chat_config(&self) -> Result<LLMConfig, AssistantError> {
        Self::default_chat_config(self)
    }
}

async fn load_llama_model(config: LLMConfig) -> Result<LlamaCppModel, AssistantError> {
    let provider = LlamaCppProvider::new(config.clone())
        .await
        .map_err(|err| AssistantError::Provider(err.to_string()))?;
    provider
        .load_model(&config.model_path)
        .await
        .map_err(|err| AssistantError::Provider(err.to_string()))
}

pub fn finish_reason_to_string(reason: &shared::llms::types::FinishReason) -> String {
    use shared::llms::types::FinishReason;
    match reason {
        FinishReason::EndOfText => "stop".to_string(),
        FinishReason::StopSequence => "stop_sequence".to_string(),
        FinishReason::MaxTokens => "length".to_string(),
        FinishReason::Cancelled => "cancelled".to_string(),
        FinishReason::Error => "error".to_string(),
    }
}

#[derive(Debug, Default)]
struct AssistantMetrics {
    active_sessions: Mutex<HashMap<String, usize>>,
}

impl AssistantMetrics {
    fn increment(&self, provider: &str, model: &str) {
        let mut guard = self
            .active_sessions
            .lock()
            .expect("assistant metrics mutex poisoned");
        let key = format!("{provider}::{model}");
        let count = guard.entry(key).or_insert(0);
        *count += 1;
        let current = *count;
        drop(guard);
        metrics::gauge!(
            "llm_active_sessions",
            "provider" => provider.to_string(),
            "model" => model.to_string()
        )
        .set(current as f64);
    }

    fn decrement(&self, provider: &str, model: &str) {
        let mut guard = self
            .active_sessions
            .lock()
            .expect("assistant metrics mutex poisoned");
        let key = format!("{provider}::{model}");
        let mut remove_entry = false;
        let current = guard.get_mut(&key).map_or(0, |count| {
            if *count > 0 {
                *count -= 1;
            }
            if *count == 0 {
                remove_entry = true;
            }
            *count
        });
        if remove_entry {
            guard.remove(&key);
        }
        drop(guard);
        metrics::gauge!(
            "llm_active_sessions",
            "provider" => provider.to_string(),
            "model" => model.to_string()
        )
        .set(current as f64);
    }
}

struct SessionMetricsGuard {
    metrics: Arc<AssistantMetrics>,
    provider: String,
    model: String,
}

impl SessionMetricsGuard {
    fn new(metrics: Arc<AssistantMetrics>, provider: String, model: String) -> Self {
        metrics.increment(&provider, &model);
        Self {
            metrics,
            provider,
            model,
        }
    }
}

impl Drop for SessionMetricsGuard {
    fn drop(&mut self) {
        self.metrics.decrement(&self.provider, &self.model);
    }
}
