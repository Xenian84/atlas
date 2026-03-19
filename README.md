<div align="center">

<br/>

<img src="https://img.shields.io/badge/ATLAS-X1%20Infrastructure-e94560?style=for-the-badge&labelColor=0f3460&logoColor=white" height="42"/>

<br/><br/>

**Production-grade blockchain infrastructure for the X1 network**

<br/>

[![Rust](https://img.shields.io/badge/Rust-1.84+-CE422B?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![X1 Network](https://img.shields.io/badge/X1-Mainnet-0f3460?style=flat-square&logo=solana&logoColor=white)](https://x1.xyz)
[![Tachyon](https://img.shields.io/badge/Tachyon-v2.2.19-7c3aed?style=flat-square)](https://github.com/x1-labs/tachyon)
[![License](https://img.shields.io/badge/License-MIT-22c55e?style=flat-square)](LICENSE)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-15+-336791?style=flat-square&logo=postgresql&logoColor=white)](https://postgresql.org)

<br/>

*Full-stack indexer · REST & JSON-RPC API · Webhooks · Wallet Intelligence · Block Explorer · CLI*

<br/>

</div>

---

## What is Atlas?

Atlas connects to a **Tachyon X1 validator** via Yellowstone gRPC, indexes every transaction and account state change into PostgreSQL, and exposes a unified REST + JSON-RPC API with **sub-100ms response times**. It is designed to run on a dedicated infrastructure server (Server B), completely separate from the validator.

<br/>

```mermaid
graph LR
    subgraph A["🖥️  Server A — Validator"]
        direction TB
        T["Tachyon v2.2.19"]
        YS["Yellowstone gRPC"]
        GV2["atlas-geyser-v2\n(account state)"]
        T --> YS
        T --> GV2
    end

    subgraph B["⚡  Server B — Atlas"]
        direction TB
        IDX["atlas-indexer"]
        DB[("PostgreSQL\n+ Redis")]
        API["atlas-api\nAxum · port 8888"]
        WH["atlas-webhooks\ndelivery worker"]
        INT["atlas-intel\nwallet intelligence"]
        EXP["explorer\nNext.js block explorer"]
        IDX --> DB
        DB --> API
        DB --> WH
        DB --> INT
        DB --> EXP
    end

    YS -->|"gRPC stream"| IDX
    GV2 -->|"account state"| DB

    style A fill:#1a1a2e,stroke:#e94560,color:#fff
    style B fill:#0f3460,stroke:#e94560,color:#fff
```

<br/>

---

## Features

<table>
<tr><th width="220">Category</th><th>Capabilities</th></tr>
<tr><td>🔄 <b>Transaction Indexing</b></td><td>Real-time streaming via Yellowstone gRPC · backfill CLI · shred-level ingestion</td></tr>
<tr><td>🗂️ <b>Account Indexing</b></td><td>Non-blocking geyser plugin (atlas-geyser-v2) · SPL Token + Token-2022 ownership maps</td></tr>
<tr><td>🌐 <b>REST API</b></td><td>Address history · tx facts · wallet balances · token accounts · program activity</td></tr>
<tr><td>⚙️ <b>JSON-RPC</b></td><td><code>getTransactionsForAddress</code>, <code>getTokenAccountsByOwner</code>, <code>getTokenSupply</code>, <code>getTokenLargestAccounts</code>, <code>getProgramAccountsV2</code>, full DAS API (10 methods), all standard Solana RPC via proxy</td></tr>
<tr><td>💵 <b>USD Pricing</b></td><td>Live token prices via <a href="https://xdex.xyz">XDex</a> — native X1 DEX price oracle</td></tr>
<tr><td>⛽ <b>Priority Fees</b></td><td><code>getPriorityFeeEstimate</code> with all 6 levels: min · low · medium · high · veryHigh · unsafeMax</td></tr>
<tr><td>🔔 <b>Webhooks</b></td><td>Address · token · program activity triggers with HMAC signing and automatic retry</td></tr>
<tr><td>🧠 <b>Wallet Intelligence</b></td><td>Bot / sniper / whale / developer classification · risk scores · behavioral profiles</td></tr>
<tr><td>🔍 <b>Block Explorer</b></td><td>Next.js 14 UI · transaction detail · address history · TOON output for AI workflows</td></tr>
<tr><td>🤖 <b>MCP Server</b></td><td>Native Model Context Protocol tool provider for Claude and AI agents</td></tr>
<tr><td>💬 <b>LLM Explain</b></td><td>Configurable provider (Ollama / OpenAI / Anthropic) for human-readable tx explanations</td></tr>
<tr><td>🖥️ <b>CLI</b></td><td><code>atlas</code> binary — keygen · rpc · tx · wallet · token · block · stream · keys · usage · <code>--json</code></td></tr>
</table>

<br/>

---

## Quick Start

### Prerequisites

| Requirement | Version |
|---|---|
| Rust | 1.84+ (`rustup`) |
| PostgreSQL | 15+ |
| Redis | 7+ |
| protoc | Protocol Buffers compiler |
| Yellowstone gRPC | `grpc.tachyon1.network` or self-hosted |

<br/>

### 1 · Clone & Configure

```bash
git clone https://github.com/Xenian84/atlas.git && cd atlas
cp .env.example .env
```

Edit `.env` — minimum required:

```env
YELLOWSTONE_GRPC_ENDPOINT=http://YOUR_VALIDATOR_IP:10000
DATABASE_URL=postgres://atlas:YOUR_PASSWORD@localhost:5432/atlas
ADMIN_API_KEY=your-strong-secret-key
```

### 2 · Run Migrations

```bash
for f in infra/migrations/*.sql; do psql "$DATABASE_URL" -f "$f"; done
```

### 3 · Start with Docker

```bash
docker compose -f infra/docker-compose.serverB.yml up -d
```

### 4 · Or Build from Source

```bash
# Build all Rust services + CLI
cargo build --release

# Run each service (reads from .env automatically)
./target/release/atlas-indexer stream   # Live transaction indexing
./target/release/atlas-api              # REST + RPC server  →  :8888
./target/release/atlas-webhooks         # Webhook delivery worker
./target/release/atlas-intel            # Wallet intelligence worker
```

### 5 · Verify

```bash
curl http://localhost:8888/health
# → {"status":"ok","chain":"x1","db":true,"v":"atlas.v1"}

atlas status     # Full system health
atlas pulse      # Network snapshot
```

<br/>

---

## CLI Reference

The `atlas` binary is a developer and operations tool. Add `--json` (or `-j`) to any command for machine-readable output — ideal for scripts, CI pipelines, and AI agents.

```bash
# Install
cp target/release/atlas /usr/local/bin/atlas
export ATLAS_API_URL=http://localhost:8888
export ATLAS_API_KEY=your-key
```

<br/>

<details>
<summary><b>⚡ Onboarding</b></summary>

```bash
# Generate an X1 keypair  (writes ~/.atlas/keypair.json)
atlas keygen
atlas keygen --output /path/to/keypair.json

# Print all RPC endpoint URLs
atlas rpc
atlas rpc --json
```
</details>

<details>
<summary><b>🔎 Data Queries</b></summary>

```bash
atlas tx 5wJb...xyz           # Look up a transaction
atlas wallet ADDRESS          # Wallet overview — balances, history, identity
atlas token MINT_ADDRESS      # Token info + supply
atlas token MINT_ADDRESS --holders   # Top holders
atlas block 291000000         # Block overview
atlas status                  # System health + indexer stats
atlas pulse                   # Live network snapshot
```
</details>

<details>
<summary><b>📡 Live Stream</b></summary>

```bash
atlas stream                       # Last 10 live transaction events
atlas stream --watch --count 50    # Watch continuously (Ctrl-C to stop)
```
</details>

<details>
<summary><b>🔑 API Key Management  (admin only)</b></summary>

```bash
atlas keys list                         # All keys + last-used timestamps
atlas keys list --json
atlas keys create "my-dapp" --rpm 1000  # Create a new key
atlas usage                             # Usage stats per key
atlas usage --json
atlas usage at_abc123                   # Filter by key prefix
atlas keys revoke KEY_UUID              # Revoke a key
```
</details>

<details>
<summary><b>📋 JSON Output Examples</b></summary>

```bash
atlas pulse --json
# → {"slot":291042100,"tps_1m":847,"indexed_txs_24h":1203847,...}

atlas rpc --json
# → {"atlas":{"rpc":"http://...","websocket":"ws://..."},"validator":{...}}

atlas tx SIG --json
# → full TxFactsV1 object

atlas keys list --json
# → {"count":3,"keys":[{"key_prefix":"atlas_abc...","tier":"pro",...}]}
```
</details>

<br/>

---

## API Reference

> All endpoints require `X-API-Key: YOUR_KEY` header unless noted.

<br/>

<details open>
<summary><b>📜 Transaction History</b></summary>

```bash
# Address history — keyset paginated, sub-100ms
GET /v1/address/{ADDRESS}/txs?limit=50&sort_order=DESC

# With time and status filters
GET /v1/address/{ADDRESS}/txs?block_time_from=1700000000&status=confirmed

# Full transaction facts
GET /v1/tx/{SIGNATURE}

# Enhanced (parsed actions, token deltas, SOL deltas)
GET /v1/tx/{SIGNATURE}/enhanced

# Batch fetch (up to 100)
POST /v1/txs/batch
{"signatures":["SIG1","SIG2"]}

# LLM explanation
POST /v1/tx/{SIGNATURE}/explain
```
</details>

<details>
<summary><b>👛 Wallet API</b></summary>

```bash
GET /v1/wallet/{ADDRESS}/balances      # Token balances + live XDex USD prices
GET /v1/wallet/{ADDRESS}/history       # History with balance changes
GET /v1/wallet/{ADDRESS}/transfers     # Incoming/outgoing token transfers
GET /v1/wallet/{ADDRESS}/identity      # Exchange / protocol / KOL classification
GET /v1/wallet/{ADDRESS}/funded-by     # Funded-by chain (sybil/compliance)
GET /v1/wallet/{ADDRESS}/context       # One-shot LLM context (TOON format)

POST /v1/wallet/batch-identity         # Batch identity lookup (up to 100)
{"addresses":["ADDR1","ADDR2"]}
```
</details>

<details>
<summary><b>🧠 Intelligence</b></summary>

```bash
GET /v1/address/{ADDRESS}/profile?window=7d   # Behavioral profile + risk score
GET /v1/address/{ADDRESS}/scores              # Scores breakdown
GET /v1/address/{ADDRESS}/related             # Related wallets (co-occurrence)
```
</details>

<details>
<summary><b>🪙 Token</b></summary>

```bash
GET /v1/token/{MINT}             # Metadata + supply
GET /v1/token/{MINT}/holders     # Top holders
GET /v1/token/{MINT}/transfers   # Transfer history
```
</details>

<details>
<summary><b>🔔 Webhooks</b></summary>

```bash
POST   /v1/webhooks/subscribe
{"event_type":"address_activity","address":"ADDRESS",
 "url":"https://your-endpoint.com/hook","secret":"your-hmac-secret"}

GET    /v1/webhooks/subscriptions
DELETE /v1/webhooks/subscriptions/{ID}
```
</details>

<details>
<summary><b>⚙️ JSON-RPC  (POST /rpc)</b></summary>

All standard Solana RPC methods are proxied to the validator. Atlas-native extensions:

```jsonc
// Enhanced tx history with filters
{"jsonrpc":"2.0","id":1,"method":"getTransactionsForAddress",
 "params":["ADDRESS",{"limit":100,"sortOrder":"DESC","type":"SWAP"}]}

// Token accounts by owner (from index)
{"jsonrpc":"2.0","id":1,"method":"getTokenAccountsByOwner",
 "params":["ADDRESS",{"programId":"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"}]}

// Token-2022 aware
{"jsonrpc":"2.0","id":1,"method":"getTokenAccountsByOwnerV2","params":["ADDRESS",{}]}

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

// DAS API
{"jsonrpc":"2.0","id":1,"method":"getAssetsByOwner",
 "params":{"ownerAddress":"ADDRESS","page":1,"limit":100}}
{"jsonrpc":"2.0","id":1,"method":"searchAssets",
 "params":{"ownerAddress":"ADDRESS","tokenType":"fungible"}}
```
</details>

<br/>

---

## Project Structure

```
atlas/
├── crates/
│   ├── atlas_types/          # TxFactsV1, RawTx, cursor, intelligence + webhook models
│   ├── atlas_common/         # AppConfig, logging, auth middleware, metrics
│   ├── atlas_toon/           # TOON renderer (token-efficient structured output)
│   ├── atlas_parser/         # Protocol modules: system · token · swap · stake · NFT · deploy
│   ├── atlas_indexer/        # Yellowstone gRPC consumer + PostgreSQL writer
│   ├── atlas_api/            # Axum REST + JSON-RPC gateway
│   ├── atlas_webhooks/       # Redis stream listener + HTTP delivery worker
│   ├── atlas_intel/          # Feature extractor + scorer + profile upsert
│   ├── atlas_shredstream/    # Shred relay helper
│   ├── atlas_alerter/        # Alerting worker
│   └── atlas_cli/            # Developer CLI
├── plugins/
│   └── atlas-geyser-v2/      # Geyser plugin (Validator side — built separately)
│       ├── src/              # Non-blocking account writer → PostgreSQL
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

<br/>

---

## Geyser Plugin

`plugins/atlas-geyser-v2` is a lightweight, non-blocking Geyser plugin that runs **on the validator** and streams account state changes directly into Atlas's PostgreSQL. Forked from [x1-geyser-postgres](https://github.com/x1-labs/x1-geyser-postgres).

```bash
# Build on the validator server (pins Tachyon v2.2.19 ABI)
cd plugins/atlas-geyser-v2
rustup override set 1.84.1
cargo build --release
# Output: target/release/libatlas_geyser.so
```

Configure with `config.example.json`, then add to `validator.sh`:

```bash
--geyser-plugin-config /etc/tachyon/atlas-geyser-config.json \
```

> **Note:** Yellowstone gRPC handles transactions, atlas-geyser-v2 handles account state. Neither plugin blocks the validator.

<br/>

---

## Environment Variables

See `.env.example` for the full reference. Key variables:

| Variable | Default | Description |
|---|---|---|
| `YELLOWSTONE_GRPC_ENDPOINT` | *(required)* | Yellowstone gRPC endpoint |
| `DATABASE_URL` | *(required)* | PostgreSQL connection string |
| `REDIS_URL` | `redis://localhost:6379` | Redis connection string |
| `ADMIN_API_KEY` | *(required)* | Master key for admin operations |
| `ATLAS_PRICE_API_URL` | `https://api.xdex.xyz/api/token-price/price` | XDex token price oracle |
| `INDEXER_COMMITMENT` | `confirmed` | `processed` · `confirmed` · `finalized` |
| `LLM_PROVIDER` | `none` | `none` · `openai` · `anthropic` · `ollama` |
| `LLM_MODEL` | `llama3.2` | e.g. `gpt-4o`, `claude-3-5-sonnet-20241022` |
| `ATLAS_PROGRAMS_CONFIG` | `config/programs.yml` | Known program IDs config |

<br/>

---

## API Coverage

| Feature | Status | Notes |
|---|:---:|---|
| `getTransactionsForAddress` | ✅ | Served from index · sort · filter · time range |
| DAS API (10 methods) | ✅ | `getAsset`, `searchAssets`, `getAssetsByOwner`, and more |
| `getPriorityFeeEstimate` | ✅ | All 6 levels: min → unsafeMax |
| `getTokenAccountsByOwner` / V2 | ✅ | Served from index · Token-2022 aware |
| `getTokenSupply` | ✅ | Served from `token_metadata` table |
| `getTokenLargestAccounts` | ✅ | Top 20 holders from `geyser_accounts` |
| `getProgramAccountsV2` | ✅ | Paginated proxy |
| Wallet balances / history / transfers | ✅ | With live XDex USD pricing |
| Wallet identity + batch identity | ✅ | Up to 100 addresses per request |
| Funded-by chain | ✅ | Sybil detection / compliance |
| Webhooks with HMAC | ✅ | Address · token · program events with retry |
| Standard Solana RPC | ✅ | All methods proxied to validator |
| Developer CLI | ✅ | `atlas` binary — keygen · rpc · tx · wallet · keys · usage |
| MCP server | ✅ | `/mcp` — native AI agent tool provider |
| WebSocket stream | ✅ | `/v1/stream` — filtered live transaction events |
| Atlas Stream gRPC | 🔄 | Planned — proprietary low-latency streaming service |
| ZK Compression | ➖ | Not available on X1 yet |

<br/>

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

<br/>

---

<div align="center">

Built for the X1 network &nbsp;·&nbsp; MIT License

</div>
