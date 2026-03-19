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
