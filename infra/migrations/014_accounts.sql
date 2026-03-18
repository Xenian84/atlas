-- Migration 014: accounts
-- Real-time account state. Written by atlas-geyser plugin inside the validator,
-- bridged from geyser_accounts via trigger trg_geyser_to_accounts.

CREATE TABLE IF NOT EXISTS geyser_accounts (
    pubkey      TEXT        NOT NULL PRIMARY KEY,
    lamports    BIGINT      NOT NULL,
    owner       TEXT        NOT NULL DEFAULT '',
    executable  BOOLEAN     NOT NULL DEFAULT FALSE,
    data        TEXT        NOT NULL DEFAULT '',   -- hex-encoded raw account data
    slot        BIGINT      NOT NULL,
    is_startup  BOOLEAN     NOT NULL DEFAULT FALSE,
    written_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS geyser_accounts_owner_idx ON geyser_accounts (owner);
CREATE INDEX IF NOT EXISTS geyser_accounts_slot_idx  ON geyser_accounts (slot);

CREATE TABLE IF NOT EXISTS accounts (
    address       TEXT        PRIMARY KEY,
    lamports      BIGINT      NOT NULL DEFAULT 0,
    owner         TEXT        NOT NULL DEFAULT '11111111111111111111111111111111',
    executable    BOOL        NOT NULL DEFAULT false,
    space         BIGINT      NOT NULL DEFAULT 0,
    updated_slot  BIGINT      NOT NULL DEFAULT 0,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Descending balance lookup (rich list)
CREATE INDEX IF NOT EXISTS accounts_lamports_idx
    ON accounts (lamports DESC)
    WHERE lamports > 0;

-- Program-owned accounts (token program, stake program, etc.)
CREATE INDEX IF NOT EXISTS accounts_owner_idx
    ON accounts (owner)
    WHERE owner != '11111111111111111111111111111111';

-- ── Trigger: geyser_accounts → accounts (real-time sync) ─────────────────────

CREATE OR REPLACE FUNCTION sync_geyser_to_accounts()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO accounts (address, lamports, owner, executable, space, updated_slot, updated_at)
    VALUES (
        NEW.pubkey,
        NEW.lamports,
        CASE WHEN NEW.owner = '' THEN '11111111111111111111111111111111' ELSE NEW.owner END,
        NEW.executable,
        length(NEW.data) / 2,   -- hex-encoded, 2 chars per byte
        NEW.slot,
        NEW.written_at
    )
    ON CONFLICT (address) DO UPDATE SET
        lamports     = EXCLUDED.lamports,
        owner        = EXCLUDED.owner,
        executable   = EXCLUDED.executable,
        space        = EXCLUDED.space,
        updated_slot = EXCLUDED.updated_slot,
        updated_at   = EXCLUDED.updated_at
    WHERE accounts.updated_slot < EXCLUDED.updated_slot;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_geyser_to_accounts
    AFTER INSERT OR UPDATE ON geyser_accounts
    FOR EACH ROW EXECUTE FUNCTION sync_geyser_to_accounts();
