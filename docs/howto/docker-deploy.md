# Docker deploy

This guide covers building the RustyGPT container image and running it alongside PostgreSQL with Docker Compose.

## Build the image

The repository ships a multi-stage [`Dockerfile`](../../Dockerfile) that builds the workspace and bundles the server binary plus
static assets:

```bash
docker build -t rustygpt/server:latest -f Dockerfile .
```

Set `BUILD_PROFILE=release` to compile with optimisations. The final image exposes the server on port `8080`.

## Compose stack

[`docker-compose.yaml`](../../docker-compose.yaml) defines two services:

- `backend` – builds from the Dockerfile (target `runtime`). Environment variables include `DATABASE_URL`, OAuth credentials, and
  feature toggles. Update them to match your deployment.
- `postgres` – `postgres:17-alpine` with credentials matching `config.example.toml`.

Bring the stack up:

```bash
docker compose up --build
```

The compose file mounts `./.data/postgres` for database storage and `./.data/postgres-init` for init scripts. To reuse the
workspace schema, copy the contents of `scripts/pg` into that directory before the first run:

```bash
mkdir -p .data/postgres-init
cp -r scripts/pg/* .data/postgres-init/
```

Alternatively, rely on the server’s bootstrap runner by exposing the same directory inside the backend container and pointing
`[db].bootstrap_path` at it.

## Configuration and secrets

- Copy `config.example.toml` to a volume or bake it into the image and set `RUSTYGPT__CONFIG` variables as needed.
- Provide OAuth credentials (`GITHUB_*`, `APPLE_*`) if you plan to use those flows; otherwise the endpoints return placeholder
  URLs.
- Set `features.auth_v1`, `features.sse_v1`, and `features.well_known` to `true` via environment variables or the config file.
- If running behind TLS terminate HTTPS at the reverse proxy and set `server.public_base_url` to the external URL.

## Post-deployment checks

1. Hit `http://HOST:8080/healthz` and `http://HOST:8080/readyz` until both return `200`.
2. POST to `/api/setup` to create the initial admin account.
3. Use the CLI container (or a local build) to log in: `docker compose exec backend rustygpt login`.
4. Visit `/metrics` and confirm counters increment when making requests.
5. If using the web UI, serve the `rustygpt-web` build either from the same container (set `[web.static_dir]`) or via a separate
   static host.

## Rollback

Tag each release in your registry. To roll back:

```bash
docker compose pull backend:previous-tag
docker compose up -d backend
```

The PostgreSQL data directory is persisted on disk so sessions and conversations remain intact.
