# Atlas Wallet Intelligence

## Overview

`atlas_intel` computes deterministic wallet profiles from indexed transaction history.
Profiles are stored per (address, window) in `intelligence_wallet_profiles`.

## Windows

- `24h` — last 24 hours
- `7d`  — last 7 days (default)
- `30d` — last 30 days
- `all` — all-time

## Trigger

The intel worker reads from the `atlas:newtx` Redis stream.
For each address seen in a batch of new transactions, it schedules a profile recompute
with a debounce cooldown (default 60s) to avoid redundant recomputes during high-volume ingest.

## Feature Extraction

Features are computed via direct SQL aggregations over `address_index` + `tx_store`:

| Feature | Description |
|---------|-------------|
| tx_count | Total transactions in window |
| active_days | Distinct calendar days with activity |
| burstiness | Max tx in any 10-minute bucket |
| unique_programs | Distinct programs invoked |
| unique_tokens | Distinct token mints in token_balance_index |
| failure_rate | Failed tx / total tx |
| swap_count | Transactions with SWAP action |
| transfer_count | Transactions with TRANSFER action |
| avg_fee_lamports | Mean fee across window |
| net_xnt_delta | Net XNT flow (positive = received more) |
| has_deploy_actions | Any DEPLOY action detected |

## Scoring (0-100, deterministic)

### automation_score
- +30: burstiness > 10 (high tx density in short windows)
- +20: tx/day > 100
- +10: always uses priority fee
- +20: unique_programs > 20 (scripted, many protocols)

### sniper_score
- +40: swap_count / tx_count > 0.8
- +20: unique_tokens > 50
- +20: burstiness > 5 AND swap_count > 10

### whale_score
- +60: |net_xnt_delta| > 1000 XNT
- +40: |net_xnt_delta| > 100 XNT
- +20: unique_programs > 15 AND unique_tokens > 30

### risk_score
- +30: failure_rate > 0.5
- +15: failure_rate > 0.2
- +20: burstiness > 50

## Classification

```
sniper_score > 80  → "sniper"
automation_score > 80 → "bot"
whale_score > 80   → "whale"
has_deploy_actions → "developer"
tx_count > 0       → "human"
else               → "unknown"
```

## V2 Roadmap

- Related wallets / clusters (`intelligence_wallet_edges`) via co-occurrence analysis
- New mint participation rate (sniper detection enhancement)
- Holding time estimates
- Alert system for watchlisted addresses
