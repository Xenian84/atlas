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
