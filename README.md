# Atlas — X1 Blockchain Infrastructure Platform

Production-grade Helius-parity+ indexer, API, webhooks, and explorer for the X1 network.

## Features

- **Indexed history** — `getTransactionsForAddress` with keyset cursor pagination (no OFFSET)
- **Enhanced transactions** — parsed actions, token deltas, XNT deltas per tx
- **Webhooks** — address/token/program activity with HMAC signing and retry
- **Wallet intelligence** — bot/sniper/whale/developer classification + risk scoring
- **Explorer** — Orb-like Next.js UI with search, address history, tx explain, TOON copy
- **TOON output** — optional token-efficient format for LLM/agent workflows
- **RPC gateway** — Helius-compatible JSON-RPC + validator pass-through with caching

## Architecture

```
Server A (Validator)              Server B (Atlas)
─────────────────────             ──────────────────────────────────
X1 Validator (Tachyon v2.2.19)    atlas-indexer  ←── Yellowstone gRPC
  └── Yellowstone gRPC Producer       └── Postgres + Redis
      (grpc.tachyon1.network)     atlas-api      (Axum :8888)
  └── atlas-geyser-v2 plugin      atlas-webhooks (delivery worker)
      (account state → Postgres)  atlas-intel    (profile worker)
                                  explorer       (Next.js)
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for full data flow.

## Quick Start

```bash
# 1. Clone and configure
git clone ... atlas && cd atlas
cp .env.example .env
# Edit .env — minimum required:
#   YELLOWSTONE_GRPC_ENDPOINT=http://SERVER_A:10000
#   DATABASE_URL=postgres://atlas:atlas@localhost:5432/atlas
#   ADMIN_API_KEY=your-secret-key

# 2. Start (Docker)
docker compose -f infra/docker-compose.serverB.yml up -d

# 3. Test
curl http://localhost:8888/health
curl -H "X-API-Key: your-secret-key" \
     "http://localhost:8888/v1/address/WALLET_ADDRESS/txs?limit=10"
```

## Building from Source

```bash
# Rust services (requires rustup, protobuf-compiler)
cargo build --release

# Explorer
cd apps/explorer && npm install && npm run build

# Geyser plugin (built separately — pins Tachyon v2.2.19 ABI)
cd plugins/atlas-geyser-v2 && cargo build --release
# Output: target/release/libatlas_geyser.so
# Deploy: copy to validator server, configure in validator.sh
```

## Project Structure

```
atlas/
├── crates/                    # Rust workspace — Server B services
│   ├── atlas_types            # TxFactsV1 + all models
│   ├── atlas_common           # Config, logging, auth, metrics
│   ├── atlas_toon             # TOON renderer
│   ├── atlas_parser           # Protocol modules (system/token/swap/stake/deploy/nft)
│   ├── atlas_indexer          # gRPC consumer + DB writer  →  atlas-indexer binary
│   ├── atlas_api              # Axum API gateway           →  atlas-api binary
│   ├── atlas_webhooks         # Webhook delivery worker    →  atlas-webhooks binary
│   ├── atlas_intel            # Wallet intelligence worker →  atlas-intel binary
│   ├── atlas_shredstream      # Shred relay helper
│   └── atlas_alerter          # Alerting worker
├── plugins/
│   └── atlas-geyser-v2/       # Geyser plugin (Server A — Validator side)
│       ├── src/               # Non-blocking account writer → Postgres
│       ├── config.example.json
│       └── README.md          # Deploy + config guide
├── apps/
│   └── explorer/              # Next.js 14 Orb-like UI
├── infra/
│   ├── migrations/            # SQL migration files (001–016)
│   ├── docker-compose.serverB.yml
│   ├── nginx/
│   └── systemd/               # Unit files for all services
├── config/
│   ├── programs.yml           # X1 + Solana program IDs (XDex confirmed)
│   ├── tags.yml               # Tag classification rules
│   └── spam.yml               # Token/program denylist
├── docs/
│   ├── ARCHITECTURE.md
│   ├── API.md
│   ├── TOON.md
│   ├── INTELLIGENCE.md
│   └── OPERATIONS.md
├── .env.example
└── README.md
```

## API Examples

```bash
# Transaction history (JSON)
curl -H "X-API-Key: KEY" \
  "http://localhost:8888/v1/address/WALLET/txs?limit=50"

# Transaction history (TOON — compact for LLM)
curl -H "X-API-Key: KEY" -H "Accept: text/toon" \
  "http://localhost:8888/v1/address/WALLET/txs?limit=50"

# Full tx facts
curl -H "X-API-Key: KEY" \
  "http://localhost:8888/v1/tx/SIGNATURE"

# Explain tx
curl -X POST -H "X-API-Key: KEY" \
  "http://localhost:8888/v1/tx/SIGNATURE/explain"

# Wallet intelligence profile
curl -H "X-API-Key: KEY" \
  "http://localhost:8888/v1/address/WALLET/profile?window=7d"

# Wallet identity
curl -H "X-API-Key: KEY" \
  "http://localhost:8888/v1/wallet/WALLET/identity"

# Helius-compatible JSON-RPC
curl -X POST -H "Content-Type: application/json" \
  http://localhost:8888/rpc \
  -d '{"jsonrpc":"2.0","id":1,"method":"getTransactionsForAddress","params":["WALLET",{"limit":100}]}'

# DAS API — assets by owner
curl -X POST -H "X-API-Key: KEY" -H "Content-Type: application/json" \
  http://localhost:8888/rpc \
  -d '{"jsonrpc":"2.0","id":1,"method":"getAssetsByOwner","params":{"ownerAddress":"WALLET","page":1}}'

# Create webhook
curl -X POST -H "X-API-Key: KEY" -H "Content-Type: application/json" \
  http://localhost:8888/v1/webhooks/subscribe \
  -d '{"event_type":"address_activity","address":"WALLET","url":"https://your.endpoint/hook","secret":"s3cr3t"}'
```

## Backfill

```bash
atlas-indexer backfill --from-slot 280000000 --to-slot 281000000
```

## V2 Roadmap

- [x] DAS API (NFT/token asset queries — 10 JSON-RPC methods)
- [x] Priority fee estimator (`getPriorityFeeEstimate`)
- [x] Transaction sender (`POST /v1/tx/send`)
- [x] Wallet API (identity, balances, transfers, funded-by)
- [x] WebSocket stream (`GET /v1/stream`) — filtered live tx events
- [x] Related wallets (`GET /v1/address/:addr/related`) — co-occurrence graph
- [x] **Atlas MCP Server** (`POST /mcp`) — native tool provider for Claude, OpenClaw, and any MCP agent
- [x] **LLM-backed explain** — configurable provider (Ollama / OpenAI / Anthropic)
- [x] **Network Pulse** (`GET /v1/network/pulse`) — compact TOON snapshot for agent heartbeats
- [x] **Wallet Context** (`GET /v1/wallet/:addr/context`) — one-shot TOON for LLM prompt injection
- [x] **OpenClaw Skill** (`config/atlas-skill/SKILL.md`) — ready-to-install skill
- [ ] ZK Compression API
- [ ] Shred delivery
- [ ] Program analytics dashboards (`/program/[id]`, `/analytics`)
