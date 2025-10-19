# Local Development

> TL;DR – Configure environment variables, run the server, CLI, and web client together, and tighten the feedback loop with watch commands.

## Environment Setup

Copy the sample configuration and customise secrets for local use:

```bash
cp config.example.toml config.toml
```

Set `DATABASE_URL`, `OPENAI_API_KEY` (if required), and optional SMTP credentials in your shell or using `.env`. Refer to [Configuration](../reference/config.md) for the full list of supported keys.

## Run Watchers

Use `just` to orchestrate simultaneous watchers:

```bash
just dev
```

This spawns an auto-reloading backend (`rustygpt-server`) and a Trunk-powered Yew frontend. Logs route to stdout, and the SSE stream is available at `http://localhost:8080/api/chat/stream`.

For targeted backend iterations, run:

```bash
cargo watch -x 'test -p rustygpt-server'
```

## CLI Tooling

Build the CLI for quick smoke tests:

```bash
cargo run -p rustygpt-cli -- chat "Summarise deployment state"
```

The CLI shares configuration parsing with the server, so ensure the same `.env` is loaded. Inspect token accounting and stream behaviour in [Streaming Delivery](../architecture/streaming.md).

## Debugging

- View tracing spans by setting `RUST_LOG=rustygpt=debug,tower_http=info`.
- Inspect SSE payloads with `curl -N http://localhost:8080/api/chat/stream`.
- When debugging auth issues, follow the runbook in [Rotate Secrets](../howto/rotate-secrets.md).
