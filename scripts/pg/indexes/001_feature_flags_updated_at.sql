-- Index to accelerate feature flag lookups by update time
CREATE INDEX IF NOT EXISTS idx_feature_flags_updated_at
    ON rustygpt.feature_flags (updated_at);
