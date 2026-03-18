-- Migration 008: intelligence_wallet_profiles
-- Note: "window" is a reserved keyword in PostgreSQL — quote it everywhere.

CREATE TABLE IF NOT EXISTS intelligence_wallet_profiles (
    address               TEXT NOT NULL,
    "window"              TEXT NOT NULL,   -- 24h|7d|30d|all
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    wallet_type           TEXT NOT NULL DEFAULT 'unknown',
    confidence            NUMERIC NOT NULL DEFAULT 0,
    automation_score      INT NOT NULL DEFAULT 0,
    sniper_score          INT NOT NULL DEFAULT 0,
    whale_score           INT NOT NULL DEFAULT 0,
    risk_score            INT NOT NULL DEFAULT 0,
    features_json         JSONB NOT NULL DEFAULT '{}'::jsonb,
    top_programs_json     JSONB NOT NULL DEFAULT '[]'::jsonb,
    top_tokens_json       JSONB NOT NULL DEFAULT '[]'::jsonb,
    top_counterparties_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    PRIMARY KEY (address, "window")
);

CREATE INDEX IF NOT EXISTS intel_profiles_window_updated_idx
    ON intelligence_wallet_profiles ("window", updated_at DESC);

CREATE INDEX IF NOT EXISTS intel_profiles_wallet_type_idx
    ON intelligence_wallet_profiles (wallet_type, "window");
