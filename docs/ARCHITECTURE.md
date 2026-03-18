# Atlas Architecture

## Overview

Atlas is a Helius-parity+ blockchain infrastructure platform for the X1 network.
It indexes all chain activity from a live X1 validator and serves:
- Indexed transaction history with keyset pagination
- Enhanced parsed transactions (actions/events/deltas)
- Webhooks for address/token/program activity
- Wallet intelligence scoring
- Orb-like explorer UI with explain + TOON copy

## Two-Server Topology

```
Server A (Validator / Chain)
  ├── X1 Validator (tachyon)
  ├── Internal RPC :8899 (private)
  └── Yellowstone gRPC Producer :10000 (private to Server B)

Server B (Atlas / Consumer)
  ├── Postgres 15 + Redis 7
  ├── atlas-indexer  (Yellowstone gRPC consumer → DB)
  ├── atlas-api      (Axum REST + JSON-RPC :8080)
  ├── atlas-webhooks (delivery worker)
  ├── atlas-intel    (wallet profile worker)
  └── explorer       (Next.js :3000)
```

## Data Pipeline

```
Yellowstone gRPC
  → atlas_indexer
    → RawTx (grpc_conv)
    → atlas_parser (module chain)
      → TxFactsV1 (normalized)
    → DB: tx_store + address_index + token_balance_index + program_activity_index
    → Redis: XADD atlas:newtx
                 ↓                         ↓
          atlas_webhooks              atlas_intel
          (delivery worker)         (profile worker)
```

## Crate Dependency Graph

```
atlas_types  ←──── atlas_common
     ↑                  ↑
atlas_toon          atlas_parser
     ↑                  ↑
atlas_api ──────── atlas_indexer
     ↑
atlas_webhooks
atlas_intel
```

## Key Design Decisions

### Inverted Index (address_index)
Never scan `tx_store.accounts_json` for history queries.
`address_index` has one row per (address, tx) with PK `(address, slot, pos)`.
Keyset pagination: `WHERE address=$1 AND (slot,pos) < (beforeSlot, beforePos)`.

### Idempotency
All writes are `INSERT ... ON CONFLICT DO UPDATE/NOTHING`.
The indexer can replay any slot range safely.

### Determinism (TxFactsV1)
`normalize()` enforces stable sort order on all collections before storage.
TOON output is therefore also deterministic.

### TOON (LLM efficiency)
JSON remains the default for all API responses.
TOON is offered via `Accept: text/toon` or `?format=toon`.
`POST /v1/tx/:sig/explain` returns both JSON facts and a `factsToon` string for LLM prompting.
