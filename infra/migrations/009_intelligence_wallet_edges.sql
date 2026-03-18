-- Migration 009: intelligence_wallet_edges (v2 — clustering/relationships)

CREATE TABLE IF NOT EXISTS intelligence_wallet_edges (
    src        TEXT NOT NULL,
    dst        TEXT NOT NULL,
    reason     TEXT NOT NULL,
    weight     NUMERIC NOT NULL DEFAULT 1.0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (src, dst, reason)
);

CREATE INDEX IF NOT EXISTS wallet_edges_src_idx ON intelligence_wallet_edges (src);
CREATE INDEX IF NOT EXISTS wallet_edges_dst_idx ON intelligence_wallet_edges (dst);
