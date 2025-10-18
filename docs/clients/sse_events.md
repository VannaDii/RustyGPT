# SSE Event Semantics & Replay Contract

RustyGPT streams conversation updates over Server-Sent Events. Persistence now covers presence, typing, unread counters, and membership changes so reconnecting clients can rebuild transient state.

## Event Types

All events share the shape `{ "type": "...", "payload": { ... } }` as defined in `shared::models::ConversationStreamEvent`.

| Event name           | Description                                      |
|----------------------|--------------------------------------------------|
| `thread.new`         | New thread summary                               |
| `thread.activity`    | Updated activity timestamp for a thread          |
| `message.delta`      | Streaming chunk for an in-flight assistant reply |
| `message.done`       | Completion marker + usage stats                  |
| `presence.update`    | User presence status update                      |
| `typing.update`      | Typing indicator with expiry timestamp           |
| `unread.update`      | Unread count snapshot for a thread               |
| `membership.changed` | Conversation membership mutation                 |
| `error`              | Terminal errors propagated to the client         |

For events scoped to a particular thread, the `root_id` is stored in the persistence log to keep replay ordering stable.

## Persistence Window

`rustygpt.sse_event_log` stores a rolling window. Age-based pruning is governed by `sse.persistence.retention_hours` (clamped to 24–72 h) while `sse.persistence.max_events_per_user` controls replay batch sizing. Events outside the retention window are removed by `sp_prune_sse_events`.

The persistence API in `services::sse_persistence` exposes:

* `record_event(conversation_id, record)` – called alongside live fan-out.
* `load_recent_events(conversation_id, limit)` – returns the most recent `limit` events for a new connection.
* `load_events_after(conversation_id, last_sequence, limit)` – used when the client sends `Last-Event-ID`.

`StreamHub::subscribe` merges (persisted + in-memory) events by sequence number before emitting them via SSE to preserve monotonic ordering.

## Replay Contract

1. Clients should send `Last-Event-ID` (or `since=` query param) when reconnecting.
2. The server loads persisted rows newer than the recorded sequence and replays them **before** live events.
3. The response includes `event.id` fields that combine `root_id`, `message_id`, and sequence where applicable (e.g. `rowid:messageid:sequence` for deltas).
4. When the persistence window does not cover the requested sequence, the server falls back to the configured recent slice; clients should treat missing data as a hint to re-sync via REST APIs.

Metrics emitted during replay:

* `rustygpt_sse_replay_events_total{type=presentation}` – counts per event type.
* `rustygpt_sse_replay_duration_ms` – histogram of replay time.

Dashboard JSON for Prometheus is provided in `deploy/grafana/sse.json`.

## Client Responsibilities

* Always listen for `X-Session-Rotated` and retry the SSE connection if a 401 with `WWW-Authenticate: session` is received.
* Be prepared to receive persisted events interleaved with live updates.
* honour the CSRF token for any POST/DELETE actions the client performs as a consequence of replayed events.
