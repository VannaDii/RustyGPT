-- Default rate limit profiles
INSERT INTO rustygpt.rate_limit_profiles (name, requests_per_second, burst)
VALUES
    ('conversation.post', 5, 10)
ON CONFLICT (name) DO NOTHING;
