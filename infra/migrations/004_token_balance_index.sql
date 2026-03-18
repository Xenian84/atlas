-- Migration 004: token_balance_index
-- Per-owner token balance change events. Used for type=balanceChanged queries.

CREATE TABLE IF NOT EXISTS token_balance_index (
    owner      TEXT     NOT NULL,
    slot       BIGINT   NOT NULL,
    pos        INT      NOT NULL,
    sig        TEXT     NOT NULL REFERENCES tx_store(sig) ON DELETE CASCADE,
    mint       TEXT     NOT NULL,
    delta      NUMERIC  NOT NULL,
    direction  SMALLINT NOT NULL CHECK (direction IN (1, 2)),  -- 1=in 2=out
    PRIMARY KEY (owner, slot, pos, mint)
);

CREATE INDEX IF NOT EXISTS token_balance_owner_slot_pos_desc
    ON token_balance_index (owner, slot DESC, pos DESC);

CREATE INDEX IF NOT EXISTS token_balance_mint_idx
    ON token_balance_index (mint);
