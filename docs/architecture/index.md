# Architecture Overview

These chapters document the runtime architecture of RustyGPT: how the Axum server is composed, how the SSE stream hub works,
and how rate limiting integrates with PostgreSQL. Use them alongside the [concepts](../concepts/index.md) and
[reference](../reference/index.md) sections when exploring the code.

- [Service topology](service-topology.md)
- [Streaming delivery](streaming.md)
- [Rate-limit architecture](rate-limits.md)
