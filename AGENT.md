# AGENT.md — Codex Operating Instructions for **RustyGPT**

> **This file is normative.** If code and AGENT.md disagree, update the code or write an ADR that amends this file with rationale.

---

### Purpose

This document tells Codex (and humans) **exactly** how to implement and verify changes in the RustyGPT repository. It aligns with the **existing repository layout** and enforces: Rust 2024, zero‐unsafe, no dead code, reproducible builds, WCAG 2.2 AA + full i18n on the web UI, and a data‑driven reasoning system (DAG + RAG) backed by PostgreSQL.

---

### Repository Layout (current, enforced)

```
RUSTY_GPT/
├─ .github/
├─ .sqlx/                   # sqlx offline data (allowed for proc calls/health checks)
├─ deploy/                  # deployment assets (see “DB & Migrations”)
├─ docs/
├─ rustygpt-cli/            # CLI entrypoints & admin tools
├─ rustygpt-doc-indexer/    # documentation indexer (ingest → memory)
├─ rustygpt-server/         # Axum API + SSE streaming (backend)
├─ rustygpt-shared/         # shared models, types, error crates
├─ rustygpt-tools/          # internal utilities, scripts-as-bins
├─ rustygpt-web/            # Yew WASM frontend (WCAG 2.2 AA + i18n + RTL)
├─ scripts/                 # dev scripts (hosted by just recipes)
├─ target/
├─ .env.template
├─ .gitignore
├─ .secignore
├─ AGENT.md
├─ book.toml
├─ Cargo.toml               # workspace
├─ CHANGELOG.md
├─ CODE_OF_CONDUCT.md
├─ config.example.toml
├─ CONTRIBUTING.md
├─ docker-compose.yaml
├─ Dockerfile
├─ Justfile                 # THE interface for all ops (local & CI)
├─ LICENSE
├─ logo.png
├─ README.md
├─ rust-toolchain.toml
├─ SECURITY_AUDIT.md
├─ SECURITY.md
└─ TODO.md
```

**Do not** rename crates or introduce new top‑level folders without updating this document and adding an ADR.

---

### Prime Directives

- **Rust 2024** only; pinned in `rust-toolchain.toml` and `package.rust-version` for every crate.
- `#![forbid(unsafe_code)]` in all library/binary crates.
- `-D warnings` and `clippy::pedantic` must be clean.
- No `unwrap`/`expect` in runtime paths (allowed in tests/examples).
- **Justfile is law** — CI and local runs invoke only `just …` targets (never raw `cargo` in CI).
- **Stored procedures** only for DB writes; no embedded SQL that mutates state.
- **Accessibility** (WCAG 2.2 AA) and **i18n** (incl. RTL) required for all UI.
- **Deterministic reasoning**: DAG nodes must be testable, seeded, and reproducible.

---

### Crate Responsibilities & Allowed Dependencies

- **rustygpt-server**

  - Axum API (REST) + **SSE** streaming endpoints.
  - Auth (local accounts + OAuth Apple/GitHub), sessions/JWT, rate limiting, CORS, CSRF where applicable.
  - Talks to `rustygpt-shared` (types) and `rustygpt-tools` (helpers), invokes **stored procedures** via sqlx for reads/writes (writes go through procs).
  - **Does not** embed business logic; delegates to internal libs in `rustygpt-tools` or future `core` crates as they are introduced.

- **rustygpt-web**

  - Yew + Tailwind UI, A11y + i18n/RTL, SSE client for streaming.
  - No `.unwrap()` in UI flow; resilient error surfaces.
  - Intl formatting via browser `Intl`; locale JSON under `rustygpt-web/i18n/`.

- **rustygpt-shared**

  - Shared data models, error enums (`thiserror`), DTOs, SSE event types.
  - Absolutely no network or DB calls here.

- **rustygpt-cli**

  - Operator/admin CLIs: migrations, health checks, backfills, export/import.
  - Output modes: `--output json|table`; JSON is contract‑stable for scripting.

- **rustygpt-doc-indexer**

  - Documentation → memory ingestion (tokenize/embeddings/metadata).
  - Integrates with the memory layer using **stored procedures** and vector API.

- **rustygpt-tools**
  - Internal helpers (tracing init, config loader, SSE utilities, small adapters).

**Inter‑crate dependency rule:** `server` → (`shared`, `tools`), `web` → (`shared` runtime types for generated bindings if needed), `cli` → (`shared`, `tools`), indexer → (`shared`, `tools`). Cross‑links beyond this require an ADR.

---

### Language & Lint Gates (apply to every crate)

```rust
#![forbid(unsafe_code)]
#![deny(
    warnings,
    dead_code,
    unused,
    unused_imports,
    unused_must_use,
    unreachable_pub,
    clippy::all,
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,
    rustdoc::broken_intra_doc_links,
    rustdoc::bare_urls,
    missing_docs
)]
#![allow(clippy::module_name_repetitions)]
```

- Public API minimal; prefer `pub(crate)`.
- Add `#[must_use]` to IDs/handles/results with effects.

---

### Database, Migrations & Stored Procedures

- **PostgreSQL + pgvector** (and PostgresML when enabled) are the source of truth.
- **Writes** (insert/update/delete) go through **stored procedures** only.
- **Reads** may use views/functions for stability; large joins belong in DB functions.
- **Migrations & procs live in**: `deploy/db/` (create if missing)
  - `deploy/db/migrations/*.sql`
  - `deploy/db/procedures/*.sql`
  - `deploy/db/views/*.sql`
- `sqlx` is permitted for:
  - calling procs (`SELECT schema.proc($1, $2, …)`),
  - read‑only queries that map to views/functions,
  - offline checking with `.sqlx/` data.
- Ensure `scripts/gen-sqlx-data.sh` refreshes `.sqlx/` (called by `just db:prepare`).

**Contract Stability:** any change to a proc/view must bump a DB schema version and be captured in an ADR.

---

### Reasoning: DAG + RAG (high‑level rules)

- **Node types:** identification, slot resolution, relationship traversal, meta‑controller, pruning, generation.
- Each node:
  - Has a single purpose and pure(ish) core (deterministic, seeded).
  - Emits typed events (span fields include `conversation_id`, `node`, `stage`).
  - Is fully unit‑tested (happy, edge, ambiguity), including backpressure/timeout cases.
- **RAG** pulls linked resources when confidence gaps exist; link discovery may propose new entities via a human‑review queue.
- **Pruning** by frequency + age; reactivation on new references.

---

### Axum API & SSE (rustygpt-server)

- REST routes are versioned under `/v1`.
- SSE streams are **per conversation** and emit structured JSON events:
  - `state` (e.g., `thinking`, `retrieving`, `streaming`, `done`, `error`),
  - `token` (partial text chunks),
  - `metrics` (optional periodic telemetry for the stream),
  - `error` (terminal, user‑safe message).
- Keep‑alive with `Sse::keep_alive`; support `Last-Event-ID` resume.
- Apply tower middleware: tracing, limits, compression, timeouts, optional rate‑limit.
- All routes validate input and bound sizes; error types are typed and logged once.

---

### Web UI (rustygpt-web)

- **Accessibility**: skip link, visible focus rings, no keyboard traps, proper roles/aria, contrast ≥ 4.5:1, modal focus management, errors announced via `role="alert"`.
- **i18n**: locales `en`, `it`, `es`, and an RTL (e.g., `ar`); runtime switcher; `<html lang>` and `dir="rtl"` toggled; JSON message files with interpolation and plural forms.
- **Performance**: `wasm-opt -Oz`, initial shell ≤ 250 KiB gzipped (excluding images), route‑based code splitting where feasible.
- **Testing**: wasm‑bindgen tests for components; E2E accessibility checks via Playwright in CI.

---

### Security & Auth

- Local accounts in Postgres; OAuth providers: **Apple** and **GitHub** only.
- Session cookies (httpOnly/secure/sameSite) and/or JWTs for API; constant‑time compares.
- Input validation at all boundaries; strict CORS; security headers on responses.
- Secrets never logged; redact via tracing layer; do not store secrets in repo.

---

### Observability

- `tracing` spans at boundaries: `http.request`, `sse.stream`, `db.proc`, `dag.node`, `rag.fetch`.
- Fields: `request_id`, `conversation_id`, `node`, `stage`, `elapsed_ms` (no PII).
- Dev logs: pretty; Prod: JSON.
- Metrics (labels: node, stage):
  - counters: `reasoning_nodes_run_total`, `prunes_total`,
  - histograms: `node_latency_ms`, `sse_chunk_latency_ms`,
  - gauges: `active_streams`, `entity_cache_size`.

---

### Testing & Coverage (gates)

- **Libraries:** coverage **≥ 90%** (`cargo llvm-cov`), no regressions.
- Unit: every module (success/failure/edge).
- Integration:
  - end‑to‑end reasoning chain with seeded data,
  - SSE streaming happy + abort + resume,
  - DB procs contract tests (golden fixtures).
- Web: wasm unit + E2E a11y flows (keyboard tab order, modal traps, RTL layouts).

Example DAG test (pseudo):

```rust
#[tokio::test]
async fn reasoning_chain_is_deterministic() {
    let seeded = TestGraph::seeded(42);
    let out = seeded.run("who are the quail?").await.unwrap();
    assert_eq!(out.summary.hash(), 0x8E57_23A4);
}
```

---

### Performance Targets (initial)

- P50 reasoning (no external fetch): **< 200 ms**.
- P95 RAG hop (single external doc retrieval + embed): **< 500 ms**.
- SSE first‑byte: **< 150 ms** after request acceptance.
- Embed reindex (10k entities): **< 30 s** incremental.

---

### Justfile (canonical)

> **Never** call `cargo` directly in CI. Add/modify Just recipes instead.

```make
default: fmt lint check

fmt:          @cargo fmt --all --check
fmt:fix:      @cargo fmt --all

lint:         @cargo clippy --workspace --all-targets --all-features -D warnings

udeps:        @cargo +stable udeps --workspace --all-targets
audit:        @cargo audit
deny:         @cargo deny check
check:        @RUSTFLAGS="-Dwarnings" cargo check --workspace --all-features

build:        @cargo build --workspace --all-features
build:rel:    @cargo build --workspace --release --all-features

test:         @RUSTFLAGS="-Dwarnings" cargo test --workspace --all-features
cov:          @cargo llvm-cov --workspace --fail-under 90

# Server
dev:server:   @cargo run -p rustygpt-server
test:server:  @cargo test -p rustygpt-server -- --include-ignored

# Web
web:serve:    @trunk serve --open --config rustygpt-web/Trunk.toml
web:build:    @trunk build --release --config rustygpt-web/Trunk.toml

# DB
db:prepare:   @scripts/gen-sqlx-data.sh
db:migrate:   @scripts/db-migrate.sh deploy/db/migrations
db:procs:     @scripts/db-apply-procedures.sh deploy/db/procedures

# Reasoning
dag:test:     @cargo test -p rustygpt-tools --features dag -- --include-ignored
rag:validate: @cargo run -p rustygpt-doc-indexer -- --validate

# CI gate
ci:           @just fmt lint udeps audit deny test cov
```

If a new feature flag is added, mirror it with `test:feat[...]` recipes and call them from CI.

---

### CI (GitHub Actions)

Jobs (all must pass):

1. **fmt** → `just fmt`
2. **lint** → `just lint`
3. **udeps** → `just udeps`
4. **supply-chain** → `just audit` + `just deny`
5. **test** → `just test`
6. **coverage** → `just cov`
7. **build-release** → `just build:rel` (publish artifacts where relevant)
8. **server** → `just test:server`
9. **web** → `just web:build` + a11y E2E
10. **db** → `just db:prepare` (sqlx offline), smoke migrations/procs in ephemeral DB
11. **dag/rag** → `just dag:test` + `just rag:validate`

CI must never run raw cargo commands.

---

### Definition of Done

- All CI jobs pass. No warnings. No unsafe.
- Coverage thresholds met.
- Stored procedure contracts validated (golden tests).
- SSE flows verified (open → tokens → done; error path handled).
- Web UI a11y + i18n verified (incl. RTL rendering and tab order).
- Config documented; secrets redacted in logs.
- ADR written when changing contracts, crates, or external surfaces.

---

### Coding Conventions

- Small, cohesive modules; avoid megafiles.
- Prefer concrete types until generic requirements are proven.
- `Result<T, E>` with domain `E` in libs; `anyhow` only in bins/tests.
- Public items documented with examples; internal helpers `pub(crate)`.
- No blocking ops on async paths; if unavoidable, use `spawn_blocking` with rationale.

---

### Quick Start (dev loop)

```bash
# 1) Toolchain
rustup show               # confirms pinned toolchain
just fmt:fix

# 2) Database (ephemeral)
docker compose up -d db
just db:migrate
just db:procs
just db:prepare           # refresh .sqlx/

# 3) Server + Web
just dev:server
just web:serve
```

---

### Notes on Existing Artifacts

- `.sqlx/` exists → treat as **offline check cache** only; the contract authority is under `deploy/db/*`.
- `docker-compose.yaml` present → ensure services `db`, `server`, `web` match crate purposes.
- `book.toml`/`docs/` → keep developer docs and ADRs in sync with this AGENT.md.

---

### Final Rule

A task is **not complete** until **every gate** in this document passes. If something in this file blocks you, propose an ADR with a narrow, time‑boxed exception and a remediation plan. Otherwise, fix the code to comply.
