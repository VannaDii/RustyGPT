# REST API

> TL;DR – RustyGPT exposes authenticated REST and SSE endpoints for conversations, admin tooling, and health checks; prefer relative URLs from the web or CLI clients.

## Authentication

All endpoints require session cookies issued by `/api/auth/login`. See [Configuration](config.md) for cookie settings and rotation windows. Clients must include `X-CSRF-Token` for non-GET requests outside `/api/auth/*`.

## Core Endpoints

| Method | Path                        | Purpose                                   |
|--------|-----------------------------|-------------------------------------------|
| GET    | `/api/health`               | Liveness probe                            |
| GET    | `/api/chat/stream`          | SSE stream for live token output          |
| POST   | `/api/chat`                 | Submit a prompt to the reasoning engine   |
| GET    | `/api/conversations/:id`    | Fetch conversation messages and metadata  |
| POST   | `/api/conversations/:id/reply` | Continue a conversation                |
| GET    | `/api/tools`                | List available deterministic tools        |

### Authentication Service

| Method | Path                 | Description                               |
|--------|----------------------|-------------------------------------------|
| POST   | `/api/auth/login`    | Verify credentials and issue session      |
| POST   | `/api/auth/refresh`  | Rotate cookies within the idle window     |
| GET    | `/api/auth/me`       | Return authenticated profile              |
| POST   | `/api/auth/logout`   | Revoke the current session                |

Extended flows are documented in [Rotate Secrets](../howto/rotate-secrets.md).

## Admin & Operations

Rate-limit endpoints are guarded by the `admin` role and `config.rate_limits.admin_api_enabled = true`.

| Method | Path                                      | Purpose                             |
|--------|-------------------------------------------|-------------------------------------|
| GET    | `/api/admin/limits/profiles`              | List rate-limit profiles            |
| POST   | `/api/admin/limits/profiles`              | Create a profile                    |
| PUT    | `/api/admin/limits/profiles/:id`          | Update a profile                    |
| DELETE | `/api/admin/limits/profiles/:id`          | Delete a profile                    |
| GET    | `/api/admin/limits/assignments`           | List route assignments              |
| POST   | `/api/admin/limits/assignments`           | Assign profile to route             |
| DELETE | `/api/admin/limits/assignments/:id`       | Remove an assignment                |

Refer to [Streaming Delivery](../architecture/streaming.md) for SSE message structure that accompanies these endpoints.

## Error Model

Errors follow the envelope:

```json
{
  "code": "rate_limit_exceeded",
  "message": "Human readable explanation",
  "details": {
    "retry_after_seconds": 3
  }
}
```

Each response sets `trace_id` headers for correlation with logs. Retries should respect `Retry-After` where present.

## OpenAPI

`rustygpt-server` exposes OpenAPI JSON/YAML via `/api/openapi.json` and `/api/openapi.yaml`. Import the schema into clients to generate strongly typed SDKs. Keep the spec in sync with code changes and surface changelog entries in [Release Notes](../changelog/index.md).
