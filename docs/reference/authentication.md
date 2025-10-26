# Authentication

RustyGPT uses cookie-based sessions backed by PostgreSQL. Session management lives in `rustygpt-server/src/auth/session.rs` and
is exposed through `/api/auth/*` handlers when `features.auth_v1 = true`.

## Session lifecycle

1. **Setup** – `POST /api/setup` hashes the supplied password and inserts the first user (admin + member roles). Further calls
   are rejected.
2. **Login** – `POST /api/auth/login` verifies credentials via `sp_auth_login`. Successful responses include:
   - `Set-Cookie: SESSION_ID=...; HttpOnly; Secure?; SameSite=Lax`
   - `Set-Cookie: CSRF-TOKEN=...; SameSite=Strict`
   - `X-Session-Rotated: 1`
3. **Authenticated requests** – non-GET operations must include the CSRF header `X-CSRF-TOKEN` with the cookie value. The web
   client (`rustygpt-web/src/api.rs`) and CLI handle this automatically.
4. **Refresh** – `POST /api/auth/refresh` rotates cookies inside the idle window (default 8 hours). If either idle or absolute
   expiry is exceeded, the call returns `401 session_expired`.
5. **Logout** – `POST /api/auth/logout` clears the session and CSRF cookies.

Sessions are stored in `rustygpt.user_sessions`. The idle and absolute windows come from `[session]` in configuration. When
`max_sessions_per_user` is set the newest session evicts the oldest via `sp_auth_login`.

## Cookie configuration

`config.toml` controls cookie behaviour:

```toml
[session]
idle_seconds = 28800
absolute_seconds = 604800
session_cookie_name = "SESSION_ID"
csrf_cookie_name = "CSRF-TOKEN"
max_sessions_per_user = 5

[security.cookie]
domain = ""
secure = false
same_site = "lax"

[security.csrf]
cookie_name = "CSRF-TOKEN"
header_name = "X-CSRF-TOKEN"
enabled = true
```

Adjust `security.cookie.secure` and `security.cookie.domain` for production deployments. When `security.csrf.enabled = false`
the middleware skips header validation (useful for service-to-service calls but not recommended for browsers).

## CLI workflow

The CLI wraps the same endpoints:

```bash
cargo run -p rustygpt-cli -- login
cargo run -p rustygpt-cli -- me
cargo run -p rustygpt-cli -- logout
```

Cookies are stored at `~/.config/rustygpt/session.cookies` by default (see `[cli.session_store]`). The `follow` and `chat`
commands automatically attach the CSRF header when present.

## Observability

Authentication currently relies on logs for troubleshooting. Set `RUST_LOG=rustygpt_server=debug` to trace session decisions
(`SessionService::authenticate`, `SessionService::refresh_session`). Prometheus metrics for auth flows are not yet implemented.
