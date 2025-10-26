# Quickstart

> TL;DR – copy `config.example.toml`, enable the feature flags you need, start PostgreSQL, run the Axum server, complete the
> `/api/setup` flow, then bring up the Yew frontend and CLI.

## 1. Prerequisites

- Rust toolchain (`rustup default stable`), `cargo`, and [`just`](https://just.systems)
- [`trunk`](https://trunkrs.dev/) for the web client (`cargo install trunk`)
- PostgreSQL 15+ running locally (the provided `docker-compose.yaml` exposes one at `postgres://tinroof:rusty@localhost:5432/rusty_gpt`)
- Optional: llama.cpp-compatible model files if you plan to exercise assistant streaming

Fetch dependencies once:

```bash
cargo fetch --workspace
```

## 2. Configure the server

Create `config.toml` and adjust it for your environment:

```bash
cp config.example.toml config.toml
```

At minimum set:

```toml
[db]
url = "postgres://tinroof:rusty@localhost/rustygpt_dev"

[features]
auth_v1 = true
sse_v1 = true
well_known = true
```

`rustygpt-shared::config::server::Config` supports TOML/YAML/JSON files and environment overrides (e.g.
`RUSTYGPT__SERVER__PORT=8080`). See [Configuration](../reference/config.md) for the complete matrix of keys.

## 3. Start PostgreSQL

If you are using Docker Compose:

```bash
docker compose up postgres -d
```

On server startup the bootstrap runner executes every SQL script under `scripts/pg/{schema,procedures,indexes,seed}` in order.
The seed stage enables feature flags and inserts the default rate-limit profile (`conversation.post`).

## 4. Run the backend

Either launch directly:

```bash
cargo run -p rustygpt-server -- serve --port 8080
```

or use the helper recipe that also builds configuration if needed:

```bash
just run-server
```

The process listens on `http://127.0.0.1:8080`. Health probes are available at `/api/healthz` and `/api/readyz`.

## 5. Complete initial setup

The first authenticated user is created by POSTing to `/api/setup`:

```bash
curl -X POST http://127.0.0.1:8080/api/setup \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","email":"admin@example.com","password":"change-me"}'
```

Subsequent calls return `400` once a user already exists.

## 6. Start the web client

In a new terminal:

```bash
cd rustygpt-web
trunk serve
```

The SPA proxies `/api/*` requests to the backend. After logging in you should see:

- Conversation list populated by `GET /api/conversations/{conversation_id}/threads`
- Thread view that streams updates from `/api/stream/conversations/{conversation_id}`
- Presence and typing indicators driven by `ConversationStreamEvent` payloads

## 7. Exercise the CLI

The `rustygpt` binary shares configuration and cookie handling with the server:

```bash
cargo run -p rustygpt-cli -- login
cargo run -p rustygpt-cli -- chat --conversation <conversation-uuid>
cargo run -p rustygpt-cli -- follow --root <thread-uuid>
```

`follow` connects to the SSE endpoint, reconstructs events, and prints deltas as they arrive. If you see `authentication required`
errors, confirm you completed the setup step and that `[features].auth_v1` is `true`.

## 8. Next steps

- Review [Local Development](local-dev.md) for watcher workflows, linting, and debugging tips.
- Explore [REST API](../reference/api.md) for a full endpoint catalogue and payload shapes.
- Consult [Service Topology](../architecture/service-topology.md) to understand how the components interact at runtime.
