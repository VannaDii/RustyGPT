# Local Development

> TL;DR – keep `config.toml` in sync with your environment, use `just dev` for paired watchers, and rely on the CLI for quick
> smoke tests of authentication and streaming.

## Environment configuration

All binaries load configuration through `rustygpt-shared::config::server::Config`. The loader merges:

1. Built-in defaults selected by the active profile (Dev/Test/Prod)
2. Optional `config.toml` / `config.yaml` / `config.json`
3. Environment variables such as `RUSTYGPT__SERVER__PORT=9000`
4. CLI overrides (e.g. `cargo run -p rustygpt-server -- serve --port 9000`)

Keep secrets out of the repo—override them with environment variables or a private `config.toml`. See
[Configuration](../reference/config.md) for the full key list.

## Watcher workflows

The [`Justfile`](../../Justfile) orchestrates the common flows:

```bash
# Run server + web watchers together (uses rustygpt-tools/confuse)
just dev

# Backend only hot-reload
just watch-server

# Run fmt, check, and clippy
just check
```

`just dev` spawns two subprocesses:

- `rustygpt-server` via `cargo watch -x 'run -- serve --port 8080'`
- `rustygpt-web` via `trunk watch`

Logs stream to stdout so you can confirm when migrations finish (`db_bootstrap_*` metrics) and when the SSE hub accepts
connections.

## CLI smoke tests

The CLI binary lives at `rustygpt-cli`. Useful commands while iterating:

```bash
# Launch the server directly from the CLI crate
cargo run -p rustygpt-cli -- serve --port 8080

# Generate the OpenAPI spec
cargo run -p rustygpt-cli -- spec openapi.yaml

# Generate config skeletons
cargo run -p rustygpt-cli -- config --format toml

# Manage sessions
cargo run -p rustygpt-cli -- login
cargo run -p rustygpt-cli -- me
cargo run -p rustygpt-cli -- logout
```

CLI commands reuse the same cookie jar as the web client. Cookies are stored under `~/.config/rustygpt/session.cookies` by
default (see `[cli]` in the configuration schema).

## Debugging tips

- Enable verbose tracing: `RUST_LOG=rustygpt_server=debug,tower_http=info just run-server`
- Inspect SSE payloads: `curl -N http://127.0.0.1:8080/api/stream/conversations/<conversation-id>` (requires an authenticated
  session and `features.sse_v1 = true`)
- Verify configuration resolution: `cargo run -p rustygpt-cli -- config --format json` and inspect the generated file
- Regenerate database bindings or seed data by restarting the server; bootstrap scripts rerun automatically when the process
  starts
- Use `docker compose logs postgres` if migrations fail during bootstrap

For operational playbooks (e.g. Docker deployment or rotating secrets) see the [How-to](../howto/index.md) section.
