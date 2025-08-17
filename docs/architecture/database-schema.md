# RustyGPT Database Schema

This document provides an architectural overview of the RustyGPT database schema with Mermaid diagrams and cross-references to detailed implementation files.

## Quick Navigation

- **📋 [Table Definitions](./database-schema-tables.md)** - Complete table structures, indexes, and constraints
- **⚙️ [Procedures & Functions](./database-schema-procs.md)** - Stored procedures, functions, and triggers
- **🔧 [Database Optimization Guide](./database-optimization.md)** - Performance tuning recommendations

## Database Architecture Overview

The RustyGPT database is designed as a high-performance knowledge graph with automatic relationship inference, confidence scoring, and vector similarity search capabilities.

### Core Design Principles

1. **Knowledge Graph Structure** - Nodes and relationships with rich attributes
2. **Confidence-Based Inference** - Automated relationship discovery with confidence scoring
3. **Vector Similarity** - Embedding-based semantic search and clustering
4. **Performance Optimization** - Materialized views and intelligent indexing
5. **System Configuration** - Tunable parameters for optimal performance

## Entity Relationship Diagram

```mermaid
erDiagram
    nodes ||--o{ node_attributes : "has attributes"
    nodes ||--o{ relationships : "source node"
    nodes ||--o{ relationships : "target node"
    nodes ||--o{ node_resource_links : "linked to"
    external_resources ||--o{ node_resource_links : "provides"
    relationships ||--o{ relationship_inference_queue : "queued for inference"
    nodes ||--o{ reference_tracking : "referenced in"
    system_config ||--o{ config_audit_log : "configuration changes"
    
    nodes {
        integer node_id PK
        text name
        text node_type
        text description
        vector embedding
        decimal confidence_score
        timestamp created_at
        timestamp updated_at
        text source
        boolean auto_managed
    }
    
    node_attributes {
        integer attribute_id PK
        integer node_id FK
        text attribute_key
        text attribute_value
        decimal confidence
        text source
        timestamp created_at
        timestamp updated_at
    }
    
    relationships {
        integer relationship_id PK
        integer source_node_id FK
        integer target_node_id FK
        text relationship_type
        decimal confidence_score
        text source
        boolean auto_managed
        integer shared_attributes_count
        timestamp created_at
        timestamp updated_at
    }
    
    external_resources {
        integer resource_id PK
        text resource_type
        text resource_url
        text title
        text description
        vector embedding
        jsonb metadata
        timestamp created_at
        timestamp updated_at
    }
    
    node_resource_links {
        integer link_id PK
        integer node_id FK
        integer resource_id FK
        decimal relevance_score
        text link_type
        timestamp created_at
    }
    
    relationship_inference_queue {
        integer queue_id PK
        integer node_id FK
        integer priority
        text status
        timestamp queued_at
        timestamp processing_started_at
        timestamp completed_at
        text error_message
        integer retry_count
    }
    
    reference_tracking {
        integer reference_id PK
        integer node_id FK
        text reference_type
        text reference_source
        text reference_context
        timestamp created_at
    }
    
    system_config {
        integer config_id PK
        text parameter_name
        text parameter_value
        text parameter_type
        text description
        text default_value
        text min_value
        text max_value
        boolean requires_restart
        text category
        timestamp created_at
        timestamp updated_at
        text updated_by
    }
    
    config_audit_log {
        integer log_id PK
        text parameter_name
        text old_value
        text new_value
        text updated_by
        timestamp updated_at
        boolean requires_restart
    }
```

## Knowledge Graph Structure

The RustyGPT knowledge graph follows a hyper-normalized design that supports dynamic relationship inference and high-performance queries.

```mermaid
graph TB
    subgraph "Core Knowledge Graph"
        N[Nodes] --> A[Node Attributes]
        N --> R[Relationships]
        N --> E[External Resources]
    end
    
    subgraph "Inference Engine"
        A --> I[Inference Queue]
        I --> P[Processing Engine]
        P --> R
        P --> C[Confidence Calculator]
    end
    
    subgraph "Vector Operations"
        N --> V[Vector Embeddings]
        E --> V
        V --> S[Similarity Search]
        S --> R
    end
    
    subgraph "System Management"
        SC[System Config] --> O[Optimization Engine]
        O --> V
        O --> P
        O --> M[Materialized Views]
    end
```

## Data Flow Diagrams

### Confidence Calculation Flow

```mermaid
flowchart TD
    A[Attribute Update] --> B{Trigger Confidence Update}
    B --> C[Calculate Attribute Confidence]
    C --> D[Weight × Source Reliability × Recency]
    D --> E[Update Node Confidence]
    E --> F[Materialized Score Cache]
    
    G[Relationship Update] --> H[Calculate Relationship Confidence]
    H --> I[Shared Attributes Analysis]
    I --> J[Confidence Score Calculation]
    J --> K[Update Materialized Score]
    
    L[Background Job] --> M[Batch Update Process]
    M --> N[Identify Stale Scores]
    N --> O[Recalculate Confidence]
    O --> F
    O --> K
```

### Relationship Inference Flow

```mermaid
flowchart LR
    A[Node/Attribute Change] --> B[Queue Inference]
    B --> C[Batch Processor]
    C --> D[Analyze Shared Attributes]
    D --> E{Meets Threshold?}
    E -->|Yes| F[Create/Update Relationship]
    E -->|No| G[Skip]
    F --> H[Calculate Confidence]
    H --> I[Store Result]
    I --> J[Update Materialized Views]
    
    K[Cleanup Job] --> L[Remove Invalid Relationships]
    L --> M[Update Statistics]
```

### Vector Index Optimization Flow

```mermaid
flowchart TD
    A[Data Growth Detection] --> B[Calculate Optimal Parameters]
    B --> C{Significant Change?}
    C -->|Yes| D[Update Configuration]
    C -->|No| E[Skip Update]
    D --> F{Rebuild Needed?}
    F -->|Yes| G[Rebuild Index]
    F -->|No| H[Update Metadata]
    G --> I[Performance Validation]
    H --> I
    I --> J[Log Results]
```

## System Configuration Architecture

```mermaid
graph TB
    subgraph "Configuration Management"
        SC[System Config Table] --> V[Validation Engine]
        V --> A[Audit Log]
        SC --> O[Optimization Functions]
    end
    
    subgraph "Performance Tuning"
        O --> VI[Vector Index Config]
        O --> DB[Database Performance]
        O --> IF[Inference Parameters]
    end
    
    subgraph "Monitoring & Health"
        HC[Health Checks] --> V
        HC --> M[Metrics Collection]
        M --> AL[Alerting]
    end
```

## Performance Optimization Features

### Materialized Confidence Scores

The database uses materialized confidence scores to eliminate expensive real-time calculations:

- **Node Confidence**: Pre-calculated aggregate scores based on attribute confidence
- **Relationship Confidence**: Cached scores based on shared attributes and node confidence
- **Automatic Updates**: Triggers maintain consistency when underlying data changes
- **Background Jobs**: Scheduled updates for stale confidence scores

### Vector Index Management

- **Adaptive Parameters**: Automatic calculation of optimal IVFFlat `lists` parameter
- **Performance Monitoring**: Tracks index usage and query performance
- **Auto-Optimization**: Rebuilds indexes when significant improvements are available
- **Configuration Management**: Tunable parameters for different deployment sizes

### Intelligent Indexing Strategy

- **Composite Indexes**: Multi-column indexes for common query patterns
- **Partial Indexes**: Conditional indexes for filtered queries
- **Vector Indexes**: Optimized for similarity search operations
- **Confidence Indexes**: Specialized indexes for confidence-based filtering

## Error Handling and Reliability

### Structured Error Framework

- **Error Codes**: Standardized RG-prefixed error codes for different categories
- **Context Preservation**: JSON-structured error details for debugging
- **Transaction Safety**: Error handling that maintains data consistency
- **Retry Logic**: Automatic retry for transient failures

### Data Consistency

- **ACID Compliance**: Full transactional consistency for critical operations
- **Constraint Enforcement**: Database-level validation of business rules
- **Referential Integrity**: Foreign key constraints with cascade rules
- **Concurrent Access**: Proper locking for multi-user environments

## Usage Guidelines

### Best Practices

1. **Confidence Thresholds**: Use materialized confidence scores for filtering
2. **Batch Operations**: Process large datasets using batch functions
3. **Index Optimization**: Regularly run vector index optimization
4. **Configuration Tuning**: Adjust parameters based on data growth patterns
5. **Monitoring**: Track performance metrics and error rates

### Performance Considerations

- **Query Planning**: Use `EXPLAIN ANALYZE` to verify index usage
- **Batch Size Tuning**: Adjust batch sizes based on available memory
- **Connection Pooling**: Use connection pooling for high-concurrency applications
- **Memory Configuration**: Tune PostgreSQL memory settings for workload

### Security Considerations

- **Parameter Validation**: All functions include comprehensive input validation
- **SQL Injection Prevention**: Parameterized queries and proper escaping
- **Access Control**: Role-based permissions for different operations
- **Audit Trail**: Complete audit logging for configuration changes

## Migration and Maintenance

### Schema Updates

- **Version Control**: All schema changes are versioned and tracked
- **Backward Compatibility**: Careful consideration of breaking changes
- **Migration Scripts**: Automated migration procedures
- **Rollback Plans**: Documented rollback procedures for each change

### Regular Maintenance

- **Confidence Updates**: Schedule regular confidence score refresh
- **Index Optimization**: Monitor and optimize vector indexes
- **Statistics Updates**: Keep PostgreSQL statistics current
- **Configuration Review**: Periodic review of configuration parameters

## Related Documentation

- **[Table Definitions](./database-schema-tables.md)** - Detailed table structures with all constraints and indexes
- **[Procedures & Functions](./database-schema-procs.md)** - Complete stored procedure documentation with examples
- **[Database Optimization Guide](./database-optimization.md)** - Advanced performance tuning recommendations
- **[API Integration Guide](./api-integration.md)** - How to integrate with the database from applications

## Schema Evolution

The RustyGPT database schema is designed to evolve with the system's needs while maintaining compatibility and performance. Key evolution strategies include:

- **Incremental Improvements**: Small, backward-compatible enhancements
- **Performance Optimization**: Continuous improvement of indexes and queries
- **Feature Additions**: New capabilities added through modular design
- **Scalability Enhancements**: Optimizations for larger datasets and higher concurrency

This architecture supports the RustyGPT system's requirements for intelligent knowledge graph management, high-performance vector operations, and reliable automated inference capabilities.