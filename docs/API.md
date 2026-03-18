# Atlas API Reference

Base URL: `http://your-server:8888`

## Authentication

All `/v1/*` endpoints require `X-API-Key: YOUR_KEY` header.
Rate limit: configurable per key (default 300 req/min).
`/health` and `/rpc` are public.

## Content Negotiation

- Default: `application/json`
- TOON: `Accept: text/toon` or `?format=toon`

---

## Health

### GET /health
Returns DB connectivity and version info.
```json
{ "status": "ok", "db": true, "chain": "x1", "v": "atlas.v1" }
```

---

## JSON-RPC (Helius compatible)

### POST /rpc

#### getTransactionsForAddress
```json
{
  "jsonrpc": "2.0", "id": 1,
  "method":  "getTransactionsForAddress",
  "params":  ["ADDRESS", {
    "limit":  100,
    "before": "SLOT:POS",
    "type":   "all"
  }]
}
```
Response: `TxHistoryPage` (see below).

#### Pass-through methods (cached)
- `getLatestBlockhash`
- `getSlot`
- `getBlockHeight`
- `getBlockTime`

---

## REST Endpoints

### GET /v1/tx/:sig
Full `TxFactsV1` for one transaction.

### GET /v1/tx/:sig/enhanced
Actions + token deltas only (compact).

### POST /v1/tx/:sig/explain
```json
{
  "facts":     { ... TxFactsV1 ... },
  "explain":   { "summary": "...", "bullets": [...], "confidence": 0.9 },
  "factsToon": "tx:\n sig: ...\n..."
}
```

### GET /v1/address/:addr/txs
Query params: `limit` (max 100), `before` (cursor), `type` (all|balanceChanged), `format`.
Returns `TxHistoryPage`.

### GET /v1/address/:addr/profile
Query: `window=7d` (24h|7d|30d|all).
Returns wallet intelligence profile with scores + features.

### GET /v1/address/:addr/scores
Compact scores-only response.

### POST /v1/webhooks/subscribe
```json
{
  "event_type": "address_activity",
  "address":    "WALLET_PUBKEY",
  "url":        "https://your-endpoint.example.com/hook",
  "secret":     "your-hmac-secret",
  "format":     "json"
}
```

### GET /v1/webhooks/subscriptions
Lists all active subscriptions.

---

## Models

### TxHistoryPage
```json
{
  "address":      "ABC...",
  "limit":        100,
  "next_cursor":  "280000000:12",
  "transactions": [ ... TxSummary ... ]
}
```

### TxSummary
```json
{
  "signature":    "5xZ...",
  "slot":         280000000,
  "pos":          12,
  "block_time":   1709000000,
  "status":       "success",
  "fee_lamports": 5000,
  "tags":         ["swap"],
  "action_types": ["SWAP"],
  "actions":      [ ... ],
  "token_deltas": [ ... ]
}
```

### Action
```json
{ "t": "SWAP", "p": "X1DEX", "s": "ABC...", "x": "POOL...", "amt": { ... } }
```

### TokenDelta
```json
{
  "mint":      "MINT...",
  "owner":     "WALLET...",
  "delta":     "1000000",
  "decimals":  6,
  "symbol":    "USDC",
  "direction": "in"
}
```
