# RustyGPT

[![CI](https://github.com/VannaDii/RustyGPT/actions/workflows/ci.yml/badge.svg)](https://github.com/VannaDii/RustyGPT/actions/workflows/ci.yml)
[![Lint](https://github.com/VannaDii/RustyGPT/actions/workflows/lint.yml/badge.svg)](https://github.com/VannaDii/RustyGPT/actions/workflows/lint.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

RustyGPT is a workspace of Rust crates that together provide a chat assistant server, a Yew web UI, and a command line interface.
The project focuses on end-to-end Rust implementations for authentication, threaded conversations, Server-Sent Event (SSE)
streaming, and local LLM execution through a pluggable llama.cpp provider.

## Workspace layout

| Crate | Purpose |
| ----- | ------- |
| [`rustygpt-server`](rustygpt-server) | Axum HTTP server with authentication, rate limiting, SSE streaming, and OpenAPI documentation. |
| [`rustygpt-web`](rustygpt-web) | Yew single-page application that consumes the server APIs and renders threaded conversations. |
| [`rustygpt-cli`](rustygpt-cli) | Command line client for logging in, inspecting conversations, following SSE streams, and running the server locally. |
| [`rustygpt-shared`](rustygpt-shared) | Shared models, configuration loader, and llama.cpp integration code reused by all binaries. |
| [`rustygpt-doc-indexer`](rustygpt-doc-indexer) | Helper used by the docs build to generate the machine-readable index. |
| [`rustygpt-tools`](rustygpt-tools)`/confuse` | Development helper that runs frontend/backend watchers via the [`just dev`](Justfile) recipe. |

Other notable directories include [`scripts/pg`](scripts/pg) for schema/procedure SQL, [`deploy/grafana`](deploy/grafana) for
metrics dashboards, and [`docs`](docs) for the mdBook documentation.

## Capabilities

* **Threaded conversations** – `/api/conversations` and `/api/threads` endpoints manage conversation membership, invites, roots,
  and replies (`rustygpt-server/src/handlers/{conversations,threads}.rs`).
* **Streaming updates** – `conversation_stream` in `handlers/streaming.rs` broadcasts `ConversationStreamEvent` values over SSE at
  `/api/stream/conversations/:conversation_id`, with optional PostgreSQL persistence configured through `[sse.persistence]`.
* **Authentication** – cookie-backed sessions, refresh, and logout flows (see `handlers/auth.rs`) plus optional GitHub or Apple
  OAuth handlers when the relevant environment variables are present. First-time setup uses `/api/setup` to create the initial
  administrator (`handlers/setup.rs`).
* **Rate limiting** – `middleware::rate_limit` enforces per-route buckets populated from the database using stored procedures in
  `scripts/pg/procs/034_limits.sql`. Admin APIs under `/api/admin/limits/*` allow live updates when `rate_limits.admin_api_enabled`
  and `features.auth_v1` are enabled.
* **Local LLM inference** – `AssistantService` streams replies via llama.cpp models configured under `[llm]` in `config.toml`,
  with metrics such as `llm_model_cache_hits_total` and `llm_model_load_seconds`.
* **Observability** – Prometheus counters and gauges for health checks, bootstrap progress, rate limiting, and LLM usage, plus
  `/metrics`, `/healthz`, and `/readyz` endpoints. Grafana dashboards live in `deploy/grafana/`.
* **Typed configuration** – `rustygpt-shared::config::server::Config` loads layered TOML/YAML/JSON files with environment
  overrides (e.g. `RUSTYGPT__SERVER__PORT`). The template [`config.example.toml`](config.example.toml) documents all sections.

## Quick start

1. **Install prerequisites**
   * Rust 1.81+ (`rustup default stable`)
   * `just`, `cargo-watch`, and `trunk`
   * PostgreSQL 15+ (local install or Docker)
   * Optional: llama.cpp-compatible model files for streaming replies

2. **Create a configuration file**
   ```bash
   cp config.example.toml config.toml
   ```
   Adjust values as needed. For a full local experience set:
   ```toml
   [features]
   auth_v1 = true
   sse_v1 = true
   well_known = true
   ```
   Ensure `[db].url` points to your PostgreSQL instance and that the database already exists.

3. **Start PostgreSQL**
   You can use the provided Compose service:
   ```bash
   docker compose up postgres -d
   ```
   The server automatically runs the bootstrap SQL in `scripts/pg` on startup.

4. **Run the backend**
   ```bash
   just run-server
   ```
   or
   ```bash
   cargo run -p rustygpt-server -- serve --port 8080
   ```
   The process listens on `http://127.0.0.1:8080` by default.

5. **Perform first-time setup**
   POST to `/api/setup` once to create the initial admin account:
   ```bash
   curl -X POST http://127.0.0.1:8080/api/setup \
     -H 'Content-Type: application/json' \
     -d '{"username":"admin","email":"admin@example.com","password":"change-me"}'
   ```

6. **Run the web client**
   ```bash
   cd rustygpt-web
   trunk serve
   ```
   The SPA proxies API requests to the backend and renders conversations, presence, and streaming updates.

7. **Use the CLI**
   ```bash
   cargo run -p rustygpt-cli -- login
   cargo run -p rustygpt-cli -- chat --conversation <uuid>
   cargo run -p rustygpt-cli -- follow --root <thread-uuid>
   ```
   Commands reuse the same configuration loader and session cookies as the server. See [`rustygpt-cli/src/main.rs`](rustygpt-cli/src/main.rs)
   for the full list of subcommands (`serve`, `chat`, `reply`, `follow`, `spec`, `completion`, `config`, `login`, `me`, `logout`).

## Observability

Metrics are exposed at `/metrics` after calling `server::metrics_handle()`. Key instruments include:

| Metric | Description |
| ------ | ----------- |
| `health_checks_total{endpoint,status}` | Count of `/healthz` and `/readyz` responses. |
| `db_bootstrap_batches_total{stage,status}` / `db_bootstrap_script_duration_seconds{stage,status}` | Bootstrap progress per SQL stage (`schema`, `procedures`, `indexes`, `seed`). |
| `db_liveness_checks_total{status}` / `db_readiness_checks_total{status}` | Database readiness probes. |
| `db_pool_max_connections`, `db_statement_timeout_ms` | Gauges reflecting the active configuration. |
| `http_rate_limit_requests_total{profile,result}` | Requests allowed or denied by the rate limit middleware. |
| `http_rate_limit_remaining{profile}` / `http_rate_limit_reset_seconds{profile}` | Current token state per bucket. |
| `rustygpt_limits_profiles`, `rustygpt_limits_assignments` | Gauges updated when admin routes reload configuration. |
| `llm_model_cache_hits_total{provider,model}` / `llm_model_load_seconds{provider,model}` | llama.cpp model cache activity. |

Import the Grafana dashboards in `deploy/grafana/*.json` to visualise these metrics.

## Documentation

The mdBook at [`docs/`](docs) covers architecture, API reference, configuration keys, and operational guides. Run
`just docs-serve` to preview it locally or browse the published version via GitHub Pages.

## Contributing

Contributions are welcome! Please review [`CONTRIBUTING.md`](CONTRIBUTING.md) and the code of conduct before opening a pull
request. Run `just check` and `just test` prior to submitting changes. Security concerns should be reported via the
[`SECURITY.md`](SECURITY.md) process.

## License

RustyGPT is available under the Apache 2.0 license. See [`LICENSE`](LICENSE) for details.
