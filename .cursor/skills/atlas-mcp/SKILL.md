---
name: atlas-mcp
description: Connect to Atlas as an MCP tool provider. Use when an agent or LLM needs to query X1 blockchain data — wallets, transactions, network stats, wallet intelligence — via the Atlas MCP server.
---

# Atlas MCP Server

Atlas exposes a full **Model Context Protocol (MCP 2024-11-05)** server so any compatible agent (Claude, OpenAI, etc.) can call Atlas tools directly.

## Endpoints

| Transport | URL |
|-----------|-----|
| Stateless JSON-RPC | `POST http://<host>:8080/mcp` |
| SSE streaming      | `GET  http://<host>:8080/mcp/sse` |

Auth: `X-API-Key: <your-key>` header required on every request.

---

## Available tools

### `get_transaction`
Fetch full details of an X1 transaction.
```json
{ "sig": "5YNmS1R9..." }
```
Returns: `TxFacts` in TOON — actions, token deltas, XNT balance changes, tags, programs invoked.

---

### `explain_transaction`
Natural-language explanation of a transaction (LLM-assisted or template).
```json
{ "sig": "5YNmS1R9..." }
```
Returns: summary, bullet points, confidence score, full TOON facts.

---

### `get_wallet_context`
Complete wallet snapshot: label, XNT balance, intelligence scores, recent transactions, related wallets.
```json
{ "address": "2sgQ7LzA..." }
```
Returns: compact TOON — optimised for LLM prompts.

---

### `get_wallet_profile`
Wallet behaviour classification and intelligence scores.
```json
{ "address": "2sgQ7LzA...", "window": "7d" }
```
`window` options: `24h` | `7d` | `30d` | `all`
Returns: `wallet_type` (bot/sniper/whale/human), `confidence`, `automation_score`, `sniper_score`, `whale_score`, `risk_score`.

---

### `get_address_history`
Recent transactions for a wallet.
```json
{ "address": "2sgQ7LzA...", "limit": 20 }
```
Returns: TOON table — sig, slot, block_time, status, fee, tags.

---

### `get_assets_by_owner`
NFTs and tokens owned by a wallet (DAS standard).
```json
{ "address": "2sgQ7LzA...", "limit": 50 }
```
Returns: TOON table — mint, name, symbol, asset_type.

---

### `get_related_wallets`
Wallets frequently co-appearing in transactions with this address.
```json
{ "address": "2sgQ7LzA...", "limit": 10 }
```
Returns: TOON table — peer address, reason, co-occurrence weight.

---

### `network_pulse`
Live X1 network statistics.
```json
{}
```
Returns: TPS (1m avg), active wallets (24h), indexed transactions (24h).

---

## Raw call example (curl)

```bash
# Initialize session
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "X-API-Key: atlas-dev-key-change-me" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{}}}'

# List tools
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "X-API-Key: atlas-dev-key-change-me" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

# Call a tool
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "X-API-Key: atlas-dev-key-change-me" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "get_wallet_context",
      "arguments": { "address": "2sgQ7LzA7urZ4joMy4uU3Rcus82ZoLbHa54UvChJc9j3" }
    }
  }'
```

---

## Connecting from Cursor / Claude Desktop

Add to your `~/.cursor/mcp.json` or `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "atlas-x1": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "http://<SERVER_B_IP>:8080/mcp"],
      "env": {
        "MCP_REMOTE_HEADER_X_API_KEY": "<your-atlas-api-key>"
      }
    }
  }
}
```

Or if your MCP client supports HTTP transport directly, point it at:
- **URL**: `http://<SERVER_B_IP>:8080/mcp`
- **SSE URL**: `http://<SERVER_B_IP>:8080/mcp/sse`
- **Header**: `X-API-Key: <your-atlas-api-key>`

---

## Implementation
- Source: `crates/atlas_api/src/handlers/mcp.rs`
- Protocol: MCP 2024-11-05 (JSON-RPC 2.0)
- All tool results are returned in **TOON format** — 40% fewer tokens than JSON, optimised for LLM consumption
