//! Llama.cpp backed provider.

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        collections::HashMap,
        convert::TryInto,
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Instant,
    };

    use async_stream::try_stream;
    use chrono::Utc;
    use futures_util::StreamExt;
    use llama_cpp::{
        LlamaContextError, LlamaLoadError, LlamaModel, LlamaParams, LlamaSession,
        LlamaTokenizationError, SessionParams,
        standard_sampler::{SamplerStage, StandardSampler},
    };
    use tokio::task;
    use tracing::info;

    use crate::llms::{
        errors::{LLMError, LLMResult},
        traits::{LLMModel, LLMProvider, StreamingResponseStream},
        types::{
            FinishReason, LLMConfig, LLMRequest, LLMResponse, ModelCapabilities, ModelInfo,
            StreamingResponse, TokenUsage,
        },
    };

    #[derive(Debug, Clone)]
    pub struct LlamaCppProvider {
        base_config: LLMConfig,
    }

    #[derive(Clone)]
    pub struct LlamaCppModel {
        config: LLMConfig,
        model: Arc<LlamaModel>,
        info: ModelInfo,
        ready: Arc<AtomicBool>,
    }

    impl LlamaCppProvider {
        fn model_params(config: &LLMConfig) -> LlamaParams {
            let mut params = LlamaParams::default();
            if let Some(layers) = config.n_gpu_layers {
                params.n_gpu_layers = layers;
            }
            if let Some(use_mmap) = config
                .additional_params
                .get("use_mmap")
                .and_then(|value| value.as_bool())
            {
                params.use_mmap = use_mmap;
            }
            if let Some(use_mlock) = config
                .additional_params
                .get("use_mlock")
                .and_then(|value| value.as_bool())
            {
                params.use_mlock = use_mlock;
            }
            params
        }

        fn build_model_info(model: &LlamaModel, config: &LLMConfig) -> ModelInfo {
            let name = Path::new(&config.model_path)
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| config.model_path.clone());

            let capabilities = ModelCapabilities {
                text_generation: true,
                text_embedding: false,
                chat_format: true,
                function_calling: false,
                streaming: true,
                max_context_length: config.context_size,
                supported_languages: vec!["en".to_string()],
            };

            ModelInfo {
                name,
                version: None,
                architecture: Some("llama.cpp".to_string()),
                parameter_count: None,
                context_length: Some(model.train_len() as u32),
                capabilities,
            }
        }
    }

    #[async_trait::async_trait]
    impl LLMProvider for LlamaCppProvider {
        type Model = LlamaCppModel;

        async fn new(config: LLMConfig) -> LLMResult<Self> {
            if config.model_path.is_empty() {
                return Err(LLMError::invalid_config(
                    "model_path",
                    "model path cannot be empty",
                ));
            }
            Ok(Self {
                base_config: config,
            })
        }

        async fn load_model(&self, model_path: &str) -> LLMResult<Self::Model> {
            let mut config = self.base_config.clone();
            config.model_path = model_path.to_string();
            self.load_model_with_config(config).await
        }

        async fn load_model_with_config(&self, config: LLMConfig) -> LLMResult<Self::Model> {
            let path = PathBuf::from(&config.model_path);
            if !path.exists() {
                return Err(LLMError::model_not_found(&config.model_path));
            }

            let params = Self::model_params(&config);
            let load_started = Instant::now();
            let model = task::spawn_blocking({
                let path = path.clone();
                move || LlamaModel::load_from_file(path, params)
            })
            .await
            .map_err(|err| LLMError::internal(err))?
            .map_err(map_load_error)?;

            let elapsed = load_started.elapsed().as_secs_f64();
            info!(
                model_path = %config.model_path,
                elapsed_seconds = elapsed,
                "loaded llama.cpp model"
            );

            let info = Self::build_model_info(&model, &config);

            Ok(LlamaCppModel {
                config,
                model: Arc::new(model),
                info,
                ready: Arc::new(AtomicBool::new(true)),
            })
        }

        async fn list_available_models(&self) -> LLMResult<Vec<String>> {
            Ok(vec![self.base_config.model_path.clone()])
        }

        fn get_provider_info(&self) -> String {
            "llama_cpp (native)".to_string()
        }

        fn is_model_supported(&self, model_path: &str) -> bool {
            Path::new(model_path)
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("gguf"))
                .unwrap_or(false)
        }
    }

    #[async_trait::async_trait]
    impl LLMModel for LlamaCppModel {
        async fn generate(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
            let mut stream = self.generate_stream(request.clone()).await?;
            let mut text = String::new();
            let mut usage = TokenUsage::default();
            let mut finish_reason = None;

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                text.push_str(&chunk.text_delta);
                usage = chunk.usage.clone();
                if chunk.is_final {
                    finish_reason = chunk.finish_reason.clone();
                }
            }

            Ok(LLMResponse {
                request_id: request.id,
                text,
                finished: true,
                finish_reason,
                usage,
                timestamp: Utc::now(),
                model_info: self.info.clone(),
                metadata: HashMap::new(),
            })
        }

        async fn generate_stream(&self, request: LLMRequest) -> LLMResult<StreamingResponseStream> {
            if !self.ready.load(Ordering::SeqCst) {
                return Err(LLMError::ModelNotLoaded);
            }

            let model = Arc::clone(&self.model);
            let config = self.config.clone();

            let stream = try_stream! {
                let mut session = create_session(model.clone(), &config).await?;
                let prompt = combine_prompt(&request);
                session
                    .advance_context_async(prompt.as_bytes())
                    .await
                    .map_err(map_context_error)?;

                let prompt_tokens = session
                    .context_size()
                    .try_into()
                    .unwrap_or(u32::MAX);

                let sampler = build_sampler(&config, &request);
                let max_tokens_limit = request
                    .max_tokens
                    .or(config.max_tokens)
                    .unwrap_or(512);
                let max_tokens = if max_tokens_limit == 0 {
                    None
                } else {
                    Some(max_tokens_limit)
                };

                let max_predictions = max_tokens
                    .map(|value| value as usize)
                    .unwrap_or(u32::MAX as usize);

                let completion = session
                    .start_completing_with(
                        sampler,
                        max_predictions,
                    )
                    .map_err(map_context_error)?;

                let mut token_bytes = completion.into_bytes();
                let mut decoder = Utf8Decoder::new();
                let mut aggregated = String::new();
                let mut emitted_tokens: u32 = 0;
                let mut finish_reason: Option<FinishReason> = None;
                let stop_sequences = request.stop_sequences.clone();

                while let Some(bytes) = StreamExt::next(&mut token_bytes).await {
                    emitted_tokens = emitted_tokens.saturating_add(1);
                    let mut delta = decoder.push_token(&bytes);
                    if delta.is_empty() {
                        continue;
                    }
                    aggregated.push_str(&delta);

                    if let Some(stop) = detect_stop_sequence(&aggregated, &stop_sequences) {
                        let stop_len = stop.len();
                        aggregated.truncate(aggregated.len().saturating_sub(stop_len));
                        if stop_len <= delta.len() {
                            delta.truncate(delta.len() - stop_len);
                        } else {
                            delta.clear();
                        }
                        if !delta.is_empty() {
                            let usage = TokenUsage::new(prompt_tokens, emitted_tokens);
                            yield StreamingResponse {
                                request_id: request.id,
                                text_delta: delta.clone(),
                                is_final: false,
                                current_text: None,
                                finish_reason: None,
                                usage,
                                timestamp: Utc::now(),
                            };
                        }
                        finish_reason = Some(FinishReason::StopSequence);
                        break;
                    }

                    let usage = TokenUsage::new(prompt_tokens, emitted_tokens);
                    yield StreamingResponse {
                        request_id: request.id,
                        text_delta: delta.clone(),
                        is_final: false,
                        current_text: None,
                        finish_reason: None,
                        usage,
                        timestamp: Utc::now(),
                    };

                    if let Some(limit) = max_tokens {
                        if emitted_tokens >= limit {
                            finish_reason = Some(FinishReason::MaxTokens);
                            break;
                        }
                    }
                }

                let final_delta = decoder.flush().unwrap_or_default();
                if !final_delta.is_empty() {
                    aggregated.push_str(&final_delta);
                }

                let usage = TokenUsage::new(prompt_tokens, emitted_tokens);
                let final_reason = finish_reason.unwrap_or(FinishReason::EndOfText);
                yield StreamingResponse {
                    request_id: request.id,
                    text_delta: final_delta.clone(),
                    is_final: true,
                    current_text: Some(aggregated.clone()),
                    finish_reason: Some(final_reason),
                    usage,
                    timestamp: Utc::now(),
                };
            };

            Ok(Box::pin(stream))
        }

        fn get_model_info(&self) -> ModelInfo {
            self.info.clone()
        }

        fn is_ready(&self) -> bool {
            self.ready.load(Ordering::SeqCst)
        }

        async fn unload(&mut self) -> LLMResult<()> {
            self.ready.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn get_memory_usage(&self) -> Option<usize> {
            // llama.cpp does not currently expose reliable per-model memory usage.
            None
        }

        async fn tokenize(&self, text: &str) -> LLMResult<Vec<u32>> {
            let model = Arc::clone(&self.model);
            let input = text.to_owned();
            let tokens =
                task::spawn_blocking(move || model.tokenize_bytes(input.as_bytes(), false, true))
                    .await
                    .map_err(|err| LLMError::internal(err))?
                    .map_err(map_tokenization_error)?;

            Ok(tokens.into_iter().map(|token| token.0 as u32).collect())
        }
    }

    async fn create_session(model: Arc<LlamaModel>, config: &LLMConfig) -> LLMResult<LlamaSession> {
        let params = session_params(config);
        task::spawn_blocking(move || model.create_session(params))
            .await
            .map_err(|err| LLMError::internal(err))?
            .map_err(map_context_error)
    }

    fn session_params(config: &LLMConfig) -> SessionParams {
        let mut params = SessionParams::default();
        if let Some(ctx) = config.context_size {
            params.n_ctx = ctx;
        }
        if let Some(batch) = config.batch_size {
            params.n_batch = batch;
            params.n_ubatch = batch;
        }
        if let Some(threads) = config.n_threads {
            params.n_threads = threads;
            params.n_threads_batch = threads;
        }
        params
    }

    fn build_sampler(config: &LLMConfig, request: &LLMRequest) -> StandardSampler {
        let mut stages = Vec::new();
        let repetition_penalty = config.repetition_penalty.unwrap_or(1.1);
        stages.push(SamplerStage::RepetitionPenalty {
            repetition_penalty,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            last_n: 64,
        });

        if let Some(top_k) = config.top_k {
            stages.push(SamplerStage::TopK(top_k as i32));
        }
        if let Some(top_p) = request
            .metadata
            .get("top_p")
            .and_then(|value| value.as_f64())
            .map(|value| value as f32)
            .or(config.top_p)
        {
            stages.push(SamplerStage::TopP(top_p));
        }
        if let Some(min_p) = request
            .metadata
            .get("min_p")
            .and_then(|value| value.as_f64())
            .map(|value| value as f32)
            .or_else(|| {
                config
                    .additional_params
                    .get("min_p")
                    .and_then(|value| value.as_f64())
                    .map(|value| value as f32)
            })
        {
            stages.push(SamplerStage::MinP(min_p));
        }

        let temperature = request.temperature.or(config.temperature).unwrap_or(0.8);
        stages.push(SamplerStage::Temperature(temperature));

        StandardSampler::new_softmax(stages, 1)
    }

    fn combine_prompt(request: &LLMRequest) -> String {
        request.system_message.as_ref().map_or_else(
            || request.prompt.clone(),
            |system| {
                if system.trim().is_empty() {
                    request.prompt.clone()
                } else {
                    format!("{}\n\n{}", system.trim(), request.prompt)
                }
            },
        )
    }

    fn detect_stop_sequence<'a>(text: &'a str, stops: &'a [String]) -> Option<&'a str> {
        stops
            .iter()
            .filter_map(|sequence| {
                if text.ends_with(sequence) {
                    Some(sequence.as_str())
                } else {
                    None
                }
            })
            .max_by_key(|sequence| sequence.len())
    }

    fn map_load_error(error: LlamaLoadError) -> LLMError {
        match error {
            LlamaLoadError::DoesNotExist(path) => {
                LLMError::model_not_found(path.display().to_string())
            }
            LlamaLoadError::LlamaError(inner) => LLMError::model_init_failed(inner.to_string()),
        }
    }

    fn map_context_error(error: LlamaContextError) -> LLMError {
        match error {
            LlamaContextError::TokenizationFailed(err) => LLMError::TokenizationError {
                details: err.to_string(),
            },
            LlamaContextError::MaxTokensExceeded {
                provided_tokens,
                max_tokens,
            } => LLMError::generation_failed(format!(
                "maximum context exceeded: {provided_tokens} > {max_tokens}"
            )),
            LlamaContextError::SessionFailed => {
                LLMError::model_init_failed("failed to create llama session")
            }
            LlamaContextError::DecodeFailed(code) => {
                LLMError::generation_failed(format!("decode failed with error code {code}"))
            }
            LlamaContextError::EmbeddingsFailed(reason) => LLMError::generation_failed(reason),
            LlamaContextError::InvalidRange => {
                LLMError::generation_failed("invalid KV cache range")
            }
            LlamaContextError::NoContext => {
                LLMError::generation_failed("no prompt context available")
            }
        }
    }

    fn map_tokenization_error(error: LlamaTokenizationError) -> LLMError {
        match error {
            LlamaTokenizationError::InputTooLarge { n_bytes, max_bytes } => {
                LLMError::TokenizationError {
                    details: format!(
                        "input too large: {n_bytes} bytes exceeds maximum {max_bytes} bytes"
                    ),
                }
            }
            LlamaTokenizationError::LlamaError(inner) => LLMError::TokenizationError {
                details: inner.to_string(),
            },
        }
    }

    #[derive(Default)]
    struct Utf8Decoder {
        buf: Vec<u8>,
    }

    impl Utf8Decoder {
        const fn new() -> Self {
            Self { buf: Vec::new() }
        }

        fn push_token(&mut self, token: &[u8]) -> String {
            let mut token = token;
            let mut output = String::new();

            let owned_storage = if self.buf.is_empty() {
                None
            } else {
                let mut combined = self.buf.clone();
                combined.extend_from_slice(token);
                self.buf.clear();
                Some(combined)
            };

            if let Some(buffer) = owned_storage.as_ref() {
                token = buffer.as_slice();
            }

            loop {
                match std::str::from_utf8(token) {
                    Ok(text) => {
                        output.push_str(text);
                        self.buf.clear();
                        break;
                    }
                    Err(err) => {
                        let valid_up_to = err.valid_up_to();
                        if valid_up_to > 0 {
                            unsafe {
                                output
                                    .push_str(std::str::from_utf8_unchecked(&token[..valid_up_to]));
                            }
                        }

                        if let Some(invalid_len) = err.error_len() {
                            output.push(char::REPLACEMENT_CHARACTER);
                            let consumed = valid_up_to + invalid_len;
                            if consumed >= token.len() {
                                self.buf.clear();
                                break;
                            }
                            token = &token[consumed..];
                        } else {
                            self.buf.clear();
                            self.buf.extend_from_slice(&token[valid_up_to..]);
                            break;
                        }
                    }
                }
            }

            output
        }

        fn flush(&mut self) -> Option<String> {
            if self.buf.is_empty() {
                None
            } else {
                let text = String::from_utf8_lossy(&self.buf).to_string();
                self.buf.clear();
                Some(text)
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod native {
    use std::collections::HashMap;

    use crate::llms::{
        errors::{LLMError, LLMResult},
        traits::{LLMModel, LLMProvider, StreamingResponseStream},
        types::{
            FinishReason, LLMConfig, LLMRequest, LLMResponse, ModelCapabilities, ModelInfo,
            StreamingResponse, TokenUsage,
        },
    };
    use async_stream::try_stream;
    use chrono::Utc;

    #[derive(Debug, Clone)]
    pub struct LlamaCppProvider {
        config: LLMConfig,
    }

    #[derive(Debug, Clone)]
    pub struct LlamaCppModel {
        config: LLMConfig,
        info: ModelInfo,
    }

    #[async_trait::async_trait]
    impl LLMProvider for LlamaCppProvider {
        type Model = LlamaCppModel;

        async fn new(config: LLMConfig) -> LLMResult<Self> {
            Ok(Self { config })
        }

        async fn load_model(&self, _: &str) -> LLMResult<Self::Model> {
            self.load_model_with_config(self.config.clone()).await
        }

        async fn load_model_with_config(&self, config: LLMConfig) -> LLMResult<Self::Model> {
            Ok(LlamaCppModel {
                info: ModelInfo {
                    name: "mock-llama".to_string(),
                    version: None,
                    architecture: Some("mock".to_string()),
                    parameter_count: None,
                    context_length: config.context_size,
                    capabilities: ModelCapabilities {
                        text_generation: true,
                        text_embedding: false,
                        chat_format: true,
                        function_calling: false,
                        streaming: true,
                        max_context_length: config.context_size,
                        supported_languages: vec!["en".to_string()],
                    },
                },
                config,
            })
        }

        async fn list_available_models(&self) -> LLMResult<Vec<String>> {
            Ok(vec![self.config.model_path.clone()])
        }

        fn get_provider_info(&self) -> String {
            "llama_cpp (wasm mock)".to_string()
        }

        fn is_model_supported(&self, _model_path: &str) -> bool {
            true
        }
    }

    #[async_trait::async_trait]
    impl LLMModel for LlamaCppModel {
        async fn generate(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
            let text = mock_response(&request.prompt);
            Ok(LLMResponse {
                request_id: request.id,
                text: text.clone(),
                finished: true,
                finish_reason: Some(FinishReason::EndOfText),
                usage: TokenUsage::new(16, 16),
                timestamp: Utc::now(),
                model_info: self.info.clone(),
                metadata: HashMap::new(),
            })
        }

        async fn generate_stream(&self, request: LLMRequest) -> LLMResult<StreamingResponseStream> {
            let text = mock_response(&request.prompt);
            let stream = try_stream! {
                yield StreamingResponse {
                    request_id: request.id,
                    text_delta: text.clone(),
                    is_final: true,
                    current_text: Some(text.clone()),
                    finish_reason: Some(FinishReason::EndOfText),
                    usage: TokenUsage::new(16, 16),
                    timestamp: Utc::now(),
                };
            };
            Ok(Box::pin(stream))
        }

        fn get_model_info(&self) -> ModelInfo {
            self.info.clone()
        }

        fn is_ready(&self) -> bool {
            true
        }

        async fn unload(&mut self) -> LLMResult<()> {
            Ok(())
        }

        fn get_memory_usage(&self) -> Option<usize> {
            None
        }

        async fn tokenize(&self, text: &str) -> LLMResult<Vec<u32>> {
            Ok(text
                .split_whitespace()
                .enumerate()
                .map(|(idx, _)| idx as u32)
                .collect())
        }
    }

    fn mock_response(prompt: &str) -> String {
        if prompt.trim().is_empty() {
            "This is a mock response from the wasm build.".to_string()
        } else {
            format!("Mock response for \"{}\"", prompt.trim())
        }
    }
}

pub use native::*;
