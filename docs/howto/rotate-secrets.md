# Rotate secrets

Use this runbook to update credentials (database passwords, OAuth secrets, session keys) while keeping RustyGPT online.

## Preparation

1. Inventory the secrets in use (e.g. `DATABASE_URL`, `GITHUB_CLIENT_SECRET`, config entries under `[security.cookie]`).
2. Update your secret manager or environment files with new values, but do not apply them yet.
3. Coordinate a maintenance window if session cookie rotation is expected to log users out.

## Rotation steps

1. **Stage** – write new values to your secret store or `.env` file.
2. **Deploy** – restart the server with updated environment variables/config (`docker compose restart backend` or rolling
   restart in your orchestrator). The bootstrap runner is idempotent, so restarting is safe.
3. **Verify** – run smoke tests:
   ```bash
   cargo run -p rustygpt-cli -- login
   cargo run -p rustygpt-cli -- me
   curl -sSf http://HOST:8080/readyz
   ```
4. **Cleanup** – remove old secrets from the manager and audit logs for unexpected errors.

Session cookies are independent of database passwords or OAuth secrets. If you change `[security.cookie]` settings (e.g. enable
`secure` or change `session_cookie_name`), expect users to sign in again.

## Observability

- Watch application logs for `SessionService` warnings.
- Confirm `http_rate_limit_requests_total` continues to increment after the restart.
- Verify the CLI can still access streaming endpoints (`cargo run -p rustygpt-cli -- follow --root <id>`).

## Incident response

If a rotation fails:

1. Roll back to the previous secret values and restart the server.
2. Capture logs around the failure (authentication errors, database connection failures, etc.).
3. File an issue or ADR documenting the change and follow-up actions.
