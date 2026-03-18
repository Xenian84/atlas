-- Migration 003: token_owner_map
-- Maps owner -> token accounts for balanceChanged queries (Helius parity).

CREATE TABLE IF NOT EXISTS token_owner_map (
    owner         TEXT NOT NULL,
    token_account TEXT PRIMARY KEY,
    mint          TEXT NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS token_owner_map_owner_idx ON token_owner_map (owner);
CREATE INDEX IF NOT EXISTS token_owner_map_mint_idx  ON token_owner_map (mint);
