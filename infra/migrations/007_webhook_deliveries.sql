-- Migration 007: webhook_deliveries

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id                BIGSERIAL PRIMARY KEY,
    subscription_id   UUID NOT NULL REFERENCES webhook_subscriptions(id) ON DELETE CASCADE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    next_attempt_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    attempt_count     INT NOT NULL DEFAULT 0,
    status            TEXT NOT NULL DEFAULT 'pending',  -- pending|success|failed
    last_error        TEXT NULL,
    payload_json      JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS webhook_deliveries_pending_idx
    ON webhook_deliveries (next_attempt_at)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS webhook_deliveries_sub_idx
    ON webhook_deliveries (subscription_id, created_at DESC);
