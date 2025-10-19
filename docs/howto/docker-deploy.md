# Docker Deploy

> TL;DR – Build RustyGPT images with Cargo, push to your registry, and deploy via Docker Compose or Kubernetes with environment-specific configuration.

## Build Containers

Use the provided multi-stage Dockerfile:

```bash
docker build -t registry.example.com/rustygpt/server:latest -f Dockerfile .
```

Set `BUILD_PROFILE=release` for production builds. Verify the binary boots by running `docker run --rm -p 8080:8080 registry.example.com/rustygpt/server:latest`.

## Provision Dependencies

RustyGPT requires:

- PostgreSQL 15+ with the schema in `deploy/postgres/migrations`.
- Optional Redis for caching (if enabled in config).
- HTTPS termination (e.g., Traefik or Nginx) that forwards SSE headers transparently.

Align credentials with the secrets workflow in [Rotate Secrets](rotate-secrets.md).

## Compose Deployment

```bash
docker compose -f docker-compose.yaml up -d
```

Review `docker-compose.yaml` for service names and volume mounts. Override environment settings via `.env` or Compose overrides when promoting to staging or production.

## Post-Deployment Checks

1. Hit `/api/health` until it returns `200`.
2. Tail logs for `rustygpt_server` and ensure migrations succeeded.
3. Exercise SSE streaming with `curl -N` as described in [Streaming Delivery](../architecture/streaming.md).
4. Inspect Prometheus metrics at `/metrics`.

## Rollback

- Keep the previous tag available in your registry.
- Downgrade via `docker compose pull` and `docker compose up -d`.
- Validate user sessions remain valid by following [Local Development](../guide/local-dev.md) steps to test login flows in a sandbox environment first.
