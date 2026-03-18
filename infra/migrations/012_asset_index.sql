-- Migration 012: asset_index
-- Stores indexed digital assets: NFTs, cNFTs, fungible tokens, inscriptions.
-- Populated by the DAS indexer module watching mint/transfer events.

CREATE TABLE IF NOT EXISTS asset_index (
    mint                TEXT        NOT NULL PRIMARY KEY,
    asset_type          TEXT        NOT NULL DEFAULT 'unknown',
        -- fungible | nft | compressed_nft | inscription
    owner               TEXT,
    update_authority    TEXT,
    creator             TEXT,
    creator_verified    BOOLEAN     NOT NULL DEFAULT false,
    collection_mint     TEXT,
    collection_verified BOOLEAN     NOT NULL DEFAULT false,
    name                TEXT        NOT NULL DEFAULT '',
    symbol              TEXT        NOT NULL DEFAULT '',
    uri                 TEXT,                               -- off-chain metadata URI
    image_uri           TEXT,                               -- resolved image URL
    decimals            SMALLINT    NOT NULL DEFAULT 0,
    supply              NUMERIC     NOT NULL DEFAULT 0,
    is_burned           BOOLEAN     NOT NULL DEFAULT false,
    is_compressed       BOOLEAN     NOT NULL DEFAULT false,
    tree_address        TEXT,                               -- Bubblegum merkle tree
    leaf_index          BIGINT,
    slot_created        BIGINT,
    slot_updated        BIGINT      NOT NULL DEFAULT 0,
    royalty_basis_pts   INT         NOT NULL DEFAULT 0,
    attributes_json     JSONB       NOT NULL DEFAULT '[]'::jsonb,
    metadata_json       JSONB       NOT NULL DEFAULT '{}'::jsonb,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Owner lookup (getAssetsByOwner)
CREATE INDEX IF NOT EXISTS asset_owner_idx
    ON asset_index (owner, slot_updated DESC)
    WHERE owner IS NOT NULL;

-- Collection lookup (getAssetsByGroup)
CREATE INDEX IF NOT EXISTS asset_collection_idx
    ON asset_index (collection_mint, slot_updated DESC)
    WHERE collection_mint IS NOT NULL;

-- Creator lookup (getAssetsByCreator)
CREATE INDEX IF NOT EXISTS asset_creator_idx
    ON asset_index (creator, creator_verified, slot_updated DESC)
    WHERE creator IS NOT NULL;

-- Update authority lookup (getAssetsByAuthority)
CREATE INDEX IF NOT EXISTS asset_update_authority_idx
    ON asset_index (update_authority)
    WHERE update_authority IS NOT NULL;

-- Type filter for searchAssets
CREATE INDEX IF NOT EXISTS asset_type_idx
    ON asset_index (asset_type, slot_updated DESC);

-- Compressed NFT tree lookup
CREATE INDEX IF NOT EXISTS asset_tree_idx
    ON asset_index (tree_address, leaf_index)
    WHERE tree_address IS NOT NULL;

-- Token accounts table: maps token accounts to owner+mint
-- Used by getTokenAccounts
CREATE TABLE IF NOT EXISTS token_account_index (
    token_account   TEXT        NOT NULL PRIMARY KEY,
    owner           TEXT        NOT NULL,
    mint            TEXT        NOT NULL,
    amount          NUMERIC     NOT NULL DEFAULT 0,
    decimals        SMALLINT    NOT NULL DEFAULT 0,
    slot_updated    BIGINT      NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS token_account_owner_idx
    ON token_account_index (owner, mint);

CREATE INDEX IF NOT EXISTS token_account_mint_idx
    ON token_account_index (mint);
