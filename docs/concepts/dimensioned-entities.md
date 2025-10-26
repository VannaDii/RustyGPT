# Shared models

`rustygpt-shared` centralises the data structures consumed by the server, web client, and CLI. Keeping these DTOs in one crate
prevents drift between components and allows `serde` + `utoipa` derives to stay consistent.

## Configuration loader

`src/config/server.rs` defines the `Config` struct and associated sub-structures (`ServerConfig`, `RateLimitConfig`,
`SseConfig`, etc.). Every binary loads configuration through `Config::load_config`, which merges defaults, optional files, and
environment overrides. Feature flags such as `features.auth_v1` gate server subsystems without requiring code changes.

## API payloads

The `src/models` directory contains strongly typed request/response structs:

- `models/chat.rs` – conversations, threads, message payloads, and streaming events
- `models/oauth.rs` – GitHub/Apple OAuth exchanges
- `models/setup.rs` – first-time setup contract (`SetupRequest`, `SetupResponse`)
- `models/limits.rs` – rate limit admin DTOs (`CreateRateLimitProfileRequest`, `RateLimitAssignment`, ...)
- `models/session.rs` – session summaries returned by `/api/auth/*`

All types derive `Serialize`, `Deserialize`, and when relevant `utoipa::ToSchema` so the OpenAPI generator stays in sync.

## LLM abstractions

`src/llms` exposes traits (`LLMProvider`, `LLMModel`) and helpers for llama.cpp integration. The server’s
`AssistantService` uses these traits to stream responses and emit metrics (`llm_model_cache_hits_total`, `llm_model_load_seconds`).
When you add a new provider, implement the traits here and update the configuration schema.

## Why it matters

- **Type safety** – clients compile against the same structs the server uses, catching breaking changes early.
- **Single-source documentation** – OpenAPI docs and mdBook pages pull names directly from these types.
- **Testing** – shared fixtures in `shared::models` make it easier to write integration tests that cover both server handlers and
  CLI commands.

Whenever you extend the API, add or update the relevant struct in `rustygpt-shared` first, then regenerate the OpenAPI spec with
`cargo run -p rustygpt-cli -- spec`.
