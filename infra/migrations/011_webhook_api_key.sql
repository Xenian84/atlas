-- Migration 011: add api_key_hash to webhook_subscriptions for per-key scoping
ALTER TABLE webhook_subscriptions
    ADD COLUMN IF NOT EXISTS api_key_hash TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS webhook_sub_api_key_idx ON webhook_subscriptions (api_key_hash);
