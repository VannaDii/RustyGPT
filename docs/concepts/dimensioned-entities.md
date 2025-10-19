# Dimensioned Entities

> TL;DR – Dimensioned entities annotate RustyGPT data structures with semantic and temporal dimensions so downstream systems can reason about provenance and lifecycle.

## Motivation

Conversation transcripts, tool inputs, and generated artifacts need contextual metadata to remain auditable. Dimensioned entities encode:

- **Subject** – the domain object (`conversation`, `agent`, `session`).
- **Scope** – user, tenant, or system ownership.
- **Temporal dimension** – creation time, last mutation, retention deadline.
- **Sensitivity** – policy hints for redaction and export.

These attributes ride along with each payload in `rustygpt-shared`, ensuring services apply the right policies without duplicating logic.

## Implementation

- Structs derive `Dimensioned` via a procedural macro that injects helpers for tagging and serialisation.
- `rustygpt-server` enforces scope isolation using the dimension metadata during database queries.
- Export pipelines honour retention windows by consulting the temporal dimensions before emitting records.

Refer to [Reasoning DAG](reasoning-dag.md) for how dimensions propagate across reasoning nodes.

## Usage Guidelines

- Prefer explicit dimensions over ad-hoc metadata fields.
- When introducing a new entity, document its semantics under [Configuration](../reference/config.md) if tunable.
- Include dimension checks in integration tests to catch accidental policy regressions.

## Further Reading

- [Service Topology](../architecture/service-topology.md) shows where dimension enforcement lives in the runtime.
- [Rotate Secrets](../howto/rotate-secrets.md) demonstrates applying dimension metadata during credential refresh workflows.
