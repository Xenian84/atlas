-- Migration 013: entity_labels
-- Known wallet/program identity database.
-- Used by Wallet API identity endpoints.

CREATE TABLE IF NOT EXISTS entity_labels (
    address     TEXT        NOT NULL PRIMARY KEY,
    name        TEXT        NOT NULL,
    category    TEXT        NOT NULL DEFAULT 'unknown',
        -- system | token_program | dex | bridge | validator | exchange |
        -- defi | nft | oracle | staking | tool | spam | hacker | other
    entity_type TEXT        NOT NULL DEFAULT 'program',
        -- program | exchange | protocol | wallet | validator | other
    verified    BOOLEAN     NOT NULL DEFAULT true,
    url         TEXT,
    notes       TEXT,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS entity_labels_category_idx
    ON entity_labels (category);

CREATE INDEX IF NOT EXISTS entity_labels_name_idx
    ON entity_labels (lower(name));

-- ── Seed: Core System Programs ────────────────────────────────────────────────
INSERT INTO entity_labels (address, name, category, entity_type) VALUES
-- System
('11111111111111111111111111111111',             'System Program',              'system',        'program'),
('Vote111111111111111111111111111111111111111',  'Vote Program',                'system',        'program'),
('Stake11111111111111111111111111111111111111',  'Stake Program',               'staking',       'program'),
('ComputeBudget111111111111111111111111111111',  'Compute Budget Program',      'system',        'program'),
('BPFLoaderUpgradeab1e11111111111111111111111',  'BPF Upgradeable Loader',     'system',        'program'),
('NativeLoader1111111111111111111111111111111',  'Native Loader',              'system',        'program'),
('Config1111111111111111111111111111111111111',  'Config Program',              'system',        'program'),
('AddressLookupTab1e1111111111111111111111111',  'Address Lookup Table Program','system',        'program'),

-- Token Programs
('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA', 'SPL Token Program',         'token_program', 'program'),
('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb', 'Token-2022 Program',        'token_program', 'program'),
('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe8bv', 'Associated Token Account',  'token_program', 'program'),
('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s', 'Metaplex Token Metadata',   'nft',           'program'),
('BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY','Metaplex Bubblegum',        'nft',           'program'),

-- Well-known token mints (X1 native)
('So11111111111111111111111111111111111111112',  'Wrapped XNT',                'token_program', 'program'),
('B69chRzqzDCmdB5WYB8NRu5Yv5ZA95ABiZcdzCgGm9Tq','USDC.x',                  'token_program', 'program')

ON CONFLICT (address) DO UPDATE SET
    name       = EXCLUDED.name,
    category   = EXCLUDED.category,
    updated_at = now();
