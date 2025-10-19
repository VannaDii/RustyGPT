# Architecture Overview

> TL;DR – System-level views of RustyGPT services, data flows, and cross-cutting behaviours such as streaming and rate limiting.

## What’s Inside

- [Service Topology](service-topology.md) maps runtime components and communication paths.
- [Streaming Delivery](streaming.md) details the SSE fan-out model.
- [Rate-Limit Architecture](rate-limits.md) covers runtime throttling against Postgres-backed profiles.

Pair these diagrams with conceptual background from [Concepts](../concepts/index.md) and operational runbooks in [How-to](../howto/index.md).
