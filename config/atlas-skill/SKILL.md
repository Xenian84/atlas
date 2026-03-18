# Atlas X1 Blockchain Data Skill

## What this skill does

Gives your agent full read access to the Atlas blockchain indexer for the X1 network.
Use this to investigate wallets, decode transactions, track token flows, and monitor
network activity — all in compact TOON format optimised for LLM consumption.

## Setup

1. Set `ATLAS_API_URL` and `ATLAS_API_KEY` in your OpenClaw secrets:
   ```
   openclaw secret set ATLAS_API_URL http://YOUR_ATLAS_SERVER:8888
   openclaw secret set ATLAS_API_KEY your-admin-or-user-key
   ```

2. Add to your `AGENTS.md`:
   ```
   skills:
     - atlas-x1
   ```

## Available Tools (via Atlas MCP server at $ATLAS_API_URL/mcp)

| Tool | When to use |
|------|-------------|
| `network_pulse` | Check X1 network health, TPS, top programs. Good for heartbeat monitoring. |
| `get_wallet_context` | Everything about a wallet in one call — balance, scores, recent txs, related wallets. |
| `explain_transaction` | Decode what a transaction did in plain English. |
| `get_transaction` | Raw TxFactsV1 with all actions, token deltas, XNT balance changes. |
| `get_address_history` | List recent transactions for a wallet. |
| `get_wallet_profile` | Bot/whale/sniper/human classification + risk score. |
| `get_assets_by_owner` | List NFTs and tokens owned by a wallet. |
| `get_related_wallets` | Find co-occurring wallets (cluster analysis). |

## MCP connection (for Claude + MCP-compatible agents)

```json
{
  "mcpServers": {
    "atlas-x1": {
      "url": "http://YOUR_ATLAS_SERVER:8888/mcp"
    }
  }
}
```

## Direct REST calls (for non-MCP agents)

```bash
# Network health snapshot (TOON, <200 tokens)
curl -s -H "Accept: text/toon" \
     -H "X-API-Key: $ATLAS_API_KEY" \
     "$ATLAS_API_URL/v1/network/pulse"

# Full wallet context for LLM prompt (TOON)
curl -s -H "Accept: text/toon" \
     -H "X-API-Key: $ATLAS_API_KEY" \
     "$ATLAS_API_URL/v1/wallet/WALLET_ADDRESS/context"

# Explain a transaction
curl -s -X POST \
     -H "X-API-Key: $ATLAS_API_KEY" \
     "$ATLAS_API_URL/v1/tx/SIGNATURE/explain"

# Live transaction stream (WebSocket)
# Connect: ws://YOUR_ATLAS_SERVER:8888/v1/stream?key=$ATLAS_API_KEY
# Subscribe: {"subscribe":{"addresses":["WALLET"],"types":["SWAP","TRANSFER"]}}
```

## HEARTBEAT.md suggestion

Add to your `HEARTBEAT.md` to monitor X1 network activity automatically:

```markdown
## Atlas X1 Monitoring

Every heartbeat:
1. Call `network_pulse` tool
2. If `tps_1m` drops below 100, send alert via Telegram
3. If `indexed_txs_24h` is 0, the indexer may be down — notify
4. Log top_programs and top_tags for trend analysis

Wallets to watch: (add your list here)
- For each watched wallet, call `get_wallet_context` if not checked in last hour
- Flag any wallet whose `risk` score exceeds 70
```

## Response format

All REST endpoints support both JSON and TOON:
- Default: `application/json`
- TOON (40% fewer tokens, better for LLMs): `Accept: text/toon` or `?format=toon`

TOON is strongly recommended when injecting Atlas data into LLM prompts.

## Example agent prompt

```
You are an X1 blockchain analyst. You have access to Atlas X1 data.

Use the `get_wallet_context` tool to analyse this wallet: {{address}}

Then:
1. Summarise what kind of wallet this is (bot/human/whale/sniper)
2. List its top 3 recent activities
3. Flag any risk indicators
4. List related wallets and explain why they are connected
```
