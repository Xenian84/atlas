<div align="center">

# Atlas

**Production-grade blockchain infrastructure for the X1 network.**  
Full-featured indexer · REST & JSON-RPC API · Webhooks · Wallet Intelligence · Block Explorer · CLI

[![Rust](https://img.shields.io/badge/Rust-1.84+-orange?logo=rust)](https://www.rust-lang.org/)
[![X1 Network](https://img.shields.io/badge/Network-X1%20Mainnet-blue)](https://x1.xyz)
[![Tachyon](https://img.shields.io/badge/Validator-Tachyon%20v2.2.19-purple)](https://github.com/x1-labs/tachyon)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)

</div>

---

## Overview

Atlas is a self-contained indexing and API platform for the X1 blockchain. It connects to a Tachyon validator via Yellowstone gRPC, indexes every transaction and account state change into PostgreSQL, and exposes a unified REST + JSON-RPC API with sub-100ms response times.

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
| **JSON-RPC** | `getTransactionsForAddress`, `getTokenAccountsByOwner`, `getTokenSupply`, `getTokenLargestAccounts`, `getProgramAccountsV2`, full DAS API (10 methods), all standard Solana RPC via proxy |
| **USD pricing** | Live token prices via [XDex](https://xdex.xyz) — native X1 DEX price oracle |
| **Priority fees** | `getPriorityFeeEstimate` with all 6 levels: min · low · medium · high · veryHigh · unsafeMax |
| **Webhooks** | Address · token · program activity triggers with HMAC signing and automatic retry |
| **Wallet Intelligence** | Bot / sniper / whale / developer classification · risk scores · behavioral profiles |
| **Block Explorer** | Next.js 14 UI · transaction detail · address history · TOON copy for LLM workflows |
| **TOON output** | 40% token-efficient structured format for AI agent and script consumption |
| **LLM explain** | Configurable provider (Ollama / OpenAI / Anthropic) for human-readable tx explanations |
| **MCP server** | Native Model Context Protocol tool provider for Claude and AI agents |
| **CLI** | `atlas` binary — keygen, rpc, tx, wallet, token, block, stream, keys, usage · `--json` flag |

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
for f in infra/migrations/*.sql; do psql "$DATABASE_URL" -f "$f"; done
```

### 3. Start (Docker)

```bash
docker compose -f infra/docker-compose.serverB.yml up -d
```

### 4. Start from Source

```bash
# Build all Rust services + CLI
cargo build --release

# Run each service (each reads from .env)
./target/release/atlas-indexer stream   # Live transaction indexing
./target/release/atlas-api              # API server  →  :8888
./target/release/atlas-webhooks         # Webhook delivery worker
./target/release/atlas-intel            # Wallet intelligence worker
```

### 5. Verify

```bash
curl http://localhost:8888/health
# → {"status":"ok","version":"0.1.0"}

atlas status          # Full system health check
atlas pulse           # Network pulse snapshot
```

---

## CLI Reference

The `atlas` binary is a developer and operations tool for the Atlas platform. Add `--json` (or `-j`) to any command for machine-readable output — ideal for scripts, CI pipelines, and AI agents.

```bash
# Install (after cargo build --release)
cp target/release/atlas /usr/local/bin/atlas
export ATLAS_API_URL=http://localhost:8888
export ATLAS_API_KEY=your-key
```

### Onboarding

```bash
# Generate an X1 keypair  (writes ~/.atlas/keypair.json)
atlas keygen
atlas keygen --output /path/to/keypair.json

# Print all RPC endpoint URLs for this Atlas instance
atlas rpc
atlas rpc --json
```

### Data Queries

```bash
# Look up a transaction
atlas tx 5wJb...xyz

# Wallet overview — identity, balances, recent history
atlas wallet ADDRESS

# Token info + top holders
atlas token MINT_ADDRESS
atlas token MINT_ADDRESS --holders

# Block overview
atlas block 291000000

# Network health + indexer stats
atlas status
atlas pulse
```

### Live Stream

```bash
# Show last 10 live transaction events
atlas stream

# Watch continuously  (Ctrl-C to stop)
atlas stream --watch --count 50
```

### API Key Management  (admin only)

```bash
# List all keys + last-used timestamps
atlas keys list
atlas keys list --json

# Create a new key
atlas keys create "my-dapp" --tier pro --rpm 1000

# Show usage stats per key
atlas usage
atlas usage --json
atlas usage at_abc123     # Filter by key prefix

# Revoke a key
atlas keys revoke KEY_UUID
```

### JSON Output

Every command supports `--json` for scripting:

```bash
atlas pulse --json
# → {"slot":291042100,"tps_1m":847,"indexed_txs_24h":1203847,...}

atlas rpc --json
# → {"atlas":{"rpc":"http://...","websocket":"ws://..."},"validator":{...}}

atlas tx SIG --json
# → full TxFactsV1 object

atlas keys list --json
# → {"count":3,"keys":[{"key_prefix":"at_abc...","tier":"pro",...}]}
```

---

## API Reference

All endpoints require `X-API-Key: YOUR_KEY` header unless otherwise noted.

### Transaction History

```bash
# Address history — keyset paginated, no OFFSET, sub-100ms
GET /v1/address/{ADDRESS}/txs?limit=50&sort_order=DESC

# With time and status filters
GET /v1/address/{ADDRESS}/txs?block_time_from=1700000000&block_time_to=1710000000&status=confirmed

# Full transaction facts (TxFactsV1)
GET /v1/tx/{SIGNATURE}

# Enhanced transaction (parsed actions, token deltas, sol deltas)
GET /v1/tx/{SIGNATURE}/enhanced

# Batch fetch (up to 100)
POST /v1/txs/batch
{"signatures":["SIG1","SIG2",...]}

# Human-readable LLM explanation
POST /v1/tx/{SIGNATURE}/explain
```

### Wallet API

```bash
# Token balances with live USD prices (XDex)
GET /v1/wallet/{ADDRESS}/balances

# Transaction history with balance changes
GET /v1/wallet/{ADDRESS}/history?limit=100&type=SWAP

# Token transfers — incoming/outgoing
GET /v1/wallet/{ADDRESS}/transfers

# Wallet identity (exchange, protocol, KOL, etc.)
GET /v1/wallet/{ADDRESS}/identity

# Batch identity lookup (up to 100 addresses)
POST /v1/wallet/batch-identity
{"addresses":["ADDR1","ADDR2",...]}

# Funded-by chain (sybil/compliance)
GET /v1/wallet/{ADDRESS}/funded-by

# One-shot LLM context (TOON format)
GET /v1/wallet/{ADDRESS}/context
```

### Intelligence

```bash
# Behavioral profile + risk score
GET /v1/address/{ADDRESS}/profile?window=7d

# Scores breakdown
GET /v1/address/{ADDRESS}/scores

# Related wallets (co-occurrence graph)
GET /v1/address/{ADDRESS}/related
```

### Token

```bash
# Token metadata + supply
GET /v1/token/{MINT}

# Top holders
GET /v1/token/{MINT}/holders

# Transfer history
GET /v1/token/{MINT}/transfers
```

### Block

```bash
GET /v1/block/{SLOT}
```

### Webhooks

```bash
# Subscribe to address/token/program events
POST /v1/webhooks/subscribe
{"event_type":"address_activity","address":"ADDRESS",
 "url":"https://your-endpoint.com/hook","secret":"your-hmac-secret"}

GET /v1/webhooks/subscriptions
DELETE /v1/webhooks/subscriptions/{ID}
```

### Network

```bash
GET /v1/network/pulse          # Live network stats (TOON or JSON)
POST /v1/tx/send               # Send transaction with priority fee estimation
```

### JSON-RPC  (`POST /rpc`)

Covers all standard Solana RPC methods (proxied to validator) plus Atlas-native methods:

```jsonc
// Enhanced transaction history
{"jsonrpc":"2.0","id":1,"method":"getTransactionsForAddress",
 "params":["ADDRESS",{"limit":100,"sortOrder":"DESC","type":"SWAP"}]}

// Token accounts by owner (served from index)
{"jsonrpc":"2.0","id":1,"method":"getTokenAccountsByOwner",
 "params":["ADDRESS",{"programId":"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"}]}

// Token-2022 aware
{"jsonrpc":"2.0","id":1,"method":"getTokenAccountsByOwnerV2",
 "params":["ADDRESS",{}]}

// Token supply (from index)
{"jsonrpc":"2.0","id":1,"method":"getTokenSupply","params":["MINT"]}

// Top 20 holders
{"jsonrpc":"2.0","id":1,"method":"getTokenLargestAccounts","params":["MINT"]}

// Paginated program accounts
{"jsonrpc":"2.0","id":1,"method":"getProgramAccountsV2",
 "params":["PROGRAM_ID",{"page":1,"limit":100}]}

// Priority fee estimate (all 6 levels)
{"jsonrpc":"2.0","id":1,"method":"getPriorityFeeEstimate",
 "params":[{"accountKeys":["PROGRAM_ID"],"options":{"includeAllPriorityFeeLevels":true}}]}

// DAS API — full Digital Asset Standard
{"jsonrpc":"2.0","id":1,"method":"getAssetsByOwner",
 "params":{"ownerAddress":"ADDRESS","page":1,"limit":100}}

{"jsonrpc":"2.0","id":1,"method":"searchAssets",
 "params":{"ownerAddress":"ADDRESS","tokenType":"fungible"}}
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
│   └── atlas_cli/            # Developer CLI (keygen, rpc, tx, wallet, token, stream, keys, usage)
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
├── .env.example
└── README.md
```

---

## Geyser Plugin (Validator Side)

`plugins/atlas-geyser-v2` is a lightweight, non-blocking Geyser plugin that runs **on the validator** and streams account state changes directly into Atlas's PostgreSQL database. Forked from [x1-geyser-postgres](https://github.com/x1-labs/x1-geyser-postgres).

```bash
# Build (on the validator server — pins Tachyon v2.2.19 ABI)
cd plugins/atlas-geyser-v2
rustup override set 1.84.1
cargo build --release
# Output: target/release/libatlas_geyser.so
```

Configure with `config.example.json`, then add to `validator.sh`:

```bash
--geyser-plugin-config /etc/tachyon/atlas-geyser-config.json \
```

> The validator runs Yellowstone gRPC alongside this plugin — Yellowstone handles transactions, atlas-geyser-v2 handles account state. Neither blocks the validator.

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

| Variable | Default | Description |
|---|---|---|
| `YELLOWSTONE_GRPC_ENDPOINT` | — | Yellowstone gRPC endpoint (required) |
| `DATABASE_URL` | — | PostgreSQL connection string (required) |
| `REDIS_URL` | `redis://localhost:6379` | Redis connection string |
| `ADMIN_API_KEY` | — | Master API key for admin operations (required) |
| `ATLAS_PRICE_API_URL` | `https://api.xdex.xyz/api/token-price/price` | XDex token price oracle |
| `INDEXER_COMMITMENT` | `confirmed` | Indexing commitment: `processed` · `confirmed` · `finalized` |
| `LLM_PROVIDER` | `none` | Explain provider: `none` · `openai` · `anthropic` · `ollama` |
| `LLM_MODEL` | `llama3.2` | Model name (e.g. `gpt-4o`, `claude-3-5-sonnet-20241022`) |
| `ATLAS_PROGRAMS_CONFIG` | `config/programs.yml` | Known program IDs config path |

---

## API Coverage

| Feature | Status | Notes |
|---|---|---|
| `getTransactionsForAddress` | ✅ | Served from index · sort · filter · time range |
| DAS API (10 methods) | ✅ | `getAsset`, `searchAssets`, `getAssetsByOwner`, and more |
| `getPriorityFeeEstimate` | ✅ | All 6 levels: min → unsafeMax from live validator data |
| `getTokenAccountsByOwner` / V2 | ✅ | Served from `token_owner_map` index · Token-2022 aware |
| `getTokenSupply` | ✅ | Served from `token_metadata` table |
| `getTokenLargestAccounts` | ✅ | Top 20 holders from `geyser_accounts` |
| `getProgramAccountsV2` | ✅ | Paginated proxy |
| Wallet balances / history / transfers | ✅ | With live XDex USD pricing |
| Wallet identity + batch identity | ✅ | Up to 100 addresses per request |
| Funded-by chain | ✅ | Sybil detection / compliance |
| Webhooks with HMAC | ✅ | Address · token · program events with retry |
| Standard Solana RPC | ✅ | All methods proxied to validator |
| Developer CLI | ✅ | `atlas` binary — keygen · rpc · tx · wallet · keys · usage |
| MCP server | ✅ | `/mcp` — native AI agent tool provider (`POST /mcp`) |
| WebSocket stream | ✅ | `/v1/stream` — filtered live transaction events |
| Atlas Stream gRPC | 🔄 | Planned — proprietary low-latency streaming service |
| ZK Compression | ➖ | Not available on X1 yet |

---

## Built On

| Component | Technology |
|---|---|
| Validator | [Tachyon v2.2.19](https://github.com/x1-labs/tachyon) — X1 network validator |
| Transaction streaming | [Yellowstone gRPC](https://github.com/rpcpool/yellowstone-grpc) |
| Account streaming | [x1-geyser-postgres](https://github.com/x1-labs/x1-geyser-postgres) (forked → atlas-geyser-v2) |
| Price oracle | [XDex](https://xdex.xyz) — native X1 DEX (`api.xdex.xyz`) |
| API runtime | [Axum](https://github.com/tokio-rs/axum) + Tokio |
| Database | PostgreSQL 15 + Redis 7 |
| Explorer | Next.js 14 |

---

<div align="center">

Built for the X1 network · MIT License

</div>
