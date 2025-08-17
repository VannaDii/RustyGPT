# Database Schema: Stored Procedures, Functions and Triggers

This document contains all stored procedures, functions, and triggers for the RustyGPT database schema. These procedures implement core business logic including error handling, confidence calculations, relationship inference, system configuration management, and performance optimization.

## Error Handling Framework

### Error Context Type

```sql
-- Create schema for error handling functions
CREATE SCHEMA IF NOT EXISTS rusty_gpt_errors;

-- Composite type for structured error context
CREATE TYPE rusty_gpt_errors.error_context AS (
    code TEXT,
    context JSONB
);
```

### Parameter Validation Errors (RG1xxx)

```sql
CREATE OR REPLACE FUNCTION rusty_gpt_errors.invalid_parameter(param_name TEXT, param_value TEXT DEFAULT NULL)
RETURNS rusty_gpt_errors.error_context AS $$
DECLARE
    context_details JSONB;
BEGIN
    context_details := jsonb_build_object(
        'parameter_name', param_name,
        'parameter_value', param_value
    );

    RETURN ROW('RG1001', context_details)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION rusty_gpt_errors.missing_required_parameter(param_name TEXT)
RETURNS rusty_gpt_errors.error_context AS $$
DECLARE
    context_details JSONB;
BEGIN
    context_details := jsonb_build_object(
        'parameter_name', param_name
    );

    RETURN ROW('RG1002', context_details)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION rusty_gpt_errors.parameter_out_of_range(param_name TEXT, param_value TEXT, valid_range TEXT)
RETURNS rusty_gpt_errors.error_context AS $$
DECLARE
    context_details JSONB;
BEGIN
    context_details := jsonb_build_object(
        'parameter_name', param_name,
        'parameter_value', param_value,
        'valid_range', valid_range
    );

    RETURN ROW('RG1003', context_details)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;
```

### Data Consistency Errors (RG2xxx)

```sql
CREATE OR REPLACE FUNCTION rusty_gpt_errors.node_not_found(node_id INTEGER)
RETURNS rusty_gpt_errors.error_context AS $$
DECLARE
    context_details JSONB;
BEGIN
    context_details := jsonb_build_object(
        'node_id', node_id
    );

    RETURN ROW('RG2001', context_details)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION rusty_gpt_errors.relationship_not_found(relationship_id INTEGER)
RETURNS rusty_gpt_errors.error_context AS $$
DECLARE
    context_details JSONB;
BEGIN
    context_details := jsonb_build_object(
        'relationship_id', relationship_id
    );

    RETURN ROW('RG2002', context_details)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION rusty_gpt_errors.duplicate_node(node_name TEXT, node_type TEXT)
RETURNS rusty_gpt_errors.error_context AS $$
DECLARE
    context_details JSONB;
BEGIN
    context_details := jsonb_build_object(
        'node_name', node_name,
        'node_type', node_type
    );

    RETURN ROW('RG2003', context_details)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION rusty_gpt_errors.self_reference_relationship()
RETURNS rusty_gpt_errors.error_context AS $$
BEGIN
    RETURN ROW('RG2004', '{}'::jsonb)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;
```

### Confidence Calculation Errors (RG3xxx)

```sql
CREATE OR REPLACE FUNCTION rusty_gpt_errors.confidence_calculation_failed(context_info TEXT)
RETURNS rusty_gpt_errors.error_context AS $$
DECLARE
    context_details JSONB;
BEGIN
    context_details := jsonb_build_object(
        'context', context_info
    );

    RETURN ROW('RG3001', context_details)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION rusty_gpt_errors.insufficient_data_for_confidence(node_id INTEGER)
RETURNS rusty_gpt_errors.error_context AS $$
DECLARE
    context_details JSONB;
BEGIN
    context_details := jsonb_build_object(
        'node_id', node_id
    );

    RETURN ROW('RG3002', context_details)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;
```

### Performance and Resource Errors (RG4xxx)

```sql
CREATE OR REPLACE FUNCTION rusty_gpt_errors.query_timeout(operation TEXT)
RETURNS rusty_gpt_errors.error_context AS $$
DECLARE
    context_details JSONB;
BEGIN
    context_details := jsonb_build_object(
        'operation', operation
    );

    RETURN ROW('RG4001', context_details)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION rusty_gpt_errors.too_many_results(limit_exceeded INTEGER, operation TEXT)
RETURNS rusty_gpt_errors.error_context AS $$
DECLARE
    context_details JSONB;
BEGIN
    context_details := jsonb_build_object(
        'limit_exceeded', limit_exceeded,
        'operation', operation
    );

    RETURN ROW('RG4002', context_details)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;
```

### Transaction Errors (RG5xxx)

```sql
CREATE OR REPLACE FUNCTION rusty_gpt_errors.transaction_conflict()
RETURNS rusty_gpt_errors.error_context AS $$
BEGIN
    RETURN ROW('RG5001', '{}'::jsonb)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION rusty_gpt_errors.deadlock_detected()
RETURNS rusty_gpt_errors.error_context AS $$
BEGIN
    RETURN ROW('RG5002', '{}'::jsonb)::rusty_gpt_errors.error_context;
END;
$$ LANGUAGE plpgsql IMMUTABLE;
```

### Error Message Helper

```sql
CREATE OR REPLACE FUNCTION rusty_gpt_errors.get_error_message(error_code TEXT)
RETURNS TEXT AS $$
BEGIN
    RETURN CASE error_code
        WHEN 'RG1001' THEN 'Invalid parameter'
        WHEN 'RG1002' THEN 'Missing required parameter'
        WHEN 'RG1003' THEN 'Parameter out of range'
        WHEN 'RG2001' THEN 'Node not found'
        WHEN 'RG2002' THEN 'Relationship not found'
        WHEN 'RG2003' THEN 'Duplicate node'
        WHEN 'RG2004' THEN 'Self-reference relationship not allowed'
        WHEN 'RG3001' THEN 'Confidence calculation failed'
        WHEN 'RG3002' THEN 'Insufficient data for confidence'
        WHEN 'RG4001' THEN 'Query timeout'
        WHEN 'RG4002' THEN 'Too many results'
        WHEN 'RG5001' THEN 'Transaction conflict'
        WHEN 'RG5002' THEN 'Deadlock detected'
        ELSE 'Unknown error'
    END;
END;
$$ LANGUAGE plpgsql IMMUTABLE;
```

## Input Validation Framework

### Validation Helper Functions

```sql
CREATE OR REPLACE FUNCTION validate_node_id(p_node_id INTEGER)
RETURNS BOOLEAN AS $$
DECLARE
    v_error_context rusty_gpt_errors.error_context;
BEGIN
    IF p_node_id IS NULL THEN
        v_error_context := rusty_gpt_errors.missing_required_parameter('node_id');
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    IF p_node_id <= 0 THEN
        v_error_context := rusty_gpt_errors.parameter_out_of_range('node_id', p_node_id::TEXT, 'positive integer');
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    -- Check if node exists
    IF NOT EXISTS (SELECT 1 FROM nodes WHERE node_id = p_node_id) THEN
        v_error_context := rusty_gpt_errors.node_not_found(p_node_id);
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION validate_confidence_range(p_value DECIMAL, p_param_name TEXT)
RETURNS BOOLEAN AS $$
DECLARE
    v_error_context rusty_gpt_errors.error_context;
BEGIN
    IF p_value IS NULL THEN
        v_error_context := rusty_gpt_errors.missing_required_parameter(p_param_name);
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    IF p_value < 0.0 OR p_value > 1.0 THEN
        v_error_context := rusty_gpt_errors.parameter_out_of_range(p_param_name, p_value::TEXT, '0.0 to 1.0');
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION validate_text_parameter(p_value TEXT, p_param_name TEXT, p_max_length INTEGER DEFAULT NULL)
RETURNS BOOLEAN AS $$
DECLARE
    v_error_context rusty_gpt_errors.error_context;
BEGIN
    IF p_value IS NULL OR TRIM(p_value) = '' THEN
        v_error_context := rusty_gpt_errors.missing_required_parameter(p_param_name);
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    IF p_max_length IS NOT NULL AND LENGTH(p_value) > p_max_length THEN
        v_error_context := rusty_gpt_errors.parameter_out_of_range(
            p_param_name,
            LENGTH(p_value)::TEXT,
            p_max_length::TEXT
        );
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION validate_embedding_dimension(p_embedding VECTOR, p_expected_dim INTEGER DEFAULT NULL)
RETURNS BOOLEAN AS $$
DECLARE
    v_error_context rusty_gpt_errors.error_context;
    v_expected_dim INTEGER;
BEGIN
    -- Use provided dimension or get from system configuration
    IF p_expected_dim IS NULL THEN
        v_expected_dim := get_config_parameter('vector_embedding_dimension')::INTEGER;
    ELSE
        v_expected_dim := p_expected_dim;
    END IF;

    IF p_embedding IS NOT NULL AND vector_dims(p_embedding) != v_expected_dim THEN
        v_error_context := rusty_gpt_errors.parameter_out_of_range(
            'embedding',
            vector_dims(p_embedding)::TEXT,
            v_expected_dim::TEXT
        );
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;
```

## Confidence Calculation Functions

### Calculate Attribute Confidence

```sql
CREATE OR REPLACE FUNCTION calculate_attribute_confidence(
    p_weight DECIMAL,
    p_source_reliability DECIMAL,
    p_last_verified TIMESTAMP DEFAULT NOW(),
    p_max_age_days INTEGER DEFAULT 365
) RETURNS DECIMAL AS $$
DECLARE
    v_recency_factor DECIMAL;
    v_age_days INTEGER;
BEGIN
    -- Calculate age in days
    v_age_days := EXTRACT(EPOCH FROM (NOW() - p_last_verified)) / 86400;

    -- Calculate recency factor (exponential decay)
    v_recency_factor := EXP(-v_age_days::DECIMAL / p_max_age_days::DECIMAL);

    -- Confidence = weight * source_reliability * recency_factor
    RETURN LEAST(1.0, p_weight * p_source_reliability * v_recency_factor);
END;
$$ LANGUAGE plpgsql IMMUTABLE;
```

### Calculate Relationship Confidence

```sql
CREATE OR REPLACE FUNCTION calculate_relationship_confidence(
    p_source_node_id INTEGER,
    p_target_node_id INTEGER,
    p_relationship_strength DECIMAL DEFAULT 1.0,
    p_source_reliability DECIMAL DEFAULT 1.0
) RETURNS DECIMAL AS $$
DECLARE
    v_shared_attr_confidence DECIMAL;
    v_avg_node_confidence DECIMAL;
    v_final_confidence DECIMAL;
    v_error_context rusty_gpt_errors.error_context;
BEGIN
    -- Input validation
    IF p_source_node_id IS NULL OR p_target_node_id IS NULL THEN
        v_error_context := rusty_gpt_errors.missing_required_parameter('node_id');
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    IF p_source_node_id = p_target_node_id THEN
        v_error_context := rusty_gpt_errors.self_reference_relationship();
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    -- Validate confidence ranges
    IF p_relationship_strength < 0.0 OR p_relationship_strength > 1.0 THEN
        v_error_context := rusty_gpt_errors.parameter_out_of_range('relationship_strength', p_relationship_strength::TEXT, '0.0 to 1.0');
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    IF p_source_reliability < 0.0 OR p_source_reliability > 1.0 THEN
        v_error_context := rusty_gpt_errors.parameter_out_of_range('source_reliability', p_source_reliability::TEXT, '0.0 to 1.0');
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    BEGIN
        -- Calculate average confidence of shared attributes
        SELECT COALESCE(AVG(
            calculate_attribute_confidence(
                na1.weight,
                na1.source_reliability,
                na1.last_verified
            ) * calculate_attribute_confidence(
                na2.weight,
                na2.source_reliability,
                na2.last_verified
            )
        ), 0.5)
        INTO v_shared_attr_confidence
        FROM node_attributes na1
        JOIN node_attributes na2 ON (
            na1.attribute_type = na2.attribute_type
            AND na1.attribute_value = na2.attribute_value
        )
        WHERE na1.node_id = p_source_node_id
            AND na2.node_id = p_target_node_id;

        -- Calculate average node confidence using materialized scores
        SELECT AVG(n.confidence_score) INTO v_avg_node_confidence
        FROM nodes n
        WHERE n.node_id IN (p_source_node_id, p_target_node_id);

        -- Relationship confidence = strength * source_reliability * shared_attributes * node_confidence
        v_final_confidence := LEAST(1.0,
            p_relationship_strength *
            p_source_reliability *
            v_shared_attr_confidence *
            v_avg_node_confidence
        );

        RETURN v_final_confidence;

    EXCEPTION
        WHEN division_by_zero THEN
            v_error_context := rusty_gpt_errors.confidence_calculation_failed('division by zero in relationship confidence calculation');
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
        WHEN numeric_value_out_of_range THEN
            v_error_context := rusty_gpt_errors.confidence_calculation_failed('numeric overflow in relationship confidence calculation');
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
        WHEN OTHERS THEN
            v_error_context := rusty_gpt_errors.confidence_calculation_failed(SQLERRM);
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END;
END;
$$ LANGUAGE plpgsql;
```

### Calculate Node Confidence

```sql
CREATE OR REPLACE FUNCTION calculate_node_confidence(
    p_node_id INTEGER
) RETURNS DECIMAL AS $$
DECLARE
    v_avg_confidence DECIMAL;
    v_attribute_count INTEGER;
    v_error_context rusty_gpt_errors.error_context;
BEGIN
    -- Input validation
    IF p_node_id IS NULL THEN
        v_error_context := rusty_gpt_errors.missing_required_parameter('node_id');
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    IF p_node_id <= 0 THEN
        v_error_context := rusty_gpt_errors.parameter_out_of_range('node_id', p_node_id::TEXT, 'positive integer');
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    BEGIN
        -- Check if node exists
        IF NOT EXISTS (SELECT 1 FROM nodes WHERE node_id = p_node_id) THEN
            v_error_context := rusty_gpt_errors.node_not_found(p_node_id);
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
        END IF;

        -- Count attributes for validation
        SELECT COUNT(*) INTO v_attribute_count
        FROM node_attributes na
        WHERE na.node_id = p_node_id;

        -- Handle nodes with no attributes
        IF v_attribute_count = 0 THEN
            v_error_context := rusty_gpt_errors.insufficient_data_for_confidence(p_node_id);
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
        END IF;

        -- Calculate average confidence of all node attributes
        SELECT COALESCE(AVG(
            calculate_attribute_confidence(
                na.weight,
                na.source_reliability,
                na.last_verified
            )
        ), 0.5)
        INTO v_avg_confidence
        FROM node_attributes na
        WHERE na.node_id = p_node_id;

        -- Ensure result is within valid range
        v_avg_confidence := GREATEST(0.0, LEAST(1.0, v_avg_confidence));

        RETURN v_avg_confidence;

    EXCEPTION
        WHEN division_by_zero THEN
            v_error_context := rusty_gpt_errors.confidence_calculation_failed('division by zero in node confidence calculation');
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
        WHEN numeric_value_out_of_range THEN
            v_error_context := rusty_gpt_errors.confidence_calculation_failed('numeric overflow in node confidence calculation');
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
        WHEN OTHERS THEN
            v_error_context := rusty_gpt_errors.confidence_calculation_failed(SQLERRM);
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END;
END;
$$ LANGUAGE plpgsql;
```

## Materialized Confidence Management

### Update Node Confidence Score

```sql
CREATE OR REPLACE FUNCTION update_node_confidence_score(
    p_node_id INTEGER
) RETURNS DECIMAL AS $$
DECLARE
    v_new_confidence DECIMAL;
    v_error_context rusty_gpt_errors.error_context;
BEGIN
    -- Input validation
    IF p_node_id IS NULL THEN
        v_error_context := rusty_gpt_errors.missing_required_parameter('node_id');
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    -- Check if node exists
    IF NOT EXISTS (SELECT 1 FROM nodes WHERE node_id = p_node_id) THEN
        v_error_context := rusty_gpt_errors.node_not_found(p_node_id);
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    BEGIN
        -- Calculate new confidence using existing function
        v_new_confidence := calculate_node_confidence(p_node_id);

        -- Update materialized confidence score
        UPDATE nodes
        SET confidence_score = v_new_confidence,
            confidence_last_updated = NOW()
        WHERE node_id = p_node_id;

        RETURN v_new_confidence;

    EXCEPTION
        WHEN OTHERS THEN
            v_error_context := rusty_gpt_errors.confidence_calculation_failed(
                'Failed to update node confidence for node_id: ' || p_node_id::TEXT
            );
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END;
END;
$$ LANGUAGE plpgsql;
```

### Update Relationship Confidence Score

```sql
CREATE OR REPLACE FUNCTION update_relationship_confidence_score(
    p_relationship_id INTEGER
) RETURNS DECIMAL AS $$
DECLARE
    v_new_confidence DECIMAL;
    v_source_node_id INTEGER;
    v_target_node_id INTEGER;
    v_strength DECIMAL;
    v_source_reliability DECIMAL;
    v_error_context rusty_gpt_errors.error_context;
BEGIN
    -- Input validation
    IF p_relationship_id IS NULL THEN
        v_error_context := rusty_gpt_errors.missing_required_parameter('relationship_id');
        RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END IF;

    BEGIN
        -- Get relationship details
        SELECT source_node_id, target_node_id, strength, source_reliability
        INTO v_source_node_id, v_target_node_id, v_strength, v_source_reliability
        FROM relationships
        WHERE relationship_id = p_relationship_id;

        IF NOT FOUND THEN
            v_error_context := rusty_gpt_errors.relationship_not_found(p_relationship_id);
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
        END IF;

        -- Calculate new confidence using existing function
        v_new_confidence := calculate_relationship_confidence(
            v_source_node_id,
            v_target_node_id,
            v_strength,
            v_source_reliability
        );

        -- Update materialized confidence score
        UPDATE relationships
        SET confidence_score = v_new_confidence,
            confidence_last_updated = NOW()
        WHERE relationship_id = p_relationship_id;

        RETURN v_new_confidence;

    EXCEPTION
        WHEN OTHERS THEN
            v_error_context := rusty_gpt_errors.confidence_calculation_failed(
                'Failed to update relationship confidence for relationship_id: ' || p_relationship_id::TEXT
            );
            RAISE EXCEPTION '%', rusty_gpt_errors.get_error_message(v_error_context.code);
    END;
END;
$$ LANGUAGE plpgsql;
```

### Batch Update Node Confidence Scores

```sql
CREATE OR REPLACE FUNCTION batch_update_node_confidence_scores(
    p_node_ids INTEGER[] DEFAULT NULL,
    p_batch_size INTEGER DEFAULT 100,
    p_max_age_hours INTEGER DEFAULT 24
) RETURNS TABLE(
    node_id INTEGER,
    old_confidence DECIMAL,
    new_confidence DECIMAL,
    updated_at TIMESTAMP
) AS $$
DECLARE
    v_node_record RECORD;
    v_batch_count INTEGER := 0;
    v_total_updated INTEGER := 0;
BEGIN
    -- Set batch processing parameters
    SET work_mem = '256MB';
    SET maintenance_work_mem = '512MB';

    -- If no specific nodes provided, update stale confidence scores
    IF p_node_ids IS NULL THEN
        FOR v_node_record IN
            SELECT n.node_id, n.confidence_score as old_score
            FROM nodes n
            WHERE n.confidence_last_updated < (NOW() - INTERVAL '1 hour' * p_max_age_hours)
            ORDER BY n.confidence_last_updated ASC
            LIMIT p_batch_size
        LOOP
            -- Update individual node confidence
            DECLARE
                v_new_score DECIMAL;
            BEGIN
                v_new_score := update_node_confidence_score(v_node_record.node_id);

                -- Return result
                node_id := v_node_record.node_id;
                old_confidence := v_node_record.old_score;
                new_confidence := v_new_score;
                updated_at := NOW();

                RETURN NEXT;

                v_batch_count := v_batch_count + 1;
                v_total_updated := v_total_updated + 1;

                -- Commit batch periodically to avoid long transactions
                IF v_batch_count >= p_batch_size THEN
                    COMMIT;
                    v_batch_count := 0;
                END IF;
            END;
        END LOOP;
    ELSE
        -- Update specific nodes
        FOR v_node_record IN
            SELECT n.node_id, n.confidence_score as old_score
            FROM nodes n
            WHERE n.node_id = ANY(p_node_ids)
        LOOP
            DECLARE
                v_new_score DECIMAL;
            BEGIN
                v_new_score := update_node_confidence_score(v_node_record.node_id);

                node_id := v_node_record.node_id;
                old_confidence := v_node_record.old_score;
                new_confidence := v_new_score;
                updated_at := NOW();

                RETURN NEXT;
                v_total_updated := v_total_updated + 1;
            END;
        END LOOP;
    END IF;

    -- Log batch update completion
    RAISE NOTICE 'Batch update completed: % nodes updated', v_total_updated;
END;
$$ LANGUAGE plpgsql;
```

### Background Confidence Update Job

```sql
CREATE OR REPLACE FUNCTION background_confidence_update_job(
    p_batch_size INTEGER DEFAULT 500,
    p_max_runtime_minutes INTEGER DEFAULT 30
) RETURNS TABLE(
    operation_type TEXT,
    records_processed INTEGER,
    avg_processing_time_ms DECIMAL,
    completion_time TIMESTAMP
) AS $$
DECLARE
    v_start_time TIMESTAMP := NOW();
    v_end_time TIMESTAMP;
    v_nodes_updated INTEGER := 0;
    v_relationships_updated INTEGER := 0;
    v_node_batch_time DECIMAL;
    v_rel_batch_time DECIMAL;
BEGIN
    -- Set runtime limit
    v_end_time := v_start_time + INTERVAL '1 minute' * p_max_runtime_minutes;

    -- Phase 1: Update stale node confidence scores
    DECLARE
        v_phase_start TIMESTAMP := NOW();
    BEGIN
        SELECT COUNT(*) INTO v_nodes_updated
        FROM batch_update_node_confidence_scores(NULL, p_batch_size, 24);

        v_node_batch_time := EXTRACT(EPOCH FROM (NOW() - v_phase_start)) * 1000;

        operation_type := 'node_confidence_update';
        records_processed := v_nodes_updated;
        avg_processing_time_ms := CASE WHEN v_nodes_updated > 0 THEN v_node_batch_time / v_nodes_updated ELSE 0 END;
        completion_time := NOW();
        RETURN NEXT;
    END;

    -- Phase 2: Update relationship confidence scores if time permits
    IF NOW() < v_end_time THEN
        DECLARE
            v_phase_start TIMESTAMP := NOW();
            v_rel_record RECORD;
        BEGIN
            FOR v_rel_record IN
                SELECT relationship_id
                FROM relationships r
                WHERE r.confidence_last_updated < (NOW() - INTERVAL '24 hours')
                ORDER BY r.confidence_last_updated ASC
                LIMIT p_batch_size
            LOOP
                PERFORM update_relationship_confidence_score(v_rel_record.relationship_id);
                v_relationships_updated := v_relationships_updated + 1;

                -- Check runtime limit
                IF NOW() > v_end_time THEN
                    EXIT;
                END IF;
            END LOOP;

            v_rel_batch_time := EXTRACT(EPOCH FROM (NOW() - v_phase_start)) * 1000;

            operation_type := 'relationship_confidence_update';
            records_processed := v_relationships_updated;
            avg_processing_time_ms := CASE WHEN v_relationships_updated > 0 THEN v_rel_batch_time / v_relationships_updated ELSE 0 END;
            completion_time := NOW();
            RETURN NEXT;
        END;
    END IF;

    -- Summary
    operation_type := 'total_background_update';
    records_processed := v_nodes_updated + v_relationships_updated;
    avg_processing_time_ms := EXTRACT(EPOCH FROM (NOW() - v_start_time)) * 1000;
    completion_time := NOW();
    RETURN NEXT;
END;
$$ LANGUAGE plpgsql;
```

## Confidence Update Triggers

### Trigger Functions

```sql
-- Trigger function to update node confidence when attributes change
CREATE OR REPLACE FUNCTION trigger_update_node_confidence()
RETURNS TRIGGER AS $$
BEGIN
    -- Update confidence score for affected node
    PERFORM update_node_confidence_score(
        CASE
            WHEN TG_OP = 'DELETE' THEN OLD.node_id
            ELSE NEW.node_id
        END
    );

    RETURN CASE
        WHEN TG_OP = 'DELETE' THEN OLD
        ELSE NEW
    END;
END;
$$ LANGUAGE plpgsql;

-- Trigger function to update relationship confidence when relationships change
CREATE OR REPLACE FUNCTION trigger_update_relationship_confidence()
RETURNS TRIGGER AS $$
BEGIN
    -- Update confidence score for the relationship
    PERFORM update_relationship_confidence_score(
        CASE
            WHEN TG_OP = 'DELETE' THEN OLD.relationship_id
            ELSE NEW.relationship_id
        END
    );

    RETURN CASE
            WHEN TG_OP = 'DELETE' THEN OLD
            ELSE NEW
        END;
END;
$$ LANGUAGE plpgsql;
```

### Create Triggers

```sql
-- Create triggers for node_attributes table
CREATE TRIGGER tr_node_attributes_confidence_update
    AFTER INSERT OR UPDATE OR DELETE ON node_attributes
    FOR EACH ROW
    EXECUTE FUNCTION trigger_update_node_confidence();

-- Create trigger for relationships table
CREATE TRIGGER tr_relationships_confidence_update
    AFTER INSERT OR UPDATE ON relationships
    FOR EACH ROW
    EXECUTE FUNCTION trigger_update_relationship_confidence();
```

## Relationship Inference Functions

### Queue Relationship Inference

```sql
CREATE OR REPLACE FUNCTION queue_relationship_inference(
    p_node_id INTEGER,
    p_priority INTEGER DEFAULT 5,
    p_batch_id TEXT DEFAULT NULL
) RETURNS BOOLEAN AS $$
DECLARE
    v_batch_id TEXT;
BEGIN
    -- Generate batch ID if not provided
    v_batch_id := COALESCE(p_batch_id, 'auto_' || EXTRACT(EPOCH FROM NOW())::TEXT);

    -- Insert into queue if not already pending
    INSERT INTO relationship_inference_queue (node_id, priority, batch_id, status, queued_at)
    VALUES (p_node_id, p_priority, v_batch_id, 'pending', NOW())
    ON CONFLICT (node_id)
    DO UPDATE SET
        priority = GREATEST(relationship_inference_queue.priority, EXCLUDED.priority),
        batch_id = EXCLUDED.batch_id,
        queued_at = NOW()
    WHERE relationship_inference_queue.status NOT IN ('processing', 'completed');

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;
```

### Relationship Inference Triggers

```sql
-- Trigger function for node attributes changes
CREATE OR REPLACE FUNCTION trigger_queue_relationship_inference()
RETURNS TRIGGER AS $$
DECLARE
    v_affected_node_id INTEGER;
    v_priority INTEGER;
BEGIN
    -- Determine affected node and priority based on operation
    v_affected_node_id := CASE
        WHEN TG_OP = 'DELETE' THEN OLD.node_id
        ELSE NEW.node_id
    END;

    -- Set priority based on operation type
    v_priority := CASE
        WHEN TG_OP = 'INSERT' THEN 7  -- New attributes get high priority
        WHEN TG_OP = 'UPDATE' THEN 5  -- Updates get medium priority
        WHEN TG_OP = 'DELETE' THEN 6  -- Deletions get higher priority for cleanup
        ELSE 5
    END;

    -- Queue relationship inference for the affected node
    PERFORM queue_relationship_inference(v_affected_node_id, v_priority);

    -- For significant attribute changes, also queue related nodes
    IF TG_OP IN ('INSERT', 'DELETE') OR
       (TG_OP = 'UPDATE' AND (OLD.attribute_value IS DISTINCT FROM NEW.attribute_value OR
                             OLD.confidence IS DISTINCT FROM NEW.confidence)) THEN

        -- Queue nodes that share this attribute for re-inference
        INSERT INTO relationship_inference_queue (node_id, priority, batch_id, status, queued_at)
        SELECT DISTINCT
            na.node_id,
            4, -- Lower priority for cascaded updates
            'cascade_' || EXTRACT(EPOCH FROM NOW())::TEXT,
            'pending',
            NOW()
        FROM node_attributes na
        WHERE na.attribute_key = COALESCE(NEW.attribute_key, OLD.attribute_key)
          AND na.attribute_value = COALESCE(NEW.attribute_value, OLD.attribute_value)
          AND na.node_id != v_affected_node_id
          AND NOT EXISTS (
              SELECT 1 FROM relationship_inference_queue riq
              WHERE riq.node_id = na.node_id
                AND riq.status IN ('pending', 'processing')
          )
        LIMIT 20; -- Prevent excessive queue growth
    END IF;

    RETURN CASE
        WHEN TG_OP = 'DELETE' THEN OLD
        ELSE NEW
    END;

EXCEPTION
    WHEN OTHERS THEN
        -- Log error but don't prevent the original operation
        RAISE WARNING 'Failed to queue relationship inference for node %: %', v_affected_node_id, SQLERRM;
        RETURN CASE
            WHEN TG_OP = 'DELETE' THEN OLD
            ELSE NEW
        END;
END;
$$ LANGUAGE plpgsql;

-- Trigger function for node changes
CREATE OR REPLACE FUNCTION trigger_node_relationship_inference()
RETURNS TRIGGER AS $$
BEGIN
    -- Queue relationship inference for new nodes or significant updates
    IF TG_OP = 'INSERT' OR
       (TG_OP = 'UPDATE' AND (OLD.node_type IS DISTINCT FROM NEW.node_type OR
                             OLD.name IS DISTINCT FROM NEW.name OR
                             OLD.embedding IS DISTINCT FROM NEW.embedding)) THEN

        -- Queue the node for relationship inference
        PERFORM queue_relationship_inference(NEW.node_id, 6);
    END IF;

    RETURN NEW;

EXCEPTION
    WHEN OTHERS THEN
        -- Log error but don't prevent the original operation
        RAISE WARNING 'Failed to queue relationship inference for node %: %', NEW.node_id, SQLERRM;
        RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

### Create Relationship Inference Triggers

```sql
-- Create triggers for node_attributes table
DROP TRIGGER IF EXISTS tr_node_attributes_relationship_inference ON node_attributes;
CREATE TRIGGER tr_node_attributes_relationship_inference
    AFTER INSERT OR UPDATE OR DELETE ON node_attributes
    FOR EACH ROW
    EXECUTE FUNCTION trigger_queue_relationship_inference();

-- Create trigger for nodes table
DROP TRIGGER IF EXISTS tr_nodes_relationship_inference ON nodes;
CREATE TRIGGER tr_nodes_relationship_inference
    AFTER INSERT OR UPDATE ON nodes
    FOR EACH ROW
    EXECUTE FUNCTION trigger_node_relationship_inference();
```

## System Configuration Management

### Configuration Management Functions

```sql
-- Get configuration parameter with type conversion and validation
CREATE OR REPLACE FUNCTION get_config_parameter(p_parameter_name TEXT)
RETURNS TEXT AS $$
DECLARE
    v_value TEXT;
    v_config RECORD;
BEGIN
    -- Get configuration parameter with metadata
    SELECT parameter_value, parameter_type, description
    INTO v_config
    FROM system_config
    WHERE parameter_name = p_parameter_name;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Configuration parameter % not found', p_parameter_name
            USING ERRCODE = 'RG2001',
                  DETAIL = format('Parameter "%s" does not exist in system_config', p_parameter_name);
    END IF;

    RETURN v_config.parameter_value;
END;
$$ LANGUAGE plpgsql STABLE;

-- Typed parameter getters for common types
CREATE OR REPLACE FUNCTION get_config_integer(p_parameter_name TEXT)
RETURNS INTEGER AS $$
DECLARE
    v_value TEXT;
BEGIN
    v_value := get_config_parameter(p_parameter_name);
    RETURN v_value::INTEGER;
EXCEPTION
    WHEN invalid_text_representation THEN
        RAISE EXCEPTION 'Configuration parameter % has invalid integer value: %', p_parameter_name, v_value
            USING ERRCODE = 'RG2002';
END;
$$ LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION get_config_float(p_parameter_name TEXT)
RETURNS FLOAT AS $$
DECLARE
    v_value TEXT;
BEGIN
    v_value := get_config_parameter(p_parameter_name);
    RETURN v_value::FLOAT;
EXCEPTION
    WHEN invalid_text_representation THEN
        RAISE EXCEPTION 'Configuration parameter % has invalid float value: %', p_parameter_name, v_value
            USING ERRCODE = 'RG2003';
END;
$$ LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION get_config_boolean(p_parameter_name TEXT)
RETURNS BOOLEAN AS $$
DECLARE
    v_value TEXT;
BEGIN
    v_value := get_config_parameter(p_parameter_name);
    RETURN v_value::BOOLEAN;
EXCEPTION
    WHEN invalid_text_representation THEN
        RAISE EXCEPTION 'Configuration parameter % has invalid boolean value: %', p_parameter_name, v_value
            USING ERRCODE = 'RG2004';
END;
$$ LANGUAGE plpgsql STABLE;

-- Set configuration parameter with validation and audit logging
CREATE OR REPLACE FUNCTION set_config_parameter(
    p_parameter_name TEXT,
    p_new_value TEXT,
    p_updated_by TEXT DEFAULT 'system'
) RETURNS BOOLEAN AS $$
DECLARE
    v_config RECORD;
    v_old_value TEXT;
    v_numeric_value NUMERIC;
BEGIN
    -- Get current configuration with validation rules
    SELECT parameter_value, parameter_type, min_value, max_value, requires_restart
    INTO v_config
    FROM system_config
    WHERE parameter_name = p_parameter_name;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Configuration parameter % not found', p_parameter_name
            USING ERRCODE = 'RG2001';
    END IF;

    v_old_value := v_config.parameter_value;

    -- Validate new value based on parameter type
    IF v_config.parameter_type = 'integer' THEN
        BEGIN
            v_numeric_value := p_new_value::INTEGER;
        EXCEPTION
            WHEN invalid_text_representation THEN
                RAISE EXCEPTION 'Invalid integer value for parameter %: %', p_parameter_name, p_new_value
                    USING ERRCODE = 'RG2002';
        END;

    ELSIF v_config.parameter_type = 'float' THEN
        BEGIN
            v_numeric_value := p_new_value::FLOAT;
        EXCEPTION
            WHEN invalid_text_representation THEN
                RAISE EXCEPTION 'Invalid float value for parameter %: %', p_parameter_name, p_new_value
                    USING ERRCODE = 'RG2003';
        END;

    ELSIF v_config.parameter_type = 'boolean' THEN
        IF p_new_value NOT IN ('true', 'false', 't', 'f', 'yes', 'no', 'y', 'n', '1', '0') THEN
            RAISE EXCEPTION 'Invalid boolean value for parameter %: %', p_parameter_name, p_new_value
                USING ERRCODE = 'RG2004';
        END IF;
    END IF;

    -- Validate range constraints for numeric types
    IF v_config.parameter_type IN ('integer', 'float') AND v_config.min_value IS NOT NULL THEN
        IF v_numeric_value < v_config.min_value::NUMERIC THEN
            RAISE EXCEPTION 'Value % for parameter % is below minimum %',
                p_new_value, p_parameter_name, v_config.min_value
                USING ERRCODE = 'RG2005';
        END IF;
    END IF;

    IF v_config.parameter_type IN ('integer', 'float') AND v_config.max_value IS NOT NULL THEN
        IF v_numeric_value > v_config.max_value::NUMERIC THEN
            RAISE EXCEPTION 'Value % for parameter % exceeds maximum %',
                p_new_value, p_parameter_name, v_config.max_value
                USING ERRCODE = 'RG2006';
        END IF;
    END IF;

    -- Update the parameter
    UPDATE system_config
    SET parameter_value = p_new_value,
        updated_at = NOW(),
        updated_by = p_updated_by
    WHERE parameter_name = p_parameter_name;

    -- Log configuration change if value actually changed
    IF v_old_value != p_new_value THEN
        INSERT INTO config_audit_log (parameter_name, old_value, new_value, updated_by, requires_restart)
        VALUES (p_parameter_name, v_old_value, p_new_value, p_updated_by, v_config.requires_restart);

        -- Warn if restart is required
        IF v_config.requires_restart THEN
            RAISE NOTICE 'Configuration change for % requires database restart to take effect', p_parameter_name;
        END IF;
    END IF;

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;
```

## Vector Index Optimization Functions

### Calculate Optimal IVFFlat Lists Parameter

```sql
CREATE OR REPLACE FUNCTION calculate_optimal_ivfflat_lists(
    p_table_name TEXT,
    p_column_name TEXT DEFAULT 'embedding'
) RETURNS INTEGER AS $$
DECLARE
    v_row_count BIGINT;
    v_optimal_lists INTEGER;
    v_min_lists INTEGER := 10;
    v_max_lists INTEGER := 10000;
BEGIN
    -- Get current row count for the table
    EXECUTE format('SELECT COUNT(*) FROM %I WHERE %I IS NOT NULL', p_table_name, p_column_name)
    INTO v_row_count;

    -- Calculate optimal lists parameter: approximately sqrt(row_count)
    -- With bounds checking and reasonable defaults
    IF v_row_count = 0 THEN
        v_optimal_lists := v_min_lists;
    ELSIF v_row_count < 100 THEN
        v_optimal_lists := v_min_lists;
    ELSIF v_row_count > 100000000 THEN  -- 100M rows
        v_optimal_lists := v_max_lists;
    ELSE
        v_optimal_lists := GREATEST(v_min_lists, LEAST(v_max_lists, ROUND(SQRT(v_row_count))::INTEGER));
    END IF;

    RETURN v_optimal_lists;
END;
$$ LANGUAGE plpgsql STABLE;
```

### Update Vector Index Configuration

```sql
CREATE OR REPLACE FUNCTION update_vector_index_config(
    p_table_name TEXT,
    p_column_name TEXT DEFAULT 'embedding',
    p_rebuild_index BOOLEAN DEFAULT FALSE
) RETURNS JSONB AS $$
DECLARE
    v_current_lists INTEGER;
    v_optimal_lists INTEGER;
    v_config_param TEXT;
    v_index_name TEXT;
    v_operator_class TEXT := 'vector_cosine_ops';
    v_result JSONB;
BEGIN
    -- Determine configuration parameter name based on table
    v_config_param := CASE p_table_name
        WHEN 'nodes' THEN 'ivfflat_lists_nodes'
        WHEN 'external_resources' THEN 'ivfflat_lists_external_resources'
        ELSE 'ivfflat_lists_' || p_table_name
    END;

    -- Get current configuration
    v_current_lists := get_config_integer(v_config_param);

    -- Calculate optimal lists parameter
    v_optimal_lists := calculate_optimal_ivfflat_lists(p_table_name, p_column_name);

    -- Build result object
    v_result := jsonb_build_object(
        'table_name', p_table_name,
        'column_name', p_column_name,
        'current_lists', v_current_lists,
        'optimal_lists', v_optimal_lists,
        'improvement_needed', ABS(v_optimal_lists - v_current_lists) > (v_current_lists * 0.2),
        'rebuild_recommended', ABS(v_optimal_lists - v_current_lists) > (v_current_lists * 0.5)
    );

    -- Update configuration if significantly different (>20% change)
    IF ABS(v_optimal_lists - v_current_lists) > (v_current_lists * 0.2) THEN
        PERFORM set_config_parameter(v_config_param, v_optimal_lists::TEXT, 'auto_optimizer');

        v_result := v_result || jsonb_build_object('config_updated', true);

        -- Rebuild index if requested and improvement is significant
        IF p_rebuild_index AND ABS(v_optimal_lists - v_current_lists) > (v_current_lists * 0.5) THEN
            v_index_name := format('idx_%s_%s', p_table_name, p_column_name);

            -- Drop and recreate the index with new parameters
            EXECUTE format('DROP INDEX IF EXISTS %I', v_index_name);
            EXECUTE format(
                'CREATE INDEX %I ON %I USING ivfflat(%I %s) WITH (lists = %s)',
                v_index_name, p_table_name, p_column_name, v_operator_class, v_optimal_lists
            );

            v_result := v_result || jsonb_build_object(
                'index_rebuilt', true,
                'new_index_name', v_index_name
            );
        END IF;
    ELSE
        v_result := v_result || jsonb_build_object('config_updated', false);
    END IF;

    RETURN v_result;
END;
$$ LANGUAGE plpgsql;
```

### Batch Update All Vector Indexes

```sql
CREATE OR REPLACE FUNCTION update_all_vector_indexes(p_rebuild_indexes BOOLEAN DEFAULT FALSE)
RETURNS JSONB AS $$
DECLARE
    v_results JSONB := '[]'::JSONB;
    v_table_config RECORD;
    v_table_result JSONB;
BEGIN
    -- Process each vector-enabled table
    FOR v_table_config IN
        SELECT DISTINCT
            CASE
                WHEN schemaname != 'public' THEN schemaname || '.' || tablename
                ELSE tablename
            END as full_table_name,
            tablename as table_name
        FROM pg_indexes
        WHERE indexdef LIKE '%ivfflat%'
          AND indexdef LIKE '%embedding%'
    LOOP
        BEGIN
            v_table_result := update_vector_index_config(
                v_table_config.table_name,
                'embedding',
                p_rebuild_indexes
            );

            v_results := v_results || jsonb_build_array(v_table_result);

        EXCEPTION WHEN OTHERS THEN
            -- Log error but continue with other tables
            v_table_result := jsonb_build_object(
                'table_name', v_table_config.table_name,
                'error', SQLERRM,
                'config_updated', false
            );
            v_results := v_results || jsonb_build_array(v_table_result);
        END;
    END LOOP;

    RETURN jsonb_build_object(
        'updated_at', NOW(),
        'rebuild_indexes', p_rebuild_indexes,
        'results', v_results
    );
END;
$$ LANGUAGE plpgsql;
```

## Database Performance Configuration

### Apply Performance Settings

```sql
CREATE OR REPLACE FUNCTION apply_performance_config()
RETURNS JSONB AS $$
DECLARE
    v_applied_settings JSONB := '[]'::JSONB;
    v_setting RECORD;
    v_current_value TEXT;
    v_requires_restart BOOLEAN := FALSE;
BEGIN
    -- Apply performance-related configuration parameters
    FOR v_setting IN
        SELECT parameter_name, parameter_value, requires_restart
        FROM system_config
        WHERE category = 'performance'
          AND parameter_name IN (
              'maintenance_work_mem', 'effective_cache_size', 'shared_buffers',
              'work_mem', 'random_page_cost'
          )
    LOOP
        BEGIN
            -- Get current PostgreSQL setting
            SELECT setting INTO v_current_value
            FROM pg_settings
            WHERE name = v_setting.parameter_name;

            -- Apply setting if different from current value
            IF v_current_value IS NULL OR v_current_value != v_setting.parameter_value THEN
                EXECUTE format('SET %I = %L', v_setting.parameter_name, v_setting.parameter_value);

                v_applied_settings := v_applied_settings || jsonb_build_array(
                    jsonb_build_object(
                        'parameter', v_setting.parameter_name,
                        'old_value', v_current_value,
                        'new_value', v_setting.parameter_value,
                        'requires_restart', v_setting.requires_restart
                    )
                );

                IF v_setting.requires_restart THEN
                    v_requires_restart := TRUE;
                END IF;
            END IF;

        EXCEPTION WHEN OTHERS THEN
            -- Log error for individual setting but continue
            v_applied_settings := v_applied_settings || jsonb_build_array(
                jsonb_build_object(
                    'parameter', v_setting.parameter_name,
                    'error', SQLERRM,
                    'applied', false
                )
            );
        END;
    END LOOP;

    RETURN jsonb_build_object(
        'applied_at', NOW(),
        'requires_restart', v_requires_restart,
        'applied_settings', v_applied_settings
    );
END;
$$ LANGUAGE plpgsql;
```

## Configuration Validation and Health Check

### Validate System Configuration

```sql
CREATE OR REPLACE FUNCTION validate_system_config()
RETURNS JSONB AS $$
DECLARE
    v_validation_results JSONB := '[]'::JSONB;
    v_config RECORD;
    v_is_valid BOOLEAN;
    v_error_message TEXT;
    v_total_configs INTEGER := 0;
    v_valid_configs INTEGER := 0;
BEGIN
    -- Validate each configuration parameter
    FOR v_config IN
        SELECT parameter_name, parameter_value, parameter_type, min_value, max_value, description
        FROM system_config
        ORDER BY category, parameter_name
    LOOP
        v_total_configs := v_total_configs + 1;
        v_is_valid := TRUE;
        v_error_message := NULL;

        BEGIN
            -- Type validation
            IF v_config.parameter_type = 'integer' THEN
                PERFORM v_config.parameter_value::INTEGER;
            ELSIF v_config.parameter_type = 'float' THEN
                PERFORM v_config.parameter_value::FLOAT;
            ELSIF v_config.parameter_type = 'boolean' THEN
                PERFORM v_config.parameter_value::BOOLEAN;
            END IF;

            -- Range validation for numeric types
            IF v_config.parameter_type IN ('integer', 'float') THEN
                IF v_config.min_value IS NOT NULL AND v_config.parameter_value::NUMERIC < v_config.min_value::NUMERIC THEN
                    v_is_valid := FALSE;
                    v_error_message := format('Value %s below minimum %s', v_config.parameter_value, v_config.min_value);
                ELSIF v_config.max_value IS NOT NULL AND v_config.parameter_value::NUMERIC > v_config.max_value::NUMERIC THEN
                    v_is_valid := FALSE;
                    v_error_message := format('Value %s exceeds maximum %s', v_config.parameter_value, v_config.max_value);
                END IF;
            END IF;

        EXCEPTION WHEN OTHERS THEN
            v_is_valid := FALSE;
            v_error_message := SQLERRM;
        END;

        IF v_is_valid THEN
            v_valid_configs := v_valid_configs + 1;
        END IF;

        v_validation_results := v_validation_results || jsonb_build_array(
            jsonb_build_object(
                'parameter_name', v_config.parameter_name,
                'parameter_value', v_config.parameter_value,
                'parameter_type', v_config.parameter_type,
                'is_valid', v_is_valid,
                'error_message', v_error_message
            )
        );
    END LOOP;

    RETURN jsonb_build_object(
        'validation_timestamp', NOW(),
        'total_parameters', v_total_configs,
        'valid_parameters', v_valid_configs,
        'validation_success', v_valid_configs = v_total_configs,
        'validation_details', v_validation_results
    );
END;
$$ LANGUAGE plpgsql;
```

### Auto-optimize Vector Indexes

```sql
CREATE OR REPLACE FUNCTION auto_optimize_vector_indexes()
RETURNS JSONB AS $$
DECLARE
    v_optimization_results JSONB;
    v_threshold INTEGER;
    v_nodes_count BIGINT;
    v_resources_count BIGINT;
BEGIN
    -- Get rebuild threshold from configuration
    v_threshold := get_config_integer('vector_index_rebuild_threshold');

    -- Check if optimization is needed based on data growth
    SELECT COUNT(*) INTO v_nodes_count FROM nodes WHERE embedding IS NOT NULL;
    SELECT COUNT(*) INTO v_resources_count FROM external_resources WHERE embedding IS NOT NULL;

    -- Only run optimization if we have significant data
    IF v_nodes_count >= v_threshold OR v_resources_count >= v_threshold THEN
        -- Update all vector indexes with rebuild if significant improvement is available
        v_optimization_results := update_all_vector_indexes(true);

        -- Log optimization run
        INSERT INTO config_audit_log (parameter_name, old_value, new_value, updated_by)
        VALUES ('auto_optimization', 'vector_indexes', 'completed', 'auto_optimizer');

        RAISE NOTICE 'Vector indexes optimized successfully';
    ELSE
        v_optimization_results := jsonb_build_object(
            'optimization_skipped', true,
            'reason', 'Insufficient data for optimization',
            'nodes_count', v_nodes_count,
            'resources_count', v_resources_count,
            'threshold', v_threshold
        );
    END IF;

    RETURN v_optimization_results;
END;
$$ LANGUAGE plpgsql;
```

## Usage Examples

### Basic Usage

```sql
-- View current configuration parameters
SELECT * FROM system_config ORDER BY category, parameter_name;

-- Get specific parameter value
SELECT get_config_parameter('vector_embedding_dimension');

-- Update a configuration parameter
SELECT set_config_parameter('maintenance_work_mem', '512MB');

-- Validate all configuration parameters
SELECT * FROM validate_system_config();

-- Apply performance configuration settings
SELECT * FROM apply_performance_config();

-- Optimize vector indexes based on current data
SELECT * FROM auto_optimize_vector_indexes();
```

### Confidence Management

```sql
-- Calculate confidence for a specific node
SELECT calculate_node_confidence(123);

-- Update confidence scores for specific nodes
SELECT * FROM batch_update_node_confidence_scores(
    ARRAY[1, 2, 3, 4, 5],  -- specific node IDs
    100,                    -- batch size
    0                       -- update regardless of age
);

-- Update stale confidence scores (older than 24 hours)
SELECT * FROM batch_update_node_confidence_scores(
    NULL,  -- all nodes
    500,   -- batch size
    24     -- max age in hours
);

-- Run background maintenance job
SELECT * FROM background_confidence_update_job(1000, 30);
```

### Relationship Inference

```sql
-- Queue a node for relationship inference
SELECT queue_relationship_inference(123, 7, 'manual_batch');

-- Check inference queue status
SELECT * FROM relationship_inference_queue
WHERE status = 'pending'
ORDER BY priority DESC, queued_at ASC;
```

This document provides the complete set of stored procedures, functions, and triggers that implement the core business logic for the RustyGPT database schema. These procedures handle error management, confidence calculations, relationship inference, system configuration, and performance optimization.
