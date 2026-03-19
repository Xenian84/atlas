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
