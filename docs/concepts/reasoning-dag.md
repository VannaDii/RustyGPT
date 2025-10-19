# Reasoning DAG

> TL;DR – RustyGPT models agent reasoning as a directed acyclic graph so parallel branches can explore hypotheses while preserving deterministic joins.

## Overview

RustyGPT splits complex prompts into nodes that represent specialised reasoning steps—retrieval, tool execution, synthesis, and user messaging. Nodes execute concurrently when dependencies permit, then merge downstream results. The orchestrator’s scheduler lives in `rustygpt-server/src/reasoning`.

## Node Types

| Node            | Purpose                            |
|-----------------|------------------------------------|
| `Prompt`        | Emits the initial LLM prompt       |
| `Retriever`     | Executes vector or keyword search  |
| `Tool`          | Calls out to deterministic actions |
| `Reducer`       | Merges branch outputs              |
| `Responder`     | Streams user-facing tokens         |

Each node publishes structured telemetry through `rustygpt-shared::telemetry`, enabling downstream analytics and reliability alerts.

## Benefits

- **Determinism** – Branch IDs are deterministic, enabling reproducible transcripts.
- **Parallelism** – Low-latency branches (e.g., caching, fast lookups) unblock slower tool invocations.
- **Observability** – Joins track input provenance, aiding postmortems and safety audits.

See [Streaming Delivery](../architecture/streaming.md) for how reducer output flows to SSE clients.

## Extending the Graph

New node types implement the `ReasoningNode` trait. Add integration tests under `rustygpt-server/tests/reasoning.rs` to validate invariants. Captured node metadata propagates to the [REST API](../reference/api.md) for clients that reconstruct reasoning trails.

## Related Concepts

- [Dimensioned Entities](dimensioned-entities.md) define the typed payloads that move through each edge.
- [Service Topology](../architecture/service-topology.md) explains where reasoning orchestration runs relative to other services.
