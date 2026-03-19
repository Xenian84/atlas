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
