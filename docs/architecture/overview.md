# RustyGPT Architecture Overview

This document provides a high-level overview of the RustyGPT project architecture and key concepts.

## Background

The design is centered around the development of a Dimensioned-Entity-Enabled Reasoning DAG with Retrieval-Augmented Generation (RAG). This architecture aims to enable dynamic, scalable, and context-rich reasoning while maintaining high performance within resource-constrained environments.

Key Concepts:

1. Dimensioned Entities: Structured representations with dynamic attributes that maintain contextual and relational properties, essential for efficient reasoning.
2. Reasoning DAG: A directed acyclic graph that allows for structured, modular reasoning processes with dynamic context adaptation.
3. RAG: Integrates external knowledge sources, enriching responses and maintaining contextual relevance.

The architecture prioritizes a Rust-based implementation for performance, leveraging the llama_cpp crate for embedding generation and the PostgreSQL database for hyper-normalized storage of entities and relationships. The design ensures modularity, allowing the integration and orchestration of nodes to perform specific reasoning tasks, making it easily testable and modifiable.

## System Architecture

The system is implemented as a Directed Acyclic Graph (DAG), where each node performs a specific reasoning function. The graph structure allows for modular, autonomous processing, supporting dynamic interactions between nodes.

### Key Components:

1. **Entity Identification Node**: Uses vector similarity to find the most relevant entities based on input data.
2. **Slot Resolution Node**: Fills required and optional slots for identified entities.
3. **Relationship Traversal Node**: Expands reasoning by exploring relationships between identified entities.
4. **Meta-Reasoning Controller**: Validates the coherence of the reasoning chain.
5. **Pruning Node**: Reduces reasoning complexity by removing low-relevance entities or outdated relationships.
6. **Generation Node**: Synthesizes responses based on resolved entities and enriched context.
7. **RAG Integration Node**: Links entities to external resources when context gaps are detected.

### Technology Stack

- **Language**: Rust (for performance, safety, and modularity)
- **Database**: PostgreSQL with ltree extension for hierarchical relationships
- **Embeddings**: llama_cpp crate for vector generation
- **Vector Search**: PostgreSQL with vector extension for semantic similarity
- **Architecture Pattern**: Directed Acyclic Graph (DAG) for modular reasoning

## Related Documents

- [Requirements](requirements.md) - Detailed system requirements with MoSCoW prioritization
- [Reasoning DAG](reasoning-dag.md) - DAG architecture, components, and orchestration workflow
- [Database Schema](database-schema.md) - Core database design, tables, and stored procedures
- [Database Optimization](database-optimization.md) - Advanced database optimization strategies
- [Error Handling](error-handling.md) - Comprehensive error handling architecture and patterns
