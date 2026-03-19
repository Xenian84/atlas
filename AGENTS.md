# Atlas — AI Agent Context

## What this project is

**Atlas** is a production-grade X1 blockchain infrastructure platform built on the Tachyon validator (an X1/Solana fork).
It provides real-time indexing, a REST + JSON-RPC API, wallet intelligence, webhooks, and a Next.js explorer.

**Owner / GitHub:** `Xenian84/atlas`

---

## Monorepo layout

```
atlas/
├── crates/
│   ├── atlas_api/          Axum REST + JSON-RPC gateway (port 8080)
│   ├── atlas_indexer/      gRPC Yellowstone stream → PostgreSQL + Redis
│   ├── atlas_webhooks/     Redis stream → HTTP webhook delivery
│   ├── atlas_intel/        Wallet scoring + behaviour profiling
│   ├── atlas_geyser_v2/    Non-blocking Geyser plugin (account-only)
│   ├── atlas_parser/       Transaction fact extraction (swap/transfer/etc.)
│   ├── atlas_cli/          CLI tool (`atlas` binary)
│   └── atlas_types/        Shared types (RawTx, TxFacts, etc.)
├── apps/
│   └── explorer/           Next.js 14 explorer (port 3000)
├── infra/
│   ├── migrations/         PostgreSQL SQL migrations (001–016+)
│   └── docker/             Docker configs
└── config/
    ├── programs.yml        Known program labels
    └── spam.toml           Spam filter config
```

---

## Key environment variables (`.env`)

```
VALIDATOR_RPC_URL=http://127.0.0.1:8899
YELLOWSTONE_GRPC_ENDPOINT=http://127.0.0.1:10000
DATABASE_URL=postgres://atlas:atlas@localhost:5432/atlas
REDIS_URL=redis://127.0.0.1:6379
ADMIN_API_KEY=atlas-dev-key-change-me
```

---

## Server topology

- **Server A** — X1 Tachyon validator ONLY. Do not run Atlas services here.
- **Server B** — Full Atlas stack: API, indexer, webhooks, intel, explorer.

---

## Design decisions

- All API routes live under `/v1/`. Auth via `X-API-Key` header.
- The explorer proxies all API calls through `/api/atlas/[...path]/route.ts` so the API key never leaks to the browser.
- PostgreSQL is the source of truth; Redis is used for event streams (`atlas:newtx`) and caching.
- The explorer design system uses IBM Plex Sans + IBM Plex Mono, zero border-radius (Orb-inspired), HSL CSS variables, and shimmer skeleton loading.

---

## MCP / TOON tooling

This workspace uses the **toon-context** MCP server for semantic code search and structured output.

- Use `toon_search` instead of Grep/Glob when searching code.
- Use `toon_read_file` instead of Read for source files.
- TOON format is preferred over JSON for all structured output (40% fewer tokens, higher accuracy).
- If the MCP server is disconnected, fall back to Grep/Glob/Read.

See `.cursor/rules/toon-context.mdc` and `.cursor/skills/toon-context-workflow/SKILL.md` for full instructions.

---

## Common tasks

```bash
# Build everything
cargo build --release

# Run migrations
sqlx migrate run --source infra/migrations --database-url $DATABASE_URL

# Start API
./target/release/atlas-api

# Start explorer (dev)
cd apps/explorer && npm run dev

# Start indexer
./target/release/atlas-indexer

# CLI
atlas keygen
atlas status
atlas balance <address>
```
