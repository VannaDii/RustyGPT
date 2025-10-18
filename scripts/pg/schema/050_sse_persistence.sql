-- SSE persistence retention enhancements.
SET search_path TO rustygpt, public;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_indexes
        WHERE schemaname = 'rustygpt'
          AND tablename = 'sse_event_log'
          AND indexname = 'idx_sse_event_log_created_at'
    ) THEN
        EXECUTE 'CREATE INDEX idx_sse_event_log_created_at ON rustygpt.sse_event_log (created_at)';
    END IF;
END;
$$;

COMMENT ON INDEX rustygpt.idx_sse_event_log_created_at IS
    'Supports time-based pruning scans over SSE event history.';
