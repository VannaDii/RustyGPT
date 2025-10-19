# RustyGPT Authentication Overview

RustyGPT now authenticates users with opaque HttpOnly cookies backed by the `rustygpt.user_sessions` table. This document describes the HTTP surface area, session lifecycle rules, and client integration expectations.

## HTTP Endpoints

All session endpoints live under `/api/auth` and return JSON responses that embed the authenticated user plus session timestamps (`issued_at`, `expires_at`, `absolute_expires_at`).

| Method | Path              | Description                                                |
|--------|-------------------|------------------------------------------------------------|
| POST   | `/api/auth/login` | Verifies email/password, issues a fresh session + CSRF     |
| POST   | `/api/auth/refresh` | Rotates an existing session when still inside lifetime  |
| GET    | `/api/auth/me`    | Returns the authenticated user and current session summary |
| POST   | `/api/auth/logout` | Revokes the current session and clears cookies            |

Successful responses include:

* `Set-Cookie: sid=...; HttpOnly; Secure; SameSite=Lax` (session id)
* `Set-Cookie: CSRF-TOKEN=...; SameSite=Strict` (client-readable CSRF secret)
* `X-Session-Rotated: 1` when a new cookie replaces the previous one

Unauthorized responses set `WWW-Authenticate: session` so web/CLI clients can attempt a silent refresh before forcing a sign-in.

## Session Lifecycle

Session lifetimes are controlled by the `[session]` block in `config.toml`:

```toml
[session]
idle_seconds = 28800          # 8 hours sliding window
absolute_seconds = 604800     # 7 days hard cap
session_cookie_name = "sid"
csrf_cookie_name = "CSRF-TOKEN"
max_sessions_per_user = 5
```

* **Sliding idle rotation** – any authenticated request inside the idle window extends the session. When a request arrives inside the "rotation threshold" the middleware replaces the cookie and sends `X-Session-Rotated: 1`.
* **Absolute lifetime** – once `absolute_seconds` elapse the session is revoked and `/api/auth/refresh` returns `401 session_expired`.
* **Privilege changes** – stored procedures can mark existing rows with `requires_rotation = TRUE`, forcing the next request to receive a rotated cookie that reflects the updated role snapshot.
* **Max concurrent sessions** – if `max_sessions_per_user` is non-zero the newest session evicts the oldest active record.

The `rustygpt.sp_auth_login` / `sp_auth_refresh` stored procedures encapsulate the Argon2id verification, session insertion, and rotation logic. Metadata (user agent, IP, optional fingerprint header) is persisted in `client_meta` for anomaly detection.

## CSRF Enforcement

All non-GET requests outside `/api/auth/*` must provide a matching `X-CSRF-Token` header and `CSRF-TOKEN` cookie. The token is issued on login and refresh and stored in-memory by the web client. The middleware exempts:

* GET/HEAD/OPTIONS
* SSE streaming endpoints
* `/api/auth/*`

Failed checks return `403 forbidden` with an explanatory message.

## Client Guidance

* Send credentials to `/api/auth/login` and persist both returned cookies. The session id is HttpOnly and managed by the browser/reqwest jar, while the CSRF token must be retained for future state-changing POST/PUT/DELETE calls.
* Use `/api/auth/me` to bootstrap client state without rotating the session; it returns the authenticated profile and session timestamps.
* On any `401` with `WWW-Authenticate: session`, call `/api/auth/refresh` once, then retry the original request.
* On logout, discard both cookies and redirect to `/login`.

## Observability

New Prometheus metrics are emitted:

* `rustygpt_auth_logins_total{result=success|invalid|disabled}`
* `rustygpt_auth_session_rotations_total{reason=idle|privilege|suspicious}`
* `rustygpt_auth_active_sessions{user_role}`

Grafana dashboards (see `deploy/grafana/auth.json`) visualize login success rates, rotation volume, and 401 trends.

## Deployment Order

To roll out cookie auth safely:

1. Apply the PostgreSQL migrations in order: `010_auth.sql`, `034_limits.sql`, `040_rate_limits.sql`, `050_sse_persistence.sql`.
2. Deploy the backend (`rustygpt-server`) so the new endpoints are available.
3. Deploy the shared crate followed by the web client and CLI to pick up the new DTOs and cookie handling.

## Session Cutover Runbook

1. **Preflight** – confirm `config.security.cookie.secure = true` in production, and verify `features.auth_v1` is enabled in both application configs and environment variables.
2. **Drain legacy sessions** – set a maintenance banner and reduce the idle timeout via `SESSION_IDLE_SECONDS` so stale device-code sessions expire quickly. Monitor `rustygpt_auth_active_sessions` gauge in Grafana.
3. **Roll deploy** – apply migrations, roll the new server build, then redeploy web + CLI artifacts. Validate `/api/auth/me` responds with the current profile and session timestamps.
4. **Force rotation** – run `SELECT rustygpt.sp_auth_mark_rotation(id, 'cutover') FROM rustygpt.users;` to ensure logged-in clients receive refreshed cookies with the updated role snapshot.
5. **Smoke test** – use the CLI: `rustygpt session login` → `rustygpt session me`. For the web app, confirm the login flow redirects to the dashboard and that the header shows the authenticated user.
6. **Post deployment** – watch `rustygpt_auth_session_rotations_total` and Grafana dashboards from `deploy/grafana/auth.json`. Investigate any spikes in `session_conflict` responses or CSRF errors.

### Prometheus & Grafana Validation

- Ensure Prometheus is scraping `/metrics` after deploy. `promtool query instant http://prometheus/api/v1/query 'sum(rustygpt_auth_logins_total)'` is a quick liveness check.
- Import `deploy/grafana/auth.json` during the rollout to visualise rotation and rejection trends. Keep the JSON in sync with metric names when adding new counters.
