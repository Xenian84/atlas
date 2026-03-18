<div align="center">

# Atlas

**Production-grade blockchain infrastructure for the X1 network.**  
Helius-parity+ indexer · REST & JSON-RPC API · Webhooks · Wallet Intelligence · Block Explorer

[![Rust](https://img.shields.io/badge/Rust-1.84+-orange?logo=rust)](https://www.rust-lang.org/)
[![X1 Network](https://img.shields.io/badge/Network-X1%20Mainnet-blue)](https://x1.xyz)
[![Tachyon](https://img.shields.io/badge/Validator-Tachyon%20v2.2.19-purple)](https://github.com/x1-labs/tachyon)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)

</div>

---

## Overview

Atlas is a self-contained indexing and API platform for the X1 blockchain, built to match and exceed [Helius](https://helius.dev) on Solana. It connects to a Tachyon validator via Yellowstone gRPC, indexes every transaction and account state change into PostgreSQL, and exposes a unified REST + JSON-RPC API with sub-100ms response times.

```
┌─────────────────────────────────┐    ┌───────────────────────────────────────────┐
│         Server A  (Validator)   │    │              Server B  (Atlas)            │
│                                 │    │                                           │
│  Tachyon v2.2.19                │    │  atlas-indexer   ◄── Yellowstone gRPC    │
│    └── Yellowstone gRPC   ──────┼───►│    └── PostgreSQL + Redis                │
│    └── atlas-geyser-v2          │    │                                           │
│         (account state)   ──────┼───►│  atlas-api       (Axum · port 8888)      │
│                                 │    │  atlas-webhooks  (delivery worker)        │
└─────────────────────────────────┘    │  atlas-intel     (wallet intelligence)   │
                                       │  explorer        (Next.js block explorer) │
                                       └───────────────────────────────────────────┘
```

---

## Features

| Category | Capabilities |
|---|---|
| **Transaction indexing** | Real-time streaming via Yellowstone gRPC · backfill CLI · shred-level ingestion |
| **Account indexing** | Non-blocking geyser plugin (atlas-geyser-v2) · SPL Token + Token-2022 ownership maps |
| **REST API** | Address history · tx facts · wallet balances · token accounts · program activity |
| **JSON-RPC** | Helius-compatible `getTransactionsForAddress`, `getTokenAccountsByOwner`, `getTokenSupply`, `getTokenLargestAccounts`, `getProgramAccountsV2`, DAS methods and more |
| **USD pricing** | Live token prices via [XDex](https://xdex.xyz) (`api.xdex.xyz/api/token-price/price`) |
| **Webhooks** | Address · token · program activity triggers with HMAC signing and automatic retry |
| **Wallet Intelligence** | Bot / sniper / whale / developer classification · risk scores · behavioral profiles |
| **Block Explorer** | Next.js 14 UI · transaction detail · address history · TOON copy for LLM workflows |
| **TOON output** | 40% token-efficient structured format for AI agent consumption |
| **LLM explain** | Configurable provider (Ollama / OpenAI / Anthropic) for human-readable tx explanations |

---

## Quick Start

### Prerequisites

- Rust 1.84+ (`rustup`)
- PostgreSQL 15+
- Redis 7+
- `protoc` (Protocol Buffers compiler)
- Access to a Yellowstone gRPC endpoint (`grpc.tachyon1.network` or self-hosted)

### 1. Configure

```bash
git clone https://github.com/TachyonZK/atlas.git && cd atlas
cp .env.example .env
```

Edit `.env` — minimum required:

```env
YELLOWSTONE_GRPC_ENDPOINT=http://YOUR_VALIDATOR_IP:10000
DATABASE_URL=postgres://atlas:YOUR_PASSWORD@localhost:5432/atlas
ADMIN_API_KEY=your-strong-secret-key
```

### 2. Run Migrations

```bash
psql "$DATABASE_URL" -f infra/migrations/001_tx_store.sql
# ... repeat for 002 through 016, or use a migration tool
```

### 3. Start (Docker)

```bash
docker compose -f infra/docker-compose.serverB.yml up -d
```

### 4. Start from Source

```bash
# Build all Rust services
cargo build --release

# Run each service (each reads from .env)
./target/release/atlas-indexer stream   # Live transaction indexing
./target/release/atlas-api              # API server on :8888
./target/release/atlas-webhooks         # Webhook delivery worker
./target/release/atlas-intel            # Wallet intelligence worker
```

### 5. Verify

```bash
curl http://localhost:8888/health
# → {"status":"ok","version":"0.1.0"}
```

---

## API Reference

All endpoints require `X-API-Key: YOUR_KEY` header unless otherwise noted.

### Transaction History

```bash
# Address history — keyset paginated, no OFFSET
GET /v1/address/{ADDRESS}/txs?limit=50&sort_order=DESC

# With filters
GET /v1/address/{ADDRESS}/txs?status=confirmed&block_time_from=1700000000&block_time_to=1710000000

# Full transaction facts
GET /v1/tx/{SIGNATURE}

# Human-readable explanation
POST /v1/tx/{SIGNATURE}/explain
```

### Wallet

```bash
# Token balances with live USD prices (XDex)
GET /v1/wallet/{ADDRESS}/balances

# Wallet identity (entity label, classification)
GET /v1/wallet/{ADDRESS}/identity

# Funded-by chain
GET /v1/wallet/{ADDRESS}/funded-by
```

### Intelligence

```bash
# Behavioral profile + risk score
GET /v1/address/{ADDRESS}/profile?window=7d

# Related wallets (co-occurrence graph)
GET /v1/address/{ADDRESS}/related
```

### JSON-RPC (Helius-compatible)

```bash
POST /rpc
Content-Type: application/json

# Transaction history
{"jsonrpc":"2.0","id":1,"method":"getTransactionsForAddress",
 "params":["ADDRESS",{"limit":100,"sortOrder":"DESC"}]}

# Token accounts by owner
{"jsonrpc":"2.0","id":1,"method":"getTokenAccountsByOwner",
 "params":["ADDRESS",{"programId":"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"}]}

# Token supply
{"jsonrpc":"2.0","id":1,"method":"getTokenSupply",
 "params":["MINT_ADDRESS"]}

# DAS — assets by owner
{"jsonrpc":"2.0","id":1,"method":"getAssetsByOwner",
 "params":{"ownerAddress":"ADDRESS","page":1,"limit":100}}
```

### Webhooks

```bash
# Subscribe
POST /v1/webhooks/subscribe
{"event_type":"address_activity","address":"ADDRESS",
 "url":"https://your-endpoint.com/hook","secret":"your-hmac-secret"}

# List subscriptions
GET /v1/webhooks/subscriptions

# Delete subscription
DELETE /v1/webhooks/subscriptions/{ID}
```

### Network

```bash
# Live network pulse (compact TOON snapshot)
GET /v1/network/pulse

# Send transaction (priority fee estimation included)
POST /v1/tx/send
```

---

## Project Structure

```
atlas/
├── crates/
│   ├── atlas_types/          # TxFactsV1, RawTx, cursor, intelligence + webhook models
│   ├── atlas_common/         # AppConfig, logging, auth middleware, metrics
│   ├── atlas_toon/           # TOON renderer (token-efficient structured output)
│   ├── atlas_parser/         # Protocol modules: system · token · swap · stake · NFT · deploy
│   ├── atlas_indexer/        # Yellowstone gRPC consumer + PostgreSQL writer  →  atlas-indexer
│   ├── atlas_api/            # Axum REST + JSON-RPC gateway                   →  atlas-api
│   ├── atlas_webhooks/       # Redis stream listener + HTTP delivery worker   →  atlas-webhooks
│   ├── atlas_intel/          # Feature extractor + scorer + profile upsert    →  atlas-intel
│   ├── atlas_shredstream/    # Shred relay helper
│   ├── atlas_alerter/        # Alerting worker
│   └── atlas_cli/            # Developer CLI (tx, wallet, token, stream, keys)
├── plugins/
│   └── atlas-geyser-v2/      # Geyser plugin (Validator side — built separately)
│       ├── src/              # Non-blocking account writer → geyser_accounts + token_owner_map
│       └── config.example.json
├── apps/
│   └── explorer/             # Next.js 14 block explorer
├── infra/
│   ├── migrations/           # 001–016 SQL migration files
│   ├── docker-compose.serverB.yml
│   ├── nginx/                # Reverse proxy config
│   ├── systemd/              # Unit files for all services
│   └── monitoring/           # Prometheus + Grafana dashboards
├── config/
│   ├── programs.yml          # X1 + Solana program IDs (XDex: sEsYH97w...)
│   ├── tags.yml              # Transaction tag classification rules
│   └── spam.yml              # Token and program denylist
├── docs/                     # Extended documentation (architecture, API, ops)
├── .env.example
└── README.md
```

---

## Geyser Plugin (Validator Side)

`plugins/atlas-geyser-v2` is a lightweight, non-blocking Geyser plugin that runs **on the validator** and streams account state changes directly into Atlas's PostgreSQL database.

It is built separately from the main workspace because it pins the Tachyon v2.2.19 ABI:

```bash
cd plugins/atlas-geyser-v2
rustup override set 1.84.1
cargo build --release
# Output: target/release/libatlas_geyser.so
```

Copy the `.so` to the validator server, configure with `config.example.json`, then add to `validator.sh`:

```bash
--geyser-plugin-config /etc/tachyon/atlas-geyser-config.json \
```

> The validator runs Yellowstone gRPC alongside this plugin — Yellowstone handles transactions, atlas-geyser-v2 handles account state. Neither blocks the other.

---

## Backfill

```bash
# Index a historical slot range
atlas-indexer backfill --from-slot 280000000 --to-slot 281000000

# Index from last checkpoint to current tip
atlas-indexer backfill --from-checkpoint
```

---

## Environment Variables

See `.env.example` for the full reference. Key variables:

| Variable | Description |
|---|---|
| `YELLOWSTONE_GRPC_ENDPOINT` | Yellowstone gRPC endpoint (e.g. `http://grpc.tachyon1.network:10000`) |
| `DATABASE_URL` | PostgreSQL connection string |
| `REDIS_URL` | Redis connection string |
| `ADMIN_API_KEY` | Master API key for admin operations |
| `ATLAS_PRICE_API_URL` | XDex price oracle (default: `https://api.xdex.xyz/api/token-price/price`) |
| `INDEXER_COMMITMENT` | Indexing commitment level: `processed` · `confirmed` · `finalized` |
| `LLM_PROVIDER` | Explain provider: `none` · `openai` · `anthropic` · `ollama` |

---

## Built On

| Component | Technology |
|---|---|
| Validator | [Tachyon v2.2.19](https://github.com/x1-labs/tachyon) — X1 network validator |
| Transaction streaming | [Yellowstone gRPC](https://github.com/rpcpool/yellowstone-grpc) |
| Account streaming | [x1-geyser-postgres](https://github.com/x1-labs/x1-geyser-postgres) (forked → atlas-geyser-v2) |
| Price oracle | [XDex](https://xdex.xyz) — native X1 DEX |
| API runtime | [Axum](https://github.com/tokio-rs/axum) + Tokio |
| Database | PostgreSQL 15 + Redis 7 |
| Explorer | Next.js 14 |

---

<div align="center">

Built for the X1 network · MIT License

</div>
