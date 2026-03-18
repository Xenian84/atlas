# atlas-geyser-v2

A Geyser plugin for the X1 / Tachyon validator (v2.2.19) that streams **account state** directly into Atlas's PostgreSQL schema.

Forked from [x1-geyser-postgres](https://github.com/x1-labs/x1-geyser-postgres) and stripped down to be **account-only** and **zero-blocking**.

> Lives in `plugins/atlas-geyser-v2/` inside the Atlas monorepo.  
> **Built separately** from the main workspace — see Build section below.

---

## What it does

| Table written         | What it stores                                      |
|-----------------------|-----------------------------------------------------|
| `geyser_accounts`     | Every account: pubkey, lamports, owner, data, slot  |
| `token_owner_map`     | SPL Token v1 + Token-2022: token_account→mint+owner |

Transaction and block data are **not** written here — that is handled by the Yellowstone gRPC plugin running alongside this one.

---

## Architecture

```
Validator thread  ──try_send──►  crossbeam bounded channel (500k cap)
                                         │
                          ┌──────────────┼──────────────┐
                     Worker-0        Worker-1        Worker-3
                    (postgres)      (postgres)      (postgres)
                        │               │               │
                   UNNEST batch    UNNEST batch    UNNEST batch
                        │               │               │
                    geyser_accounts + token_owner_map
```

- **Non-blocking**: if the channel is full, the update is dropped and logged. The validator thread is never stalled.
- **Parallel**: N configurable workers each hold one long-lived postgres connection.
- **Batch UNNEST**: one round-trip per batch via PostgreSQL `UNNEST` upsert.
- **Auto-reconnect**: if postgres goes down, workers reconnect automatically.

---

## PostgreSQL schema required

```sql
-- Already present in Atlas migrations (014_accounts.sql)
CREATE TABLE IF NOT EXISTS geyser_accounts (
    pubkey      TEXT        PRIMARY KEY,
    lamports    NUMERIC     NOT NULL,
    owner       TEXT        NOT NULL,
    executable  BOOLEAN     NOT NULL DEFAULT false,
    data        BYTEA,
    slot        BIGINT      NOT NULL,
    is_startup  BOOLEAN     NOT NULL DEFAULT false,
    written_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Already present in Atlas migrations
CREATE TABLE IF NOT EXISTS token_owner_map (
    token_account TEXT PRIMARY KEY,
    mint          TEXT NOT NULL,
    owner         TEXT NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

---

## Build

```bash
# Requires Rust 1.84.1 (matches Tachyon toolchain)
rustup override set 1.84.1

cargo build --release
# Output: target/release/libatlas_geyser.so
```

---

## Config

Copy `config.example.json` and edit:

```json
{
  "libpath": "/path/to/libatlas_geyser.so",
  "connection_str": "host=127.0.0.1 port=5432 user=atlas dbname=atlas password=YOUR_PASSWORD",
  "threads": 4,
  "batch_size": 250,
  "channel_capacity": 500000,
  "log_every": 100000
}
```

Add to `validator.sh`:

```bash
--geyser-plugin-config /etc/tachyon/atlas-geyser-config.json \
```

---

## Differences from x1-geyser-postgres

| Feature                   | x1-geyser-postgres     | atlas-geyser           |
|---------------------------|------------------------|------------------------|
| Tables written            | account, slot, tx, block | geyser_accounts, token_owner_map |
| Transaction indexing      | Yes (optional)         | **No** (Yellowstone does it) |
| Block metadata            | Yes                    | **No**                 |
| Account filtering         | Configurable selectors | **All accounts** (no filter) |
| Channel on full           | Blocks validator       | **Drops + logs** (never blocks) |
| Token parsing             | Owner + mint index tables | Inline into token_owner_map |
| Dependencies              | Heavy (openssl, ssl, metrics) | **Minimal** |
