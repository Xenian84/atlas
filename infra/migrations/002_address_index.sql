-- Migration 002: address_index
-- Inverted index: address -> (slot, pos, sig). The core of history queries.
-- Never query tx_store.accounts directly; always go through this table.

CREATE TABLE IF NOT EXISTS address_index (
    address      TEXT NOT NULL,
    slot         BIGINT NOT NULL,
    pos          INT NOT NULL,
    sig          TEXT NOT NULL,
    block_time   BIGINT NULL,
    status       SMALLINT NOT NULL,
    tags         TEXT[] NOT NULL DEFAULT '{}',
    action_types TEXT[] NOT NULL DEFAULT '{}',
    PRIMARY KEY (address, slot, pos)
);

-- Primary lookup: keyset pagination (address + cursor)
CREATE INDEX IF NOT EXISTS address_index_addr_slot_pos_desc
    ON address_index (address, slot DESC, pos DESC);

-- Reverse lookup: sig -> rows (for cascading deletes / re-index)
CREATE INDEX IF NOT EXISTS address_index_sig_idx
    ON address_index (sig);
