---
name: atlas-indexer
description: Work on the Atlas indexer — gRPC stream ingestion, transaction parsing, DB writes, backfill, or checkpoint logic. Use when touching crates/atlas_indexer or crates/atlas_parser.
---

# Atlas Indexer Skill

## What it does
Connects to the X1 Tachyon validator via Yellowstone gRPC, receives a stream of confirmed transactions, parses them into `TxFacts`, writes to PostgreSQL, and publishes events to Redis streams.

## Pipeline
```
Yellowstone gRPC
  └─ stream.rs         Subscribe + receive UpdateOneof events
       └─ grpc_conv.rs Convert gRPC proto → RawTx (merge ALT addresses)
            └─ parser.rs Run parser modules on RawTx → TxFacts
                 └─ db.rs  Upsert tx_store + address_index (bulk UNNEST)
                      └─ stream.rs  XADD to Redis atlas:newtx stream
```

## Key files
```
crates/atlas_indexer/src/
├── main.rs          Startup: load config, connect DB/Redis/gRPC, spawn tasks
├── stream.rs        Yellowstone gRPC subscription loop + BlockMeta handler
├── grpc_conv.rs     Proto → RawTx conversion; merges loaded ALT addresses
├── backfill.rs      Historical backfill via getBlock RPC
├── db.rs            PostgreSQL upserts (bulk UNNEST for address_index)
├── checkpoint.rs    Slot checkpoint: saves last processed slot to Redis
└── config.rs        IndexerConfig (env vars)

crates/atlas_parser/src/
├── parser.rs        Runs all modules; produces TxFacts from RawTx
├── tags.rs          apply_tags(): classifies tx (swap, transfer, spam, etc.)
├── deltas.rs        compute_sol_deltas(): net SOL change per account
└── modules/
    ├── token_transfer.rs   SPL token transfer detection + amount extraction
    ├── swap.rs             DEX swap detection
    ├── compute_budget.rs   ComputeUnitLimit + priority fee extraction
    └── ...
```

## Key types (`crates/atlas_types/src/`)
- `RawTx` — raw decoded transaction before parsing
- `TxFacts` — fully parsed tx with actions, tags, deltas, programs
- `AccountKey` — address + role (writable/signer/etc.)

## Environment variables
```
YELLOWSTONE_GRPC_ENDPOINT   http://127.0.0.1:10000
YELLOWSTONE_GRPC_TOKEN      (optional auth token)
DATABASE_URL                postgres://atlas:atlas@localhost:5432/atlas
REDIS_URL                   redis://127.0.0.1:6379
ATLAS_COMMITMENT            confirmed   (or finalized)
ATLAS_PROGRAMS_CONFIG       config/programs.yml
ATLAS_SPAM_CONFIG           config/spam.toml
```

## Redis stream format
Stream key: `atlas:newtx`
Field: `data` → JSON-serialized `TxFacts`

Consumer groups:
- `atlas-webhooks` (atlas_webhooks listener.rs)
- `atlas-intel`    (atlas_intel trigger.rs)

## Common debugging
```bash
# Check indexer is running and processing
journalctl -u atlas-indexer -f

# Check Redis stream depth
redis-cli XLEN atlas:newtx

# Check last checkpoint
redis-cli GET atlas:checkpoint:confirmed

# Backfill a specific slot range
./target/release/atlas-indexer --backfill-from 1000000 --backfill-to 1001000
```

## Important invariants
- `grpc_conv.rs` MUST merge `meta.loaded_writable_addresses` + `meta.loaded_readonly_addresses` into `account_keys` — without this, ALT-using txs (Jupiter, Raydium) have empty from/to/program_id.
- `checkpoint.rs` uses Redis `SET` with `XX` (only if exists) to avoid TOCTOU races on startup.
- `db.rs` uses bulk `UNNEST` insert for `address_index` — never revert to N+1 loop.
