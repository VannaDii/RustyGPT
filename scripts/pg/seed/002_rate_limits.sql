-- Default rate limit profiles
INSERT INTO rustygpt.rate_limit_profiles (name, algorithm, params, description)
VALUES
    (
        'conversation.post',
        'gcra',
        jsonb_build_object('requests_per_second', 5, 'burst', 10),
        'Default conversation posting limits'
    )
ON CONFLICT (name) DO UPDATE
SET algorithm = EXCLUDED.algorithm,
    params = EXCLUDED.params,
    description = EXCLUDED.description,
    updated_at = now();
