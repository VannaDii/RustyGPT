# Rotate Secrets

> TL;DR – Rotate API keys and credentials without downtime by staging new values, triggering config reloads, and confirming session continuity.

## Preparation

1. Inventory secrets stored in Vault or your secret manager.
2. Update `config.toml` placeholders and export new values as environment variables.
3. Schedule a rotation window and notify stakeholders.

Document default keys in [Configuration](../reference/config.md) for posterity, but never commit actual secrets.

## Rotation Steps

1. **Stage** – Write the new secret to your secret manager and ensure the deployment pipeline can read it.
2. **Deploy** – Redeploy the service with refreshed environment variables. For Kubernetes, restart pods with `kubectl rollout restart deployment/rustygpt-server`.
3. **Verify** – Run smoke tests:

```bash
cargo run -p rustygpt-cli -- auth me
curl -sSf http://localhost:8080/api/health
```

Sessions should remain valid because cookies are independent of backend secrets. If rotating session signing keys, drain sessions gracefully using the guidance in [REST API](../reference/api.md).

## Observability

- Monitor `rustygpt_auth_session_rotations_total`.
- Check error logs for failed decryptions or outbound integration issues.
- Confirm SSE streams reconnect cleanly as described in [Streaming Delivery](../architecture/streaming.md).

## Incident Response

If a rotation fails:

1. Roll back to the previous secret value.
2. Capture logs and metrics for the incident timeline.
3. File an ADR per the template if process changes are needed.

## Automation Ideas

1. Integrate with `just docs-index` to ensure documentation references stay current.
2. Add alerts when secrets near expiry windows to prompt proactive rotations.
