-- Create Users Table
CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  username TEXT UNIQUE NOT NULL,
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT,
  apple_id TEXT UNIQUE,
  github_id TEXT UNIQUE,
  created_at TIMESTAMP DEFAULT NOW()
);

-- Create indexes for optimized access
CREATE INDEX IF NOT EXISTS idx_users_username_password ON users(username, password_hash);
CREATE INDEX IF NOT EXISTS idx_users_email_password ON users(email, password_hash);