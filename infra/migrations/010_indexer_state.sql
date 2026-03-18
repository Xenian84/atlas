-- Migration 010: indexer_state + api_keys

CREATE TABLE IF NOT EXISTS indexer_state (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Seed initial checkpoints
INSERT INTO indexer_state (key, value) VALUES
    ('last_ingested_slot_confirmed',  '0'),
    ('last_ingested_slot_finalized',  '0'),
    ('backfill_progress',             '{}')
ON CONFLICT DO NOTHING;

-- API keys table for auth
CREATE TABLE IF NOT EXISTS api_keys (
    key_hash    TEXT PRIMARY KEY,              -- SHA-256 hex of the raw key
    label       TEXT NOT NULL DEFAULT '',
    is_active   BOOLEAN NOT NULL DEFAULT true,
    is_admin    BOOLEAN NOT NULL DEFAULT false,
    rate_limit  INT NOT NULL DEFAULT 300,      -- requests per minute
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS api_keys_active_idx ON api_keys (is_active);
