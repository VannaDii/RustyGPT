# Streaming delivery

RustyGPT streams conversation activity to authenticated clients over Server-Sent Events (SSE). The implementation lives in
`rustygpt-server/src/handlers/streaming.rs` and is gated by `features.sse_v1`.

## Flow

{{#include ../_snippets/diagrams/sse-flow.mmd}}

Clients subscribe to `/api/stream/conversations/:conversation_id`. The route is protected by the auth middleware when
`features.auth_v1` is enabled, so callers must present a valid session cookie (the CLI handles this automatically).

## Event payloads

Events are instances of `shared::models::ConversationStreamEvent` and are encoded as JSON envelopes with `type` and `payload`
fields. See [Threaded conversations](../concepts/reasoning-dag.md) for the full list of variants.

The SSE handler assigns monotonically increasing sequence numbers per conversation. When persistence is enabled the sequence is
also stored in `rustygpt.sse_event_log`, allowing reconnecting clients to pass `Last-Event-ID` and receive any missed events
before resuming the live stream.

## Persistence and retention

Configure persistence via `[sse.persistence]` in `config.toml`:

```toml
[sse.persistence]
enabled = true
max_events_per_user = 500
prune_batch_size = 100
retention_hours = 48
```

`services::sse_persistence` stores events using the stored procedures in `scripts/pg/schema/050_sse_persistence.sql`. The pruning
logic runs after each insert to keep the table bounded.

## Backpressure handling

The in-memory queue for each conversation defaults to `channel_capacity = 128`. Configure behaviour under `[sse.backpressure]`:

- `drop_strategy = "drop_tokens"` drops assistant token events first
- `drop_strategy = "drop_tokens_and_system"` also discards system events once the queue fills
- `warn_queue_ratio` controls when a warning is logged about queue pressure

These settings keep hot conversations from exhausting memory while still delivering key state changes (presence, membership,
unread counters).

## Client responsibilities

- Reconnect with `Last-Event-ID` so the server can replay persisted events when available
- Handle `401` responses by re-running the session refresh flow (`/api/auth/refresh`); the CLI and web client do this
  automatically
- Clear typing state on `typing.update` and update unread counters when `unread.update` arrives

Use [REST API](../reference/api.md) endpoints to backfill state when the requested `Last-Event-ID` falls outside the retention
window.
