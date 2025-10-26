# Threaded conversations

RustyGPT models chat history as **threaded conversations** stored in PostgreSQL. Each conversation has participants, invites,
thread roots, and replies. The server exposes this structure through the DTOs in `rustygpt-shared/src/models/chat.rs`.

## Core data types

| Type | Purpose | Defined in |
| ---- | ------- | ---------- |
| `ConversationCreateRequest` | Payload for creating a new conversation. | `shared::models::chat` |
| `ThreadTreeResponse` | Depth-first snapshot of a thread including metadata for each node. | `shared::models::chat` |
| `MessageChunk` | Persisted assistant output chunk (used when streaming replies). | `shared::models::chat` |
| `ConversationStreamEvent` | Enum describing SSE events (`thread.new`, `message.delta`, etc.). | `shared::models::chat` |

Each thread is anchored by a root message (`POST /api/threads/{conversation_id}/root`). Replies hang off the tree using parent
IDs (`POST /api/messages/{message_id}/reply`). The `ThreadTreeResponse` payload includes ancestry hints so clients can render the
structure without additional queries.

## Streaming lifecycle

When `features.sse_v1 = true`, the server emits `ConversationStreamEvent` variants via `StreamHub` (`handlers/streaming.rs`). The
naming mirrors the enum variants:

- `thread.new` – new thread summary created
- `thread.activity` – updated `last_activity_at`
- `message.delta` – incremental assistant tokens (`ChatDeltaChunk`)
- `message.done` – completion marker with usage stats
- `presence.update` – user presence heartbeat
- `typing.update` – typing indicator state
- `unread.update` – unread count per thread root
- `membership.changed` – conversation membership change
- `error` – terminal failure while streaming

Events carry both the `conversation_id` and (when applicable) `root_id` so clients can scope updates precisely. SSE persistence is
optional: enable `[sse.persistence]` in configuration to record events in `rustygpt.sse_event_log` via
`services::sse_persistence` and replay them on reconnect.

## Access control

The chat service (`services::chat_service.rs`) enforces membership checks and rate limits before mutating data. Rate limit
profiles are backed by tables in `scripts/pg/schema/040_rate_limits.sql` and can be tuned via the admin API. Presence updates
mark the acting user online and emit events so all subscribers stay consistent.

For endpoint details see [REST API](../reference/api.md); for the transport-level diagram visit
[Streaming Delivery](../architecture/streaming.md).
