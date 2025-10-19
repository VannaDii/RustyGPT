# Configuration

> TL;DR – All RustyGPT services share a TOML configuration merged with environment variables; this page maps each key to runtime behaviour.

## Loading Order

1. `config.example.toml` documents defaults.
2. `config.toml` (optional) overrides per environment.
3. Environment variables (e.g., `RUSTYGPT__SERVER__PORT`) take precedence.

The loader lives in `rustygpt-shared::config`. Combine these steps with the env setup in [Local Development](../guide/local-dev.md).

## Server Block

```toml
[server]
port = 8080
bind_address = "0.0.0.0"
sse_keepalive_seconds = 15
```

- `port` – HTTP port.
- `bind_address` – interface binding.
- `sse_keepalive_seconds` – interval for no-op SSE frames (see [Streaming Delivery](../architecture/streaming.md)).

## Session Block

```toml
[session]
idle_seconds = 28800
absolute_seconds = 604800
session_cookie_name = "sid"
csrf_cookie_name = "CSRF-TOKEN"
max_sessions_per_user = 5
```

These values govern cookie lifetime and rotation; align with the practices in [Rotate Secrets](../howto/rotate-secrets.md).

## Database Block

```toml
[database]
url = "postgres://postgres:postgres@localhost:5432/rustygpt"
pool_max_connections = 10
statement_timeout_ms = 10000
```

- `url` – connection string; honour TLS requirements in production.
- `statement_timeout_ms` – prevents long-running queries from stalling the pool.

## Rate-Limit Block

```toml
[rate_limits]
admin_api_enabled = true
default_rps = 10
default_burst = 20
```

- `admin_api_enabled` – toggles the admin endpoints described in [REST API](api.md).
- `default_rps` / `default_burst` – fallback strategy for unassigned routes.

## Observability Block

```toml
[telemetry]
log_level = "info"
metrics_endpoint = "/metrics"
```

Expose `/metrics` and feed it into your Prometheus deployment. Dashboards shipped in `deploy/grafana` expect this path.

## Secrets

Environment variables prefixed with `RUSTYGPT__SECRETS__` override sensitive values (API keys, signing secrets). Rotate them via the workflow in [Rotate Secrets](../howto/rotate-secrets.md) to avoid service interruption.
