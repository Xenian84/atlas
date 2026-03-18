-- Migration 006: webhook_subscriptions

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS webhook_subscriptions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    is_active   BOOLEAN NOT NULL DEFAULT true,
    event_type  TEXT NOT NULL,   -- address_activity|token_balance_changed|program_activity
    address     TEXT NULL,
    owner       TEXT NULL,
    program_id  TEXT NULL,
    url         TEXT NOT NULL,
    secret      TEXT NOT NULL,
    min_conf    TEXT NOT NULL DEFAULT 'confirmed',
    format      TEXT NOT NULL DEFAULT 'json',    -- json|toon
    filter_json JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX IF NOT EXISTS webhook_sub_event_type_idx  ON webhook_subscriptions (event_type);
CREATE INDEX IF NOT EXISTS webhook_sub_address_idx     ON webhook_subscriptions (address) WHERE address IS NOT NULL;
CREATE INDEX IF NOT EXISTS webhook_sub_owner_idx       ON webhook_subscriptions (owner)   WHERE owner IS NOT NULL;
CREATE INDEX IF NOT EXISTS webhook_sub_program_idx     ON webhook_subscriptions (program_id) WHERE program_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS webhook_sub_active_idx      ON webhook_subscriptions (is_active);
