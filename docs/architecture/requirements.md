# RustyGPT Requirements

This document outlines the detailed functional and non-functional requirements for the RustyGPT system.

## Overview

The system will be built entirely in Rust to ensure performance, safety, and modularity. The primary objective is to develop a Reasoning DAG with RAG integration capable of dynamically orchestrating nodes in a coded function workflow. The following requirements are categorized using the MoSCoW prioritization:

## Must-Have

### 1. Dynamic Reasoning DAG Structure

- Support modular and autonomous nodes that can dynamically interact.
- Ensure each node can perform specific reasoning tasks independently.

### 2. Hyper-Normalized Data Storage

- Utilize PostgreSQL for structured entity storage.
- Efficiently handle dynamic entity relationships and attributes.
- Employ embedding vectors for semantic matching.

### 3. Embedding Generation and Maintenance

- Generate entity embeddings using llama_cpp or equivalent Rust libraries.
- Support incremental indexing to minimize computational overhead during updates.
- Implement model re-basing to maintain embedding consistency.

### 4. Node Orchestration for Workflow Management

- Enable coded function workflows for testing and modification.
- Ensure nodes can be easily added, removed, or updated without disrupting the DAG.

### 5. Efficient Pruning and Memory Management

- Employ pruning strategies to maintain high performance in large-scale deployments.
- Implement caching and garbage collection for embedding vectors.

### 6. Retrieval-Augmented Generation (RAG) Integration

- Seamlessly link entities to external data sources.
- Efficiently incorporate external data into the reasoning process.

### 7. Performance Optimization

- Utilize GPU acceleration for embedding operations when available.
- Implement real-time indexing and retrieval for rapid reasoning.

## Should-Have

### 1. Robust Testing and Debugging Framework

- Enable testing of individual nodes and entire workflows.
- Provide detailed logs for reasoning paths and node interactions.

### 2. Lifecycle Management for Entities

- Implement lifecycle stages from creation to pruning, ensuring long-term accuracy and relevance.
- Automate embedding updates and archiving of inactive entities.

## Could-Have

### 1. Adaptive Pruning Thresholds

- Dynamically adjust pruning intensity based on system load.

### 2. Data Visualization Dashboard

- Visualize the DAG structure and entity relationships for better monitoring.

## Won't Have

### 1. Web-Based User Interface

- Initial implementation will focus on core functionality and command-line interaction.
- GUI or web-based interfaces are not a priority at this stage.

## Related Documents

For detailed information about specific aspects of the architecture, see:

- [Architecture Overview](./overview.md) - High-level system overview and key concepts
- [Reasoning DAG](./reasoning-dag.md) - In-depth DAG architecture and node orchestration
- [Database Schema](./database-schema.md) - Core database design and stored procedures
- [Database Optimization](./database-optimization.md) - Advanced optimization strategies
- [Error Handling](./error-handling.md) - Comprehensive error handling architecture
