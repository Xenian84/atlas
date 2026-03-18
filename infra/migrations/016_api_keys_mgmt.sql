-- Migration 016: api_keys full management
-- Extend api_keys with name, tier, usage tracking.
-- The table may already exist from earlier migrations — use IF NOT EXISTS + ALTER.

CREATE TABLE IF NOT EXISTS api_keys (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    key_hash      TEXT        NOT NULL UNIQUE,
    key_prefix    TEXT        NOT NULL,   -- first 8 chars for display
    name          TEXT        NOT NULL DEFAULT 'default',
    tier          TEXT        NOT NULL DEFAULT 'free',
    rate_limit    INT         NOT NULL DEFAULT 300,
    is_active     BOOL        NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at  TIMESTAMPTZ,
    owner_email   TEXT
);

CREATE INDEX IF NOT EXISTS api_keys_hash_idx  ON api_keys (key_hash);
CREATE INDEX IF NOT EXISTS api_keys_active_idx ON api_keys (is_active) WHERE is_active = true;
