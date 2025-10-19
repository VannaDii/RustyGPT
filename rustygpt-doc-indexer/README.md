# rustygpt-doc-indexer

Rust-native mdBook companion that scans the `docs/` tree, extracts metadata, and emits machine-readable manifests (`manifest.json`, `summaries.json`) for LLM ingestion.

The binary is invoked via `just docs-index` and enforces schema validation defined in `docs/llm/schema.json`.
