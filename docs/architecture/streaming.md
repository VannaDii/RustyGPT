# Streaming Delivery

> TL;DR – RustyGPT streams conversation updates over Server-Sent Events (SSE) with persisted replay, ensuring reconnecting clients rebuild state without losing tokens.

## Diagram

{{#include ../_snippets/diagrams/sse-flow.mmd}}

_Caption: SSE requests flow through the gateway, persist to the event log, and replay before live streaming._

See [Service Topology](service-topology.md) for broader context on how StreamHub integrates with other components.

## Event Model

All SSE messages share the shape `{ "type": "...", "payload": { ... } }` defined in `shared::models::ConversationStreamEvent`.

| Event name           | Description                                      |
|----------------------|--------------------------------------------------|
| `thread.new`         | New thread summary                               |
| `thread.activity`    | Updated activity timestamp for a thread          |
| `message.delta`      | Streaming chunk for an in-flight assistant reply |
| `message.done`       | Completion marker plus usage stats               |
| `presence.update`    | User presence status update                      |
| `typing.update`      | Typing indicator with expiry timestamp           |
| `unread.update`      | Unread count snapshot for a thread               |
| `membership.changed` | Conversation membership mutation                 |
| `error`              | Terminal errors propagated to the client         |

The `root_id` field keeps replay ordering stable for thread-scoped events.

## Persistence Window

`rustygpt.sse_event_log` stores a rolling window controlled by:

- `sse.persistence.retention_hours` (clamped 24–72h)
- `sse.persistence.max_events_per_user` (replay batch size)

Key stored procedures in `services::sse_persistence`:

- `record_event(conversation_id, record)` – invoked alongside live fan-out
- `load_recent_events(conversation_id, limit)` – fetches the most recent persisted window
- `load_events_after(conversation_id, last_sequence, limit)` – used when clients send `Last-Event-ID`

`StreamHub::subscribe` merges persisted and live events by sequence before emitting to SSE clients.

## Replay Contract

1. Clients send `Last-Event-ID` (or `since=` query param) when reconnecting.
2. The server replays persisted rows newer than the recorded sequence before live events resume.
3. Responses include `event.id` fields combining `root_id`, `message_id`, and sequence (`rowid:messageid:sequence` for deltas).
4. When the window no longer covers a requested sequence, clients re-sync via REST APIs using [REST API](../reference/api.md).

Metrics emitted during replay:

- `rustygpt_sse_replay_events_total{type=...}`
- `rustygpt_sse_replay_duration_ms`

Dashboards live in `deploy/grafana/sse.json`; see [Docker Deploy](../howto/docker-deploy.md) for production rollouts.

## Client Responsibilities

- React to `X-Session-Rotated` headers and retry the SSE connection if a `401` with `WWW-Authenticate: session` arrives.
- Accept persisted events interleaved with live ones.
- Honour CSRF tokens for any POST/DELETE calls triggered by replayed events.

For end-to-end reliability, combine this guidance with [Rotate Secrets](../howto/rotate-secrets.md) to avoid invalidating sessions mid-stream.
