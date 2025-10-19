# Quickstart

> TL;DR – Install the Rust 2024 toolchain, run the RustyGPT server and web UI locally, and verify streaming chat flows with seeded demo data.

## Prerequisites

- Rust 1.81+ with `rustup` configured for the 2024 edition
- `cargo` and `just` on your `PATH`
- PostgreSQL 15+ accessible at `postgres://localhost`

Install workspace dependencies:

```bash
rustup default stable
cargo fetch --workspace
just install
```

For a deeper dive into the dev environment, see [Local Development](local-dev.md).

## Start the Backend

1. Apply the SQL migrations in `deploy/postgres/migrations` using your preferred tool.
2. Launch the server with configuration pointing at your database:

```bash
just run-server
```

Health checks respond at `http://localhost:8080/api/health`. The chat SSE endpoint is documented under [REST API](../reference/api.md).

## Build the Web Client

Open a new terminal and run:

```bash
cd rustygpt-web
trunk serve
```

The Yew client proxies API calls to the backend. Log in using seeded credentials from `deploy/dev-seed.sql`. Confirm that streaming responses appear in the conversation view; troubleshooting tips live in [Streaming Delivery](../architecture/streaming.md).

## Seed Demo Data

Load the included demo dataset to populate agents and sample conversations:

```bash
psql $DATABASE_URL -f deploy/dev-seed.sql
```

After seeding, visit the dashboard and run a few prompts to validate token usage counters. Follow the operational checklist in [Docker Deploy](../howto/docker-deploy.md) before promoting to shared environments.
