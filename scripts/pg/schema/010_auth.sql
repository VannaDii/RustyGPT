-- Core authentication schema (users, roles, sessions)
SET search_path TO rustygpt, public;

-- Required extensions for auth domain
CREATE EXTENSION IF NOT EXISTS citext;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Enumerations ----------------------------------------------------------------

DO $role_enum$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_type typ
        JOIN pg_namespace nsp ON nsp.oid = typ.typnamespace
        WHERE typ.typname = 'user_role'
          AND nsp.nspname = 'rustygpt'
    ) THEN
        CREATE TYPE rustygpt.user_role AS ENUM ('admin', 'member', 'read_only');
    END IF;
END;
$role_enum$;

-- Users -----------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS rustygpt.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email CITEXT NOT NULL UNIQUE,
    username CITEXT NOT NULL UNIQUE,
    display_name TEXT,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_users_created_at
    ON rustygpt.users (created_at DESC);

-- Global role grants ----------------------------------------------------------

CREATE TABLE IF NOT EXISTS rustygpt.user_roles (
    user_id UUID NOT NULL REFERENCES rustygpt.users(id) ON DELETE CASCADE,
    role rustygpt.user_role NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    granted_by UUID,
    PRIMARY KEY (user_id, role)
);

-- Session storage -------------------------------------------------------------

CREATE TABLE IF NOT EXISTS rustygpt.user_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES rustygpt.users(id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    rotated_at TIMESTAMPTZ,
    rotated_by UUID,
    revoked_at TIMESTAMPTZ,
    revoked_by UUID,
    user_agent TEXT,
    ip inet
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_user_sessions_token_hash
    ON rustygpt.user_sessions (token_hash);

CREATE INDEX IF NOT EXISTS idx_user_sessions_user
    ON rustygpt.user_sessions (user_id);

CREATE INDEX IF NOT EXISTS idx_user_sessions_activity
    ON rustygpt.user_sessions (last_seen_at DESC);
