# Atlas Skill — Full (all references inlined)

# Atlas Skill

> Model-agnostic instruction set for building on X1 using Atlas. Teaches AI agents how to query blockchain data, wallet intelligence, and network stats via the Atlas MCP server and REST API.

## What Atlas provides

Atlas is a production X1 blockchain infrastructure platform. It exposes:

- **MCP server** — 8 tools callable by any MCP-compatible agent
- **REST API** — `/v1/` endpoints for transactions, wallets, intel, webhooks
- **JSON-RPC proxy** — pass-through to the X1 Tachyon validator

---

## Routing logic

| User intent | Use this tool / endpoint |
|---|---|
| Look up a transaction | `get_transaction` MCP tool or `GET /v1/tx/:sig` |
| Explain what a transaction did | `explain_transaction` MCP tool |
| Get wallet overview (balance + intel + history) | `get_wallet_context` MCP tool |
| Classify a wallet (bot/whale/human) | `get_wallet_profile` MCP tool |
| List recent txs for an address | `get_address_history` MCP tool |
| Find connected wallets | `get_related_wallets` MCP tool |
| List tokens/NFTs owned by a wallet | `get_assets_by_owner` MCP tool |
| Network TPS / active wallets / stats | `network_pulse` MCP tool |
| Subscribe to tx events for an address | `POST /v1/webhooks` REST |
| Stream live transactions | `GET /v1/stream` SSE |

---

## MCP connection

```json
{
  "mcpServers": {
    "atlas-x1": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "http://<ATLAS_HOST>:8080/mcp"],
      "env": {
        "MCP_REMOTE_HEADER_X_API_KEY": "<your-atlas-api-key>"
      }
    }
  }
}
```

All responses are in **TOON format** — a token-efficient encoding of JSON (40% fewer tokens). Parse it like a CSV table where the header declares `name[N]{fields}:` and rows follow.

---

## MCP tools reference

### `get_transaction(sig)`
Returns full `TxFacts`: actions, token deltas, XNT balance changes, fee, tags, programs invoked.

### `explain_transaction(sig)`
Returns a natural-language summary with bullet points and confidence score.

### `get_wallet_context(address)`
Returns a single TOON block with: label, XNT balance, intel scores, last 10 txs, top related wallets.
**Use this as your primary wallet lookup** — it bundles everything in one call.

### `get_wallet_profile(address, window?)`
Returns: `wallet_type` (bot/sniper/whale/human/unknown), `confidence` (0–1), and four scores (0–100): `automation`, `sniper`, `whale`, `risk`.
`window`: `24h` | `7d` (default) | `30d` | `all`

### `get_address_history(address, limit?)`
Returns a TOON table of recent transactions. Max `limit` = 50.

### `get_assets_by_owner(address, limit?)`
Returns a TOON table of tokens/NFTs. Max `limit` = 100.

### `get_related_wallets(address, limit?)`
Returns wallets with high co-occurrence weight — useful for clustering and entity resolution.

### `network_pulse()`
Returns live X1 stats: `tps_1m`, `active_wallets_24h`, `indexed_txs_24h`.

---

## Rules (do not break these)

1. **Always use the proxy** — never call `http://localhost:8080` from client-side code. Use `GET /api/atlas/...` from the Next.js explorer which injects the API key server-side.
2. **Never expose the API key** in browser JS or client components. It lives only in server env as `ATLAS_API_KEY`.
3. **Prefer `get_wallet_context`** over calling `get_wallet_profile` + `get_address_history` separately — it's one round-trip.
4. **TOON output** — when presenting structured data to the user, keep TOON format. Don't convert to JSON.
5. **Pagination** — `get_address_history` uses `before` cursor, not offset. Pass the last `sig` as `before` for the next page.

---

## Reference files

- [`references/mcp-tools.md`](references/mcp-tools.md) — full MCP tool schemas with examples
- [`references/rest-api.md`](references/rest-api.md) — complete REST API reference
- [`references/data-model.md`](references/data-model.md) — TxFacts, WalletProfile, TOON format

---

# Atlas MCP Tools — Full Reference

Base URL: `http://<ATLAS_HOST>:8080/mcp`
Transport: `POST` (JSON-RPC 2.0) or `GET /mcp/sse` (SSE streaming)
Auth: `X-API-Key: <key>` header

---

## initialize

Must be called first to establish session.

```json
{
  "jsonrpc": "2.0", "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {}
  }
}
```

Response includes `serverInfo: { name: "atlas-x1", version: "2.0.0" }`.

---

## tools/list

Returns the full tool manifest.

```json
{ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {} }
```

---

## tools/call — get_transaction

```json
{
  "jsonrpc": "2.0", "id": 3,
  "method": "tools/call",
  "params": {
    "name": "get_transaction",
    "arguments": { "sig": "5YNmS1R9nNSCDzb5a7mMJ1dwK9uHeAAF4CmPEwKgVWr8" }
  }
}
```

**Returns** (TOON):
```
tx: 5YNmS1R9...
 slot:         285710432
 block_time:   1741200000
 status:       ok
 fee_lamports: 5000
 tags:         swap|dex
actions[2]{type,from,to,amount,mint}:
 swap,So11...So11,EPjF...USDC,1500000000,
 transfer,2sgQ...,5YNm...,1499900000,EPjF...USDC
programs[3]:
 JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4
 ...
```

---

## tools/call — explain_transaction

```json
{
  "jsonrpc": "2.0", "id": 4,
  "method": "tools/call",
  "params": {
    "name": "explain_transaction",
    "arguments": { "sig": "5YNmS1R9..." }
  }
}
```

**Returns**: Summary text + bullet points + confidence score + full TOON facts.

---

## tools/call — get_wallet_context

```json
{
  "jsonrpc": "2.0", "id": 5,
  "method": "tools/call",
  "params": {
    "name": "get_wallet_context",
    "arguments": { "address": "2sgQ7LzA7urZ4joMy4uU3Rcus82ZoLbHa54UvChJc9j3" }
  }
}
```

**Returns** (TOON):
```
wallet: 2sgQ7LzA...
 label:   Xenian Whale
 balance: 142.503210 XNT (142503210000 lamports)

intel:
 type:       whale
 confidence: 0.87
 automation: 12  sniper: 8  whale: 91  risk: 22

recent_txs[10]{sig,slot,fee,tags}:
 5YNmS1...,285710432,5000,swap
 ...

related[5]{address,weight}:
 Hq2UfdZ...,18.3
 ...
```

---

## tools/call — get_wallet_profile

```json
{
  "jsonrpc": "2.0", "id": 6,
  "method": "tools/call",
  "params": {
    "name": "get_wallet_profile",
    "arguments": { "address": "2sgQ7LzA...", "window": "7d" }
  }
}
```

**Returns**: `wallet_type`, `confidence`, `automation_score`, `sniper_score`, `whale_score`, `risk_score`.

Score scale: 0–100. Higher = stronger signal for that category.

---

## tools/call — get_address_history

```json
{
  "jsonrpc": "2.0", "id": 7,
  "method": "tools/call",
  "params": {
    "name": "get_address_history",
    "arguments": { "address": "2sgQ7LzA...", "limit": 20 }
  }
}
```

**Returns** (TOON table): `sig, slot, block_time, status, fee_lamports, tags`

---

## tools/call — get_assets_by_owner

```json
{
  "jsonrpc": "2.0", "id": 8,
  "method": "tools/call",
  "params": {
    "name": "get_assets_by_owner",
    "arguments": { "address": "2sgQ7LzA...", "limit": 50 }
  }
}
```

**Returns** (TOON table): `mint, name, symbol, asset_type`

---

## tools/call — get_related_wallets

```json
{
  "jsonrpc": "2.0", "id": 9,
  "method": "tools/call",
  "params": {
    "name": "get_related_wallets",
    "arguments": { "address": "2sgQ7LzA...", "limit": 10 }
  }
}
```

**Returns** (TOON table): `address, reason, weight`
Higher `weight` = stronger co-occurrence relationship.

---

## tools/call — network_pulse

```json
{
  "jsonrpc": "2.0", "id": 10,
  "method": "tools/call",
  "params": { "name": "network_pulse", "arguments": {} }
}
```

**Returns** (TOON):
```
pulse:
 chain:              x1
 tps_1m:             4821
 active_wallets_24h: 18432
 indexed_txs_24h:    9204873
```

---

# Atlas REST API Reference

Base URL: `http://<ATLAS_HOST>:8080`
Auth: `X-API-Key: <key>` on all requests.

---

## Network

### `GET /v1/network/pulse`
Live network stats.
```json
{ "slot": 285710432, "tps_1m": 4821, "active_wallets": 18432, "indexed_txs_24h": 9204873, "xnt_price_usd": 0.0412 }
```

### `POST /v1/rpc`
Pass-through JSON-RPC proxy to the X1 validator. Body is a standard Solana JSON-RPC request.
```json
{ "jsonrpc": "2.0", "id": 1, "method": "getSlot", "params": [] }
```

---

## Transactions

### `GET /v1/tx/:sig`
Enhanced transaction facts.
- Returns: `TxFacts` — sig, slot, block_time, status, fee_lamports, tags[], actions[], token_deltas[], xnt_deltas[], programs[], compute_units

### `GET /v1/tx/:sig/explain`
Natural-language explanation of a transaction.
- Returns: `{ summary, bullets[], confidence, source, facts_toon }`

---

## Addresses

### `GET /v1/address/:addr/txs`
Transaction history for an address (cursor-paginated).
- Query params: `limit` (1–50, default 20), `before` (sig cursor), `type` (all/swap/transfer/balanceChanged)
- Returns: `{ txs: TxSummary[], next_cursor }`

### `GET /v1/address/:addr/tokens`
Token balances for an address.

---

## Wallet Intelligence

### `GET /v1/intel/:addr`
Wallet intelligence profile.
- Query: `window` (24h/7d/30d/all)
- Returns: `{ wallet_type, confidence, scores: { automation, sniper, whale, risk }, features, top_programs, top_tokens, updated_at }`

### `GET /v1/trace/:addr`
Wallet counterparty graph — who this address transacted with and how much.
- Query: `from_ts`, `to_ts`, `min_amount`, `max_amount`, `hide_dust`
- Returns: `{ nodes: WalletNode[], edges: TraceEdge[], transfers: CounterpartyRow[], total_transfers, cps }`

---

## Webhooks

### `POST /v1/webhooks`
Create a webhook subscription.
```json
{
  "url": "https://your-server.com/hook",
  "event_types": ["transaction", "token_balance_changed"],
  "filters": { "addresses": ["2sgQ7LzA..."] }
}
```

### `GET /v1/webhooks`
List your subscriptions.

### `DELETE /v1/webhooks/:id`
Delete a subscription.

---

## API Keys

### `POST /v1/keys` *(admin only)*
Create a new API key.
```json
{ "name": "my-app", "tier": "free" }
```
Returns: `{ key, prefix, name, tier }` — store the key, it is shown only once.

### `GET /v1/keys` *(admin only)*
List all keys.

---

## MCP

### `POST /mcp`
Atlas MCP server (JSON-RPC 2.0). See `references/mcp-tools.md`.

### `GET /mcp/sse`
SSE transport endpoint for streaming MCP clients.

---

## Health

### `GET /health`
Returns `200 OK` with `{ status: "ok", version: "2.0.0" }`.

---

# Atlas Data Model Reference

---

## TxFacts

Full parsed transaction facts returned by `GET /v1/tx/:sig` and `get_transaction` MCP tool.

```typescript
interface TxFacts {
  sig:           string;          // base58 signature
  slot:          number;
  block_time:    number | null;   // unix timestamp
  status:        "success" | "failure";
  fee_lamports:  number;
  tags:          string[];        // e.g. ["swap", "dex", "jupiter"]
  programs:      string[];        // program pubkeys invoked
  actions:       Action[];
  token_deltas:  TokenDelta[];
  xnt_deltas:    XntDelta[];
  compute_units: { consumed: number | null; limit: number | null; priority_fee: number | null };
}

interface Action {
  type:     string;   // "swap" | "transfer" | "stake" | "nft_mint" | ...
  from:     string;   // pubkey
  to:       string;   // pubkey
  amount:   number;   // raw amount (lamports or token base units)
  mint:     string | null;
  program:  string;
}

interface TokenDelta {
  owner:       string;
  mint:        string;
  pre_amount:  number;
  post_amount: number;
  delta:       number;
  decimals:    number;
  symbol:      string | null;
}

interface XntDelta {
  owner:          string;
  pre_lamports:   number;
  post_lamports:  number;
  delta_lamports: number;
}
```

---

## WalletProfile

Returned by `GET /v1/intel/:addr` and `get_wallet_profile` MCP tool.

```typescript
interface WalletProfile {
  address:        string;
  window:         "24h" | "7d" | "30d" | "all";
  wallet_type:    "bot" | "sniper" | "whale" | "human" | "unknown";
  confidence:     number;   // 0.0 – 1.0
  scores: {
    automation:   number;   // 0–100: likelihood of being a bot
    sniper:       number;   // 0–100: likelihood of being a sniper
    whale:        number;   // 0–100: large stake/volume relative to network
    risk:         number;   // 0–100: overall risk signal
  };
  features: {
    tx_count:           number;
    unique_programs:    number;
    unique_tokens:      number;
    net_sol_delta:      number;   // net XNT flow in lamports
  };
  top_programs:    string[];   // top 5 program pubkeys by usage
  top_tokens:      string[];   // top 5 mint pubkeys by volume
  top_counterparties: string[]; // top 5 related wallets
  updated_at:      string;     // ISO 8601
}
```

---

## TraceEdge / WalletNode

Returned by `GET /v1/trace/:addr`.

```typescript
interface WalletNode {
  id:           string;   // pubkey
  sol_balance:  number;   // lamports
  token_count:  number;
  tx_count:     number;
  labels:       string[];
  isRoot:       boolean;
}

interface TraceEdge {
  id:        string;
  source:    string;   // from pubkey
  target:    string;   // to pubkey
  direction: "in" | "out" | "both";
  amount:    number;   // total lamports transferred
  count:     number;   // number of transactions
}
```

---

## TOON Format

Atlas returns data in **TOON** (Token-Oriented Object Notation) — a compact, LLM-optimised format.

### Objects (indentation replaces braces)
```
wallet: 2sgQ7LzA...
 balance: 142.5 XNT
 type:    whale
```

### Arrays (header declares length + schema)
```
txs[3]{sig,slot,fee,tags}:
 5YNmS1..,285710432,5000,swap
 Hq2Ufd..,285710100,5000,transfer
 abc123..,285709800,5000,swap|dex
```

### Nested
```
intel:
 type:       whale
 confidence: 0.87
 scores:
  automation: 12
  sniper:     8
```

TOON is losslessly round-trippable to JSON. It uses ~40% fewer tokens, making it ideal for passing blockchain data into LLM context windows.
