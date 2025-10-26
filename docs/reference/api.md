# REST API

All endpoints are served under `/api` unless noted otherwise. Session cookies and CSRF headers are required for non-authenticated
GET requests when `features.auth_v1` is enabled. The OpenAPI schema is generated from `rustygpt-server/src/openapi.rs` and can
be exported with `cargo run -p rustygpt-cli -- spec`.

## Setup

| Method | Path | Description |
| ------ | ---- | ----------- |
| GET | `/api/setup` | Returns `{ "is_setup": bool }` by calling `is_setup()` in PostgreSQL. |
| POST | `/api/setup` | Creates the first administrator account (see `scripts/pg/procs/010_auth.sql::init_setup`). Subsequent calls return `400`. |

## Authentication

| Method | Path | Description |
| ------ | ---- | ----------- |
| POST | `/api/auth/login` | Email/password login. Returns `LoginResponse` with session + CSRF cookies. |
| POST | `/api/auth/logout` | Revokes the current session. Requires CSRF header. |
| POST | `/api/auth/refresh` | Rotates session cookies inside the idle window. |
| GET | `/api/auth/me` | Returns `MeResponse` (requires authenticated session). |

### OAuth helpers

Handlers in `handlers/github_auth.rs` and `handlers/apple_auth.rs` expose optional OAuth flows when credentials are present:

| Method | Path | Notes |
| ------ | ---- | ----- |
| GET | `/api/oauth/github` | Returns an authorization URL based on `GITHUB_*` environment variables. |
| GET | `/api/oauth/github/callback` | Exchanges the code for a session via `ProductionOAuthService`. |
| POST | `/api/oauth/github/manual` | Developer helper that accepts a raw auth code. |
| GET | `/api/oauth/apple` | Same as GitHub but for Apple. |
| GET | `/api/oauth/apple/callback` | Callback handler. |
| POST | `/api/oauth/apple/manual` | Manual exchange helper. |

## Conversations & membership

Routes implemented in `handlers/conversations.rs`:

| Method | Path | Description |
| ------ | ---- | ----------- |
| POST | `/api/conversations` | Create a new conversation. |
| POST | `/api/conversations/{conversation_id}/participants` | Invite/add a participant. Emits membership + presence SSE events. |
| DELETE | `/api/conversations/{conversation_id}/participants/{user_id}` | Remove a participant. |
| POST | `/api/conversations/{conversation_id}/invites` | Create an invite token. |
| POST | `/api/invites/accept` | Accept an invite token. |
| POST | `/api/invites/{token}/revoke` | Revoke an invite token. |
| GET | `/api/conversations/{conversation_id}/threads` | List thread summaries (supports `after` + `limit` query params). |
| GET | `/api/conversations/{conversation_id}/unread` | Return unread counts per thread. |

## Threads & messages

Routes from `handlers/threads.rs`:

| Method | Path | Description |
| ------ | ---- | ----------- |
| GET | `/api/threads/{root_id}/tree` | Depth-first thread slice (`cursor_path` + `limit` optional). |
| POST | `/api/threads/{conversation_id}/root` | Create a new thread root. Triggers assistant streaming when role = `assistant`. |
| POST | `/api/messages/{parent_id}/reply` | Reply to an existing message. |
| GET | `/api/messages/{message_id}/chunks` | Retrieve persisted assistant chunks. |
| POST | `/api/threads/{root_id}/read` | Mark thread as read (`MarkThreadReadRequest`). |
| POST | `/api/messages/{message_id}/delete` | Soft-delete a message. |
| POST | `/api/messages/{message_id}/restore` | Restore a previously deleted message. |
| POST | `/api/messages/{message_id}/edit` | Replace message content. |
| POST | `/api/typing` | Set typing state (`TypingRequest`). |
| POST | `/api/presence/heartbeat` | Update presence heartbeat. |

## Streaming

| Method | Path | Description |
| ------ | ---- | ----------- |
| GET | `/api/stream/conversations/{conversation_id}` | SSE endpoint producing `ConversationStreamEvent` values. Requires session cookie and (optionally) `Last-Event-ID`. |

## Copilot-compatible endpoints

These helpers live in `handlers/copilot.rs` and provide simple echo responses for integration tests:

| Method | Path | Description |
| ------ | ---- | ----------- |
| GET | `/v1/models` | Returns `ModelsResponse` with two static models (`gpt-4`, `gpt-3.5`). |
| POST | `/v1/chat/completions` | Echoes provided messages as assistant responses (`ChatCompletionResponse`). |

## Admin rate limit API

Available when `features.auth_v1 = true` and `rate_limits.admin_api_enabled = true` (`handlers/admin_limits.rs`):

| Method | Path | Description |
| ------ | ---- | ----------- |
| GET | `/api/admin/limits/profiles` | List profiles. |
| POST | `/api/admin/limits/profiles` | Create a profile. |
| PUT | `/api/admin/limits/profiles/{id}` | Update profile parameters/description. |
| DELETE | `/api/admin/limits/profiles/{id}` | Delete a profile (fails if still assigned). |
| GET | `/api/admin/limits/assignments` | List route assignments. |
| POST | `/api/admin/limits/assignments` | Assign a profile to a route. |
| DELETE | `/api/admin/limits/assignments/{id}` | Remove an assignment. |

## Health and observability

Outside of the `/api` prefix, the server exposes:

| Method | Path | Description |
| ------ | ---- | ----------- |
| GET | `/healthz` | Liveness probe. |
| GET | `/readyz` | Readiness probe (verifies PostgreSQL bootstrap). |
| GET | `/metrics` | Prometheus metrics via `metrics-exporter-prometheus`. |
| GET | `/.well-known/{path}` | Served when `features.well_known = true`; entries configured via `[well_known.entries]`. |
| GET | `/openapi.json` / `/openapi.yaml` | Generated OpenAPI spec. |

Refer to [Authentication](authentication.md) for cookie details and [Configuration](config.md) for the relevant keys.
