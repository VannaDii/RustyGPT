# Rate-Limit Architecture

> TL;DR – RustyGPT enforces per-route throttling through Postgres-backed profiles cached in memory, exposing admin APIs for live tuning.

## Data Model

- `rustygpt.rate_limit_profiles` – GCRA parameters (`algorithm`, `params` JSON, optional description).
- `rustygpt.rate_limit_assignments` – Maps `{method, path_pattern}` pairs to profiles.
- `rustygpt.message_rate_limits` – Conversation-scoped throttling reused by the chat service.

Stored procedures (`sp_limits_*`) manage CRUD operations. After each change, call `RateLimitState::reload_from_db` to refresh the cache.

## Admin API

| Method | Path                                   | Description                    |
|--------|----------------------------------------|--------------------------------|
| GET    | `/api/admin/limits/profiles`           | List profiles                  |
| POST   | `/api/admin/limits/profiles`           | Create a profile               |
| PUT    | `/api/admin/limits/profiles/:id`       | Update a profile               |
| DELETE | `/api/admin/limits/profiles/:id`       | Delete a profile               |
| GET    | `/api/admin/limits/assignments`        | List assignments               |
| POST   | `/api/admin/limits/assignments`        | Assign profile to a route      |
| DELETE | `/api/admin/limits/assignments/:id`    | Remove an assignment           |

Payloads align with DTOs in `rustygpt-shared/src/models/limits.rs` and are documented in [REST API](../reference/api.md).

### Create a Profile

```http
POST /api/admin/limits/profiles
Content-Type: application/json

{
  "name": "messages.fast-track",
  "algorithm": "gcra",
  "params": { "requests_per_second": 20, "burst": 40 },
  "description": "Lenient burst bucket for trusted integrations"
}
```

### Assign to a Route

```http
POST /api/admin/limits/assignments
Content-Type: application/json

{
  "profile_id": "f26d1c7c-621c-4e9a-815c-21ed6f63c1db",
  "method": "POST",
  "path": "/api/messages/:id/reply"
}
```

## Runtime Matching

- Keys normalise to uppercase method + path pattern (`POST /api/messages/:id/reply`).
- Patterns support `*` suffix (`/api/admin/*`) and colon parameters (`:id`).
- Auth endpoints fallback to `auth_strategy` so login traffic cannot exhaust the global bucket.
- Defaults use `[default_rps, default_burst]` from configuration.

Denials return:

```json
{
  "code": "rate_limit_exceeded",
  "message": "...",
  "details": { "retry_after_seconds": 3 }
}
```

Headers include `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`, and `X-RateLimit-Profile`.

## Observability

Prometheus metrics:

- `rustygpt_limits_profiles{count}`
- `rustygpt_limits_assignments{count}`
- `rustygpt_rate_limit_denials_total`

Dashboards live in `deploy/grafana/limits.json`. Roll out configuration changes using [Docker Deploy](../howto/docker-deploy.md) for production and validate with `lychee` or smoke tests in [Local Development](../guide/local-dev.md).
