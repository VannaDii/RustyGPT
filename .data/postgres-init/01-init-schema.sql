-- Create Users Table
CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT,
  apple_id TEXT UNIQUE,
  github_id TEXT UNIQUE,
  created_at TIMESTAMP DEFAULT NOW()
);