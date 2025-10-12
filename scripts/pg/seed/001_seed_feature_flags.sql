-- Default feature flag values. Safe to run repeatedly.
INSERT INTO rustygpt.feature_flags (name, enabled)
VALUES
    ('auth_v1', false),
    ('well_known', false),
    ('sse_v1', false)
ON CONFLICT (name) DO NOTHING;
