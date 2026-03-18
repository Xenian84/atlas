-- Migration 015: token_metadata
-- SPL token name, symbol, decimals, logo. Populated lazily on first encounter,
-- then retried if name is empty (Token-2022 extension or Metaplex).
-- supply is NUMERIC (not BIGINT) because SPL u64 can exceed i64 max.

CREATE TABLE IF NOT EXISTS token_metadata (
    mint        TEXT        PRIMARY KEY,
    name        TEXT        NOT NULL DEFAULT '',
    symbol      TEXT        NOT NULL DEFAULT '',
    decimals    SMALLINT    NOT NULL DEFAULT 0,
    logo_uri    TEXT,
    uri         TEXT,           -- off-chain metadata URI
    supply      NUMERIC     NOT NULL DEFAULT 0,  -- u64 fits in NUMERIC, not BIGINT
    is_nft      BOOL        NOT NULL DEFAULT false,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS token_metadata_symbol_idx
    ON token_metadata (symbol)
    WHERE symbol != '';

-- Partial index for mints still needing metadata resolution
CREATE INDEX IF NOT EXISTS token_metadata_unnamed_idx
    ON token_metadata (updated_at)
    WHERE name = '';
