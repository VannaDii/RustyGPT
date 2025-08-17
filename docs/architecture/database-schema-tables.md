# RustyGPT Database Schema - Tables

This document contains all table definitions for the RustyGPT database schema.

## Required Extensions

```sql
CREATE EXTENSION IF NOT EXISTS ltree;
CREATE EXTENSION IF NOT EXISTS vector;
```

## Core Tables

### Users Table

Authentication and user management table.

```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    is_active BOOLEAN DEFAULT TRUE,
    last_login TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_active ON users(is_active);
```

### Nodes Table

Core entity storage with hierarchical paths and vector embeddings.

```sql
CREATE TABLE nodes (
    node_id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    node_type VARCHAR(50) NOT NULL,
    path LTREE,
    parent_node_id INTEGER REFERENCES nodes(node_id),
    embedding VECTOR(768),
    metadata JSONB DEFAULT '{}',
    confidence_score DECIMAL(5,4) DEFAULT 0.5,
    confidence_last_updated TIMESTAMP DEFAULT NOW(),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by INTEGER REFERENCES users(id),
    source VARCHAR(50) DEFAULT 'manual',
    CONSTRAINT valid_confidence_score CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0)
);

-- Indexes for performance
CREATE INDEX idx_nodes_type ON nodes(node_type);
CREATE INDEX idx_nodes_path ON nodes USING gist(path);
CREATE INDEX idx_nodes_parent ON nodes(parent_node_id);
CREATE INDEX idx_nodes_confidence ON nodes(confidence_score DESC);
CREATE INDEX idx_nodes_confidence_updated ON nodes(confidence_last_updated);
CREATE INDEX idx_nodes_created ON nodes(created_at DESC);
CREATE INDEX idx_nodes_source ON nodes(source);

-- Vector similarity index with configurable parameters
-- The lists parameter should be calculated as approximately sqrt(row_count)
-- See vector index optimization functions in database-schema-procs.md
CREATE INDEX idx_nodes_embedding ON nodes USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
```

### Node Attributes Table

Flexible attribute storage with confidence tracking and source reliability.

```sql
CREATE TABLE node_attributes (
    attribute_id SERIAL PRIMARY KEY,
    node_id INTEGER NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
    attribute_type VARCHAR(100) NOT NULL,
    attribute_key VARCHAR(255) NOT NULL,
    attribute_value TEXT NOT NULL,
    weight DECIMAL(3,2) DEFAULT 1.0,
    confidence DECIMAL(5,4) DEFAULT 0.5,
    source_reliability DECIMAL(5,4) DEFAULT 1.0,
    last_verified TIMESTAMP DEFAULT NOW(),
    created_at TIMESTAMP DEFAULT NOW(),
    created_by INTEGER REFERENCES users(id),
    metadata JSONB DEFAULT '{}',
    CONSTRAINT valid_weight CHECK (weight >= 0.0 AND weight <= 1.0),
    CONSTRAINT valid_confidence CHECK (confidence >= 0.0 AND confidence <= 1.0),
    CONSTRAINT valid_source_reliability CHECK (source_reliability >= 0.0 AND source_reliability <= 1.0)
);

-- Indexes for attribute queries and relationship inference
CREATE INDEX idx_node_attributes_node ON node_attributes(node_id);
CREATE INDEX idx_node_attributes_type ON node_attributes(attribute_type);
CREATE INDEX idx_node_attributes_key ON node_attributes(attribute_key);
CREATE INDEX idx_node_attributes_value ON node_attributes(attribute_value);
CREATE INDEX idx_node_attributes_confidence ON node_attributes(confidence DESC);
CREATE INDEX idx_node_attributes_composite ON node_attributes(attribute_type, attribute_key, attribute_value);
CREATE INDEX idx_node_attributes_verified ON node_attributes(last_verified DESC);
```

### Relationships Table

Manages relationships between nodes with confidence tracking and inference metadata.

```sql
CREATE TABLE relationships (
    relationship_id SERIAL PRIMARY KEY,
    source_node_id INTEGER NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
    target_node_id INTEGER NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
    relationship_type VARCHAR(100) NOT NULL,
    strength DECIMAL(5,4) DEFAULT 1.0,
    confidence_score DECIMAL(5,4) DEFAULT 0.5,
    confidence_last_updated TIMESTAMP DEFAULT NOW(),
    bidirectional BOOLEAN DEFAULT FALSE,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by INTEGER REFERENCES users(id),
    source VARCHAR(50) DEFAULT 'manual',
    auto_managed BOOLEAN DEFAULT FALSE,
    shared_attributes_count INTEGER DEFAULT 0,
    inference_confidence DECIMAL(5,4),
    last_inference_update TIMESTAMP,
    source_reliability DECIMAL(5,4) DEFAULT 1.0,
    CONSTRAINT valid_strength CHECK (strength >= 0.0 AND strength <= 1.0),
    CONSTRAINT valid_confidence_score CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    CONSTRAINT valid_inference_confidence CHECK (inference_confidence IS NULL OR (inference_confidence >= 0.0 AND inference_confidence <= 1.0)),
    CONSTRAINT valid_source_reliability CHECK (source_reliability >= 0.0 AND source_reliability <= 1.0),
    CONSTRAINT no_self_reference CHECK (source_node_id != target_node_id),
    CONSTRAINT unique_relationship UNIQUE (source_node_id, target_node_id, relationship_type)
);

-- Indexes for relationship queries
CREATE INDEX idx_relationships_source ON relationships(source_node_id);
CREATE INDEX idx_relationships_target ON relationships(target_node_id);
CREATE INDEX idx_relationships_type ON relationships(relationship_type);
CREATE INDEX idx_relationships_confidence ON relationships(confidence_score DESC);
CREATE INDEX idx_relationships_confidence_updated ON relationships(confidence_last_updated);
CREATE INDEX idx_relationships_strength ON relationships(strength DESC);
CREATE INDEX idx_relationships_bidirectional ON relationships(bidirectional);
CREATE INDEX idx_relationships_source_type ON relationships(source);
CREATE INDEX idx_relationships_auto_managed ON relationships(auto_managed);
CREATE INDEX idx_relationships_inference_updated ON relationships(last_inference_update);
CREATE INDEX idx_relationships_composite ON relationships(source_node_id, target_node_id, relationship_type);
```

### Relationship Inference Queue Table

Manages background processing of relationship inference operations.

```sql
CREATE TABLE relationship_inference_queue (
    queue_id SERIAL PRIMARY KEY,
    node_id INTEGER NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
    priority INTEGER DEFAULT 5,
    batch_id TEXT,
    status VARCHAR(20) DEFAULT 'pending',
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    error_message TEXT,
    queued_at TIMESTAMP DEFAULT NOW(),
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    processing_node TEXT,
    estimated_completion TIMESTAMP,
    CONSTRAINT valid_priority CHECK (priority >= 1 AND priority <= 10),
    CONSTRAINT valid_status CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled'))
);

-- Indexes for queue processing
CREATE INDEX idx_inference_queue_status ON relationship_inference_queue(status);
CREATE INDEX idx_inference_queue_priority ON relationship_inference_queue(priority DESC, queued_at ASC);
CREATE INDEX idx_inference_queue_node ON relationship_inference_queue(node_id);
CREATE INDEX idx_inference_queue_batch ON relationship_inference_queue(batch_id);
CREATE INDEX idx_inference_queue_processing ON relationship_inference_queue(status, priority DESC) WHERE status = 'pending';
CREATE INDEX idx_inference_queue_retry ON relationship_inference_queue(retry_count, max_retries);
```

### External Resources Table

Manages external resource references with vector embeddings for similarity search.

```sql
CREATE TABLE external_resources (
    resource_id SERIAL PRIMARY KEY,
    url TEXT UNIQUE NOT NULL,
    title TEXT,
    description TEXT,
    resource_type VARCHAR(50),
    embedding VECTOR(768),
    content_hash VARCHAR(64),
    last_crawled TIMESTAMP,
    crawl_status VARCHAR(20) DEFAULT 'pending',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by INTEGER REFERENCES users(id),
    CONSTRAINT valid_crawl_status CHECK (crawl_status IN ('pending', 'crawling', 'completed', 'failed', 'stale'))
);

-- Indexes for external resources
CREATE INDEX idx_external_resources_url ON external_resources(url);
CREATE INDEX idx_external_resources_type ON external_resources(resource_type);
CREATE INDEX idx_external_resources_status ON external_resources(crawl_status);
CREATE INDEX idx_external_resources_crawled ON external_resources(last_crawled DESC);
CREATE INDEX idx_external_resources_hash ON external_resources(content_hash);

-- Vector similarity index with configurable parameters
CREATE INDEX idx_external_resources_embedding ON external_resources USING ivfflat (embedding vector_cosine_ops) WITH (lists = 50);
```

### Node Resource Links Table

Links nodes to external resources with relationship metadata.

```sql
CREATE TABLE node_resource_links (
    link_id SERIAL PRIMARY KEY,
    node_id INTEGER NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
    resource_id INTEGER NOT NULL REFERENCES external_resources(resource_id) ON DELETE CASCADE,
    link_type VARCHAR(50) DEFAULT 'reference',
    confidence DECIMAL(5,4) DEFAULT 0.5,
    context TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    created_by INTEGER REFERENCES users(id),
    CONSTRAINT valid_confidence CHECK (confidence >= 0.0 AND confidence <= 1.0),
    CONSTRAINT unique_node_resource_link UNIQUE (node_id, resource_id, link_type)
);

-- Indexes for node-resource relationships
CREATE INDEX idx_node_resource_links_node ON node_resource_links(node_id);
CREATE INDEX idx_node_resource_links_resource ON node_resource_links(resource_id);
CREATE INDEX idx_node_resource_links_type ON node_resource_links(link_type);
CREATE INDEX idx_node_resource_links_confidence ON node_resource_links(confidence DESC);
```

### Reference Tracking Table

Tracks references between entities for citation and provenance.

```sql
CREATE TABLE reference_tracking (
    reference_id SERIAL PRIMARY KEY,
    source_node_id INTEGER REFERENCES nodes(node_id) ON DELETE CASCADE,
    source_resource_id INTEGER REFERENCES external_resources(resource_id) ON DELETE CASCADE,
    target_node_id INTEGER REFERENCES nodes(node_id) ON DELETE CASCADE,
    target_resource_id INTEGER REFERENCES external_resources(resource_id) ON DELETE CASCADE,
    reference_type VARCHAR(50) NOT NULL,
    page_number INTEGER,
    section TEXT,
    quote TEXT,
    context JSONB DEFAULT '{}',
    created_at TIMESTAMP DEFAULT NOW(),
    created_by INTEGER REFERENCES users(id),
    CONSTRAINT valid_reference_source CHECK (
        (source_node_id IS NOT NULL AND source_resource_id IS NULL) OR
        (source_node_id IS NULL AND source_resource_id IS NOT NULL)
    ),
    CONSTRAINT valid_reference_target CHECK (
        (target_node_id IS NOT NULL AND target_resource_id IS NULL) OR
        (target_node_id IS NULL AND target_resource_id IS NOT NULL)
    )
);

-- Indexes for reference tracking
CREATE INDEX idx_reference_tracking_source_node ON reference_tracking(source_node_id);
CREATE INDEX idx_reference_tracking_source_resource ON reference_tracking(source_resource_id);
CREATE INDEX idx_reference_tracking_target_node ON reference_tracking(target_node_id);
CREATE INDEX idx_reference_tracking_target_resource ON reference_tracking(target_resource_id);
CREATE INDEX idx_reference_tracking_type ON reference_tracking(reference_type);
```

## System Configuration Tables

### System Configuration Table

Stores tunable database parameters for performance optimization and vector index configuration.

```sql
CREATE TABLE system_config (
    config_id SERIAL PRIMARY KEY,
    parameter_name VARCHAR(100) UNIQUE NOT NULL,
    parameter_value TEXT NOT NULL,
    parameter_type VARCHAR(20) NOT NULL CHECK (parameter_type IN ('integer', 'float', 'text', 'boolean')),
    description TEXT NOT NULL,
    default_value TEXT NOT NULL,
    min_value TEXT,
    max_value TEXT,
    requires_restart BOOLEAN DEFAULT FALSE,
    category VARCHAR(50) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    updated_by TEXT DEFAULT 'system'
);

-- Indexes for efficient parameter lookups
CREATE INDEX idx_system_config_name ON system_config(parameter_name);
CREATE INDEX idx_system_config_category ON system_config(category);

-- Initialize default configuration parameters
INSERT INTO system_config (parameter_name, parameter_value, parameter_type, description, default_value, min_value, max_value, requires_restart, category) VALUES

-- Vector Index Configuration
('vector_embedding_dimension', '768', 'integer', 'Default vector embedding dimension for new indexes', '768', '64', '4096', false, 'vector_index'),
('ivfflat_lists_nodes', '100', 'integer', 'IVFFlat lists parameter for nodes table vector index', '100', '10', '10000', false, 'vector_index'),
('ivfflat_lists_external_resources', '50', 'integer', 'IVFFlat lists parameter for external_resources table vector index', '50', '10', '10000', false, 'vector_index'),
('vector_index_rebuild_threshold', '10000', 'integer', 'Row count threshold for automatic vector index parameter recalculation', '10000', '1000', '1000000', false, 'vector_index'),
('vector_similarity_threshold', '0.3', 'float', 'Default similarity threshold for vector searches', '0.3', '0.0', '1.0', false, 'vector_index'),

-- Database Performance Configuration
('maintenance_work_mem', '256MB', 'text', 'Memory used for maintenance operations like index creation', '256MB', '64MB', '2GB', true, 'performance'),
('effective_cache_size', '4GB', 'text', 'Estimate of memory available for disk caching', '4GB', '1GB', '64GB', true, 'performance'),
('shared_buffers', '256MB', 'text', 'Amount of memory for shared buffer cache', '256MB', '128MB', '8GB', true, 'performance'),
('work_mem', '4MB', 'text', 'Memory used for query operations before spilling to disk', '4MB', '1MB', '1GB', false, 'performance'),
('random_page_cost', '1.1', 'float', 'Cost estimate for non-sequential disk page fetch', '1.1', '0.1', '4.0', false, 'performance'),

-- Inference and Processing Configuration
('inference_batch_size', '100', 'integer', 'Default batch size for relationship inference processing', '100', '10', '1000', false, 'inference'),
('confidence_threshold_cleanup', '0.1', 'float', 'Minimum confidence threshold for relationship cleanup', '0.1', '0.0', '0.5', false, 'inference'),
('max_inference_age_days', '30', 'integer', 'Maximum age in days for cached inference results', '30', '1', '365', false, 'inference'),
('parallel_workers', '4', 'integer', 'Number of parallel workers for background processing', '4', '1', '16', false, 'processing'),

-- Materialized View Configuration
('mv_refresh_interval_hours', '24', 'integer', 'Hours between materialized view refreshes', '24', '1', '168', false, 'materialized_views'),
('mv_incremental_threshold', '1000', 'integer', 'Row change threshold for incremental vs full refresh', '1000', '100', '100000', false, 'materialized_views');
```

### Configuration Audit Log Table

Tracks changes to system configuration parameters.

```sql
CREATE TABLE config_audit_log (
    log_id SERIAL PRIMARY KEY,
    parameter_name TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT NOW(),
    requires_restart BOOLEAN DEFAULT FALSE
);

CREATE INDEX idx_config_audit_log_parameter ON config_audit_log(parameter_name);
CREATE INDEX idx_config_audit_log_timestamp ON config_audit_log(updated_at DESC);
```

## Materialized Views for Performance Optimization

### Node Confidence Cache

Materialized view for frequently accessed node confidence scores.

```sql
CREATE MATERIALIZED VIEW mv_node_confidence_cache AS
SELECT
    node_id,
    name,
    node_type,
    confidence_score,
    confidence_last_updated,
    CASE
        WHEN confidence_score >= 0.8 THEN 'high'
        WHEN confidence_score >= 0.5 THEN 'medium'
        ELSE 'low'
    END as confidence_tier
FROM nodes
WHERE confidence_score IS NOT NULL;

CREATE UNIQUE INDEX idx_mv_node_confidence_cache_id ON mv_node_confidence_cache(node_id);
CREATE INDEX idx_mv_node_confidence_cache_tier ON mv_node_confidence_cache(confidence_tier, confidence_score DESC);
CREATE INDEX idx_mv_node_confidence_cache_type ON mv_node_confidence_cache(node_type, confidence_score DESC);
```

### Relationship Strength Matrix

Materialized view for relationship analysis and graph traversal optimization.

```sql
CREATE MATERIALIZED VIEW mv_relationship_strength_matrix AS
SELECT
    r.source_node_id,
    r.target_node_id,
    r.relationship_type,
    r.confidence_score * r.strength as weighted_strength,
    r.confidence_score,
    r.strength,
    sn.node_type as source_type,
    tn.node_type as target_type,
    r.bidirectional
FROM relationships r
JOIN nodes sn ON r.source_node_id = sn.node_id
JOIN nodes tn ON r.target_node_id = tn.node_id
WHERE r.confidence_score >= 0.3;

CREATE INDEX idx_mv_relationship_matrix_source ON mv_relationship_strength_matrix(source_node_id, weighted_strength DESC);
CREATE INDEX idx_mv_relationship_matrix_target ON mv_relationship_strength_matrix(target_node_id, weighted_strength DESC);
CREATE INDEX idx_mv_relationship_matrix_type ON mv_relationship_strength_matrix(relationship_type, weighted_strength DESC);
CREATE INDEX idx_mv_relationship_matrix_strength ON mv_relationship_strength_matrix(weighted_strength DESC);
```

### Graph Connectivity Statistics

Materialized view for graph analysis and health monitoring.

```sql
CREATE MATERIALIZED VIEW mv_graph_connectivity_stats AS
SELECT
    node_type,
    COUNT(*) as node_count,
    AVG(confidence_score) as avg_confidence,
    COUNT(*) FILTER (WHERE confidence_score >= 0.8) as high_confidence_count,
    COUNT(*) FILTER (WHERE confidence_score < 0.3) as low_confidence_count,
    (SELECT COUNT(*) FROM relationships r WHERE r.source_node_id IN (SELECT n2.node_id FROM nodes n2 WHERE n2.node_type = nodes.node_type)) as outgoing_relationships,
    (SELECT COUNT(*) FROM relationships r WHERE r.target_node_id IN (SELECT n2.node_id FROM nodes n2 WHERE n2.node_type = nodes.node_type)) as incoming_relationships
FROM nodes
GROUP BY node_type;

CREATE INDEX idx_mv_graph_connectivity_type ON mv_graph_connectivity_stats(node_type);
CREATE INDEX idx_mv_graph_connectivity_count ON mv_graph_connectivity_stats(node_count DESC);
```

### Popular Entity Rankings

Materialized view for identifying frequently referenced entities.

```sql
CREATE MATERIALIZED VIEW mv_popular_entity_rankings AS
SELECT
    n.node_id,
    n.name,
    n.node_type,
    n.confidence_score,
    COUNT(DISTINCT r1.relationship_id) as outgoing_relationship_count,
    COUNT(DISTINCT r2.relationship_id) as incoming_relationship_count,
    COUNT(DISTINCT r1.relationship_id) + COUNT(DISTINCT r2.relationship_id) as total_relationship_count,
    AVG(COALESCE(r1.confidence_score, r2.confidence_score)) as avg_relationship_confidence,
    COUNT(DISTINCT nrl.resource_id) as linked_resource_count
FROM nodes n
LEFT JOIN relationships r1 ON n.node_id = r1.source_node_id
LEFT JOIN relationships r2 ON n.node_id = r2.target_node_id
LEFT JOIN node_resource_links nrl ON n.node_id = nrl.node_id
GROUP BY n.node_id, n.name, n.node_type, n.confidence_score
HAVING COUNT(DISTINCT r1.relationship_id) + COUNT(DISTINCT r2.relationship_id) > 0;

CREATE INDEX idx_mv_popular_entities_total_rels ON mv_popular_entity_rankings(total_relationship_count DESC);
CREATE INDEX idx_mv_popular_entities_type ON mv_popular_entity_rankings(node_type, total_relationship_count DESC);
CREATE INDEX idx_mv_popular_entities_confidence ON mv_popular_entity_rankings(confidence_score DESC, total_relationship_count DESC);
```

### Embedding Similarity Cache

Materialized view for caching frequently computed similarity scores.

```sql
CREATE MATERIALIZED VIEW mv_embedding_similarity_cache AS
SELECT
    n1.node_id as source_node_id,
    n2.node_id as target_node_id,
    n1.embedding <=> n2.embedding as similarity_distance,
    1 - (n1.embedding <=> n2.embedding) as similarity_score,
    n1.node_type as source_type,
    n2.node_type as target_type
FROM nodes n1
CROSS JOIN nodes n2
WHERE n1.node_id < n2.node_id
  AND n1.embedding IS NOT NULL
  AND n2.embedding IS NOT NULL
  AND (n1.embedding <=> n2.embedding) < 0.7; -- Only cache similar embeddings

CREATE INDEX idx_mv_similarity_cache_source ON mv_embedding_similarity_cache(source_node_id, similarity_score DESC);
CREATE INDEX idx_mv_similarity_cache_target ON mv_embedding_similarity_cache(target_node_id, similarity_score DESC);
CREATE INDEX idx_mv_similarity_cache_score ON mv_embedding_similarity_cache(similarity_score DESC);
CREATE INDEX idx_mv_similarity_cache_types ON mv_embedding_similarity_cache(source_type, target_type, similarity_score DESC);
```

## Refresh Strategy

All materialized views should be refreshed based on the configuration parameters:

```sql
-- Scheduled refresh function (see database-schema-procs.md for implementation)
-- This would typically be called by a cron job or background worker
SELECT refresh_materialized_views();
```

For more information about the stored procedures and functions that manage these tables, see [database-schema-procs.md](./database-schema-procs.md).
