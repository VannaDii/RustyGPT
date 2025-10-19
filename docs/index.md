# RustyGPT Documentation

> TL;DR – Everything you need to build, run, and extend RustyGPT with a pure Rust toolchain. Start with Quickstart, then dive into Concepts and Architecture for deeper system insight.

Welcome to the RustyGPT docs. This site is:

- **Rust-native**: powered by mdBook with Rust preprocessors
- **Versioned**: each release is immutable; `latest` tracks `main`
- **Machine-friendly**: LLM manifests are published beside the book

## Fast Links

- [Quickstart](guide/quickstart.md)
- [Local Development](guide/local-dev.md)
- [Streaming Delivery](architecture/streaming.md)
- [REST API](reference/api.md)

## About RustyGPT

RustyGPT is a modular chat platform composed of a Rust backend, CLI tools, and a Yew-powered web client. The project emphasises deterministic reasoning, low-latency streaming, and reproducible deployments. For system context, read [Service Topology](architecture/service-topology.md) and the shared [Reasoning DAG](concepts/reasoning-dag.md).

## Governance & Support

Documentation changes follow the [docs review checklist](../CONTRIBUTING.md) and this repository’s [CODE_OF_CONDUCT.md](../CODE_OF_CONDUCT.md). Open issues and proposals in the `docs` label so we can triage them efficiently.
