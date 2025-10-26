# Configuration

RustyGPT uses a layered configuration loader (`rustygpt-shared::config::server::Config`). Defaults are determined by the active
profile (Dev/Test/Prod), then merged with an optional file and environment overrides. CLI flags can override specific values
(e.g. `--port`).

## Loading order

1. Profile defaults (`Config::default_for_profile(Profile::Dev)`)
2. Optional config file (`config.toml`, `config.yaml`, or `config.json`)
3. Environment variables using double underscores (e.g. `RUSTYGPT__SERVER__PORT=9000`)
4. CLI overrides (currently the server/CLI `--port` flag)

`Config::load_config(path, override_port)` performs this merge and validates required fields.

## Key sections

### `[logging]`

```toml
[logging]
level = "info"        # tracing level passed to tracing-subscriber
format = "text"        # "text" or "json"
```

### `[server]`

```toml
[server]
host = "127.0.0.1"
port = 8080
public_base_url = "http://localhost:8080"
request_id_header = "x-request-id"

[server.cors]
allowed_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]
allow_credentials = false
max_age_seconds = 600
```

`public_base_url` is derived automatically when not supplied (scheme depends on profile). `request_id_header` controls which
header the middleware reads when assigning request IDs.

### `[security]`

```toml
[security.hsts]
enabled = false
max_age_seconds = 63072000
include_subdomains = true
preload = false

[security.cookie]
domain = ""
secure = false
same_site = "lax"

[security.csrf]
cookie_name = "CSRF-TOKEN"
header_name = "X-CSRF-TOKEN"
enabled = true
```

### `[rate_limits]`

```toml
[rate_limits]
auth_login_per_ip_per_min = 10
default_rps = 50.0
burst = 100
admin_api_enabled = false
```

When `admin_api_enabled = true` the `/api/admin/limits/*` routes become available.

### `[session]`

```toml
[session]
idle_seconds = 28800
absolute_seconds = 604800
session_cookie_name = "SESSION_ID"
csrf_cookie_name = "CSRF-TOKEN"
max_sessions_per_user = 5
```

Set `max_sessions_per_user = 0` (or `null`) to disable automatic eviction.

### `[oauth]`

```toml
[oauth]
redirect_base = "http://localhost:8080/api/auth/github/callback"

[oauth.github]
client_id = "..."
client_secret = "..."
```

If `oauth.github` is omitted the GitHub endpoints still respond but return empty URLs. Apple support reads `APPLE_*`
environment variables directly in the handler.

### `[db]`

```toml
[db]
url = "postgres://tinroof:rusty@localhost/rustygpt_dev"
statement_timeout_ms = 5000
max_connections = 10
bootstrap_path = "../scripts/pg"
```

`bootstrap_path` points to the directory containing `schema/`, `procedures/`, `indexes/`, and `seed/` folders.

### `[sse]`

```toml
[sse]
heartbeat_seconds = 20
channel_capacity = 128
id_prefix = "evt_"

[sse.persistence]
enabled = false
max_events_per_user = 500
prune_batch_size = 100
retention_hours = 48

[sse.backpressure]
drop_strategy = "drop_tokens"
warn_queue_ratio = 0.75
```

### `[features]`

```toml
[features]
auth_v1 = true
sse_v1 = true
well_known = true
```

Flags gate optional subsystems without recompiling the binary.

### `[cli]` and `[web]`

```toml
[cli]
session_store = "~/.config/rustygpt/session.cookies"

[web]
static_dir = "../rustygpt-web/dist"
spa_index = "../rustygpt-web/dist/index.html"
```

### `[llm]`

`Config` embeds `LLMConfiguration` from `rustygpt-shared::config::llm`. Use it to describe llama.cpp models/providers:

```toml
[llm.global_settings]
persist_stream_chunks = true

[llm.providers.default]
provider_type = "llama_cpp"
model_path = "./models/your-model.gguf"
```

See `rustygpt-shared/src/config/llm.rs` for the full schema.

## Environment variable syntax

Nested keys map to uppercase names with double underscores. Examples:

- `RUSTYGPT__SERVER__PORT=9001`
- `RUSTYGPT__SECURITY__COOKIE__SECURE=true`
- `RUSTYGPT__FEATURES__SSE_V1=true`

Booleans and numbers follow standard Rust parsing rules. Paths can be relative or absolute.
