# Rate-Limit Administration

RustyGPT exposes dynamic per-route throttling managed via PostgreSQL stored procedures and an in-memory cache inside `RateLimitState`. This document explains the data model, HTTP API, and runtime behaviour.

## Data Model

* `rustygpt.rate_limit_profiles` – GCRA configuration payloads (`algorithm`, `params` JSON, optional description).
* `rustygpt.rate_limit_assignments` – Maps `{method, path_pattern}` pairs to profiles.
* `rustygpt.message_rate_limits` – Existing per-conversation throttling table reused by the chat service.

Profiles and assignments are created via stored procedures (`sp_limits_*`) invoked by the admin HTTP endpoints. After every change we call `RateLimitState::reload_from_db`, re-hydrating the in-process cache and updating the Prometheus gauges:

* `rustygpt_limits_profiles{count}`
* `rustygpt_limits_assignments{count}`

## HTTP Endpoints

The admin API is gated by `config.rate_limits.admin_api_enabled` **and** requires an authenticated user with the `admin` role.

```
GET    /api/admin/limits/profiles
POST   /api/admin/limits/profiles
PUT    /api/admin/limits/profiles/:id
DELETE /api/admin/limits/profiles/:id

GET    /api/admin/limits/assignments
POST   /api/admin/limits/assignments
DELETE /api/admin/limits/assignments/:id
```

Payloads align with the shared DTOs in `rustygpt-shared/src/models/limits.rs`.

### Example – Create a profile

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

### Example – Assign to a route

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

* Keys are normalised to uppercase method + path pattern (e.g. `POST /api/messages/:id/reply`).
* Patterns support `*` suffix (`/api/admin/*`) and colon parameters (`:id`).
* Auth endpoints fall back to a dedicated `auth_strategy` so that login traffic cannot exhaust the global bucket.
* When no assignment matches, the default strategy uses `[default_rps, burst]` from configuration.

Denials return `429` with:

```
{
  "code": "rate_limit_exceeded",
  "message": "...",
  "details": { "retry_after_seconds": 3 }
}
```

The middleware also emits standard headers: `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`, and the custom `X-RateLimit-Profile`.

See `deploy/grafana/limits.json` for an example dashboard that tracks throttled requests and profile counts in Prometheus.
