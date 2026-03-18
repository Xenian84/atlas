-- Migration 001: tx_store
-- One row per transaction signature. Stores the canonical TxFactsV1 fields.
-- commitment: shred < processed < confirmed < finalized (upgraded in-place via ON CONFLICT)

CREATE TABLE IF NOT EXISTS tx_store (
    sig                         TEXT        PRIMARY KEY,
    slot                        BIGINT      NOT NULL,
    pos                         INT         NOT NULL,
    block_time                  BIGINT      NULL,           -- NULL for shred (unknown until confirmed)
    status                      SMALLINT    NOT NULL        -- 0=pending(shred) 1=success 2=failed
                                            DEFAULT 0
                                            CHECK (status IN (0, 1, 2)),
    fee_lamports                BIGINT      NOT NULL DEFAULT 0,
    compute_consumed            INT         NULL,
    compute_limit               INT         NULL,
    priority_fee_micro_lamports BIGINT      NULL,
    programs                    TEXT[]      NOT NULL DEFAULT '{}',
    tags                        TEXT[]      NOT NULL DEFAULT '{}',
    accounts_json               JSONB       NOT NULL DEFAULT '[]'::jsonb,
    actions_json                JSONB       NOT NULL DEFAULT '[]'::jsonb,
    token_deltas_json           JSONB       NOT NULL DEFAULT '[]'::jsonb,
    sol_deltas_json             JSONB       NOT NULL DEFAULT '[]'::jsonb,
    logs_digest                 JSONB       NULL,
    err_json                    JSONB       NULL,
    raw_ref                     TEXT        NULL,
    commitment                  TEXT        NOT NULL DEFAULT 'shred'
                                            CHECK (commitment IN ('shred','processed','confirmed','finalized')),
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Keyset pagination by slot
CREATE INDEX IF NOT EXISTS tx_store_slot_pos_idx
    ON tx_store (slot DESC, pos DESC);

-- Commitment-scoped slot scan (used by block endpoint + gap detector)
CREATE INDEX IF NOT EXISTS tx_store_commitment_slot_idx
    ON tx_store (commitment, slot DESC);

-- block_time range queries (used by token 24h stats)
CREATE INDEX IF NOT EXISTS tx_store_block_time_idx
    ON tx_store (block_time DESC)
    WHERE commitment = 'confirmed' AND block_time IS NOT NULL;

-- Program filter (webhook program_activity type)
CREATE INDEX IF NOT EXISTS tx_store_programs_gin
    ON tx_store USING GIN (programs);

-- Tag filter (fee_only, transfer, swap, etc.)
CREATE INDEX IF NOT EXISTS tx_store_tags_gin
    ON tx_store USING GIN (tags);
