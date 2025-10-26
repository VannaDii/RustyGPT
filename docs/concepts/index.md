# Concepts Overview

This section explains the core ideas that appear across the server, web client, and CLI. Use it to understand the vocabulary
used in API responses and stream payloads before diving into the reference material.

- [Threaded conversations](reasoning-dag.md) describes how messages, threads, and `ConversationStreamEvent` values relate to each
  other.
- [Shared models](dimensioned-entities.md) covers the `rustygpt-shared` crate, focusing on how typed DTOs and enums keep clients
  in sync with the server.

Pair these concepts with the [Architecture](../architecture/index.md) diagrams to see where each part lives at runtime.
