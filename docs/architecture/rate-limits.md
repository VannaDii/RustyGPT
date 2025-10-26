# Rate-limit architecture

RustyGPT enforces per-route throttling using a leaky-bucket strategy implemented in `middleware::rate_limit`. Configuration
comes from two tables managed by stored procedures in `scripts/pg/procs/034_limits.sql`.

## Data model

- `rustygpt.rate_limit_profiles` – named profiles containing algorithm + JSON parameters (currently `gcra` style with
  `requests_per_second` / `burst` options).
- `rustygpt.rate_limit_assignments` – maps HTTP method + path pattern to a profile.
- `rustygpt.message_rate_limits` – per-user, per-conversation state used by `sp_user_can_post` to throttle message posting.

`RateLimitState::reload_from_db` loads profiles and assignments into memory. The admin API under `/api/admin/limits/*` can
create, update, or delete records at runtime; after each change the state refreshes automatically.

## Matching logic

`enforce_rate_limits` computes a cache key as `"{METHOD} {path}"` and finds the first matching pattern. Supported patterns:

- Exact path matches (`/api/messages/{id}/reply` becomes `/api/messages/:id/reply` in the database)
- `*` suffix for prefixes (e.g. `/api/admin/*`)

If no assignment matches, the middleware falls back to the default strategy derived from `[rate_limits.default_rps]` and
`[rate_limits.burst]`. Login routes (`/api/auth/login`) use the dedicated `auth_login_per_ip_per_min` limiter.

## Metrics and headers

When a request is evaluated the middleware records:

- `http_rate_limit_requests_total{profile,result}` – allowed vs denied counts
- `http_rate_limit_remaining{profile}` – remaining tokens after the decision
- `http_rate_limit_reset_seconds{profile}` – seconds until the bucket refills
- `rustygpt_limits_profiles` / `rustygpt_limits_assignments` – gauges updated on reload

Responses include the standard headers `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`, and
`X-RateLimit-Profile` so clients can react accordingly.

## Admin API payloads

All admin payloads live in `shared::models::limits`:

- `CreateRateLimitProfileRequest`
- `UpdateRateLimitProfileRequest`
- `AssignRateLimitRequest`
- `RateLimitProfile` / `RateLimitAssignment`

These endpoints require an authenticated session with the `admin` role (`handlers/admin_limits.rs`).

## Conversation posting limits

`ChatService::post_root_message` and `ChatService::reply_message` call `sp_user_can_post`, which enforces a GCRA window per
`(user_id, conversation_id)` using `rustygpt.message_rate_limits`. Tweak the `conversation.post` profile via SQL or the admin
API to adjust posting cadence.
