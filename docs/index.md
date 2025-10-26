# RustyGPT Documentation

Welcome! This mdBook describes the RustyGPT workspace in depth: how the server and clients are structured, how the
PostgreSQL-backed features behave, and how to operate the platform locally or in shared environments.

*Start with the guides to get a local environment running, then dive into the reference and architecture chapters for precise
APIs and design notes.*

## Quick navigation

- [Quickstart](guide/quickstart.md) – configure and launch the server, web client, and CLI.
- [Local development](guide/local-dev.md) – watcher workflows, debugging tools, and environment variables.
- [REST API](reference/api.md) – endpoint catalogue for conversations, streaming, authentication, and admin features.
- [Service topology](architecture/service-topology.md) – how the Axum server, Yew SPA, PostgreSQL, and SSE stream hub fit together.

## What RustyGPT ships today

RustyGPT focuses on a cohesive Rust stack:

- `rustygpt-server` exposes REST + SSE endpoints with cookie-based auth (`handlers/auth.rs`), rate limiting
  (`middleware/rate_limit.rs`), and OpenAPI documentation (`openapi.rs`).
- `rustygpt-web` is a Yew SPA that consumes the server API via `src/api.rs` and renders threaded conversations, presence, and
  typing indicators.
- `rustygpt-cli` shares the same models as the server, providing commands for login, conversation inspection, SSE following, and
  OpenAPI generation (`src/commands`).
- `rustygpt-shared` houses configuration loading, llama.cpp bindings, and the data transfer objects used across all crates.

Each documentation section links back to the relevant modules so you can cross-reference behaviour with the implementation.
