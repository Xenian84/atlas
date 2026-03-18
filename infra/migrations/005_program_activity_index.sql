-- Migration 005: program_activity_index
-- Per-program transaction references. Required for program_activity webhook type (v2).

CREATE TABLE IF NOT EXISTS program_activity_index (
    program_id TEXT NOT NULL,
    slot       BIGINT NOT NULL,
    pos        INT NOT NULL,
    sig        TEXT NOT NULL,
    block_time BIGINT NULL,
    tags       TEXT[] NOT NULL DEFAULT '{}',
    PRIMARY KEY (program_id, slot, pos)
);

CREATE INDEX IF NOT EXISTS program_activity_prog_slot_pos_desc
    ON program_activity_index (program_id, slot DESC, pos DESC);

CREATE INDEX IF NOT EXISTS program_activity_sig_idx
    ON program_activity_index (sig);
