# Authentication

> TL;DR – RustyGPT authenticates with HttpOnly cookie sessions backed by Postgres, providing rotation, CSRF protection, and admin observability.

## Session Endpoints

All session APIs live under `/api/auth` and return JSON with the authenticated user and timestamps (`issued_at`, `expires_at`, `absolute_expires_at`).

| Method | Path                  | Description                               |
|--------|-----------------------|-------------------------------------------|
| POST   | `/api/auth/login`     | Verifies credentials and issues session   |
| POST   | `/api/auth/refresh`   | Rotates session inside idle window        |
| GET    | `/api/auth/me`        | Returns the current session summary       |
| POST   | `/api/auth/logout`    | Revokes the active session                |

Successful responses include:

- `Set-Cookie: sid=...; HttpOnly; Secure; SameSite=Lax`
- `Set-Cookie: CSRF-TOKEN=...; SameSite=Strict`
- `X-Session-Rotated: 1` when rotation occurs

Unauthorized responses set `WWW-Authenticate: session` so clients can attempt a silent refresh. See [REST API](api.md) for the broader surface area.

## Session Lifecycle

Configuration lives in `[session]` (see [Configuration](config.md)):

```toml
[session]
idle_seconds = 28800
absolute_seconds = 604800
session_cookie_name = "sid"
csrf_cookie_name = "CSRF-TOKEN"
max_sessions_per_user = 5
```

- **Sliding rotation** – any authenticated request inside the idle window extends the session and may rotate cookies.
- **Absolute lifetime** – once `absolute_seconds` elapse, `/api/auth/refresh` returns `401 session_expired`.
- **Privilege changes** – stored procedures mark rows with `requires_rotation = TRUE`, forcing cookie refresh.
- **Max concurrent sessions** – newest sessions evict the oldest when the cap is exceeded.

Stored procedures `rustygpt.sp_auth_login` and `sp_auth_refresh` encapsulate Argon2id verification and rotation logic.

## CSRF Enforcement

Non-GET requests outside `/api/auth/*` must include:

- `X-CSRF-Token` header
- `CSRF-TOKEN` cookie

Failures return `403 Forbidden`. Exemptions: GET/HEAD/OPTIONS, SSE streaming endpoints, and `/api/auth/*`.

## Client Guidance

- Persist both cookies; the session id stays HttpOnly while the CSRF token is user-accessible.
- On `401` with `WWW-Authenticate: session`, call `/api/auth/refresh` once before retrying.
- On logout, delete cookies and redirect to `/login`.

CLI smoke tests:

```bash
cargo run -p rustygpt-cli -- auth login
cargo run -p rustygpt-cli -- auth me
```

## Observability

Prometheus metrics:

- `rustygpt_auth_logins_total{result=...}`
- `rustygpt_auth_session_rotations_total{reason=...}`
- `rustygpt_auth_active_sessions{user_role}`

Dashboards live in `deploy/grafana/auth.json`. Integrate rotation procedures with [Rotate Secrets](../howto/rotate-secrets.md) when updating signing keys.

## Cutover Runbook

1. Apply migrations (`010_auth.sql`, `034_limits.sql`, `040_rate_limits.sql`, `050_sse_persistence.sql`).
2. Deploy backend, then web + CLI to pick up DTO changes.
3. Force rotations with `SELECT rustygpt.sp_auth_mark_rotation(id, 'cutover') FROM rustygpt.users;`.
4. Smoke test via CLI and web UI.

If issues arise, follow the incident response guidance in [Docker Deploy](../howto/docker-deploy.md).
