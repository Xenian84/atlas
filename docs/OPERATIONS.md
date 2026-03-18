# Atlas Operations Guide

## Prerequisites

- Server A: X1 validator running with Yellowstone gRPC plugin enabled
- Server B: Docker + docker-compose OR bare metal with Rust + Node.js

## Quick Start (Docker, Server B)

```bash
cd /root/atlas

# 1. Configure environment
cp .env.example .env
# Edit .env: set YELLOWSTONE_GRPC_ENDPOINT, DATABASE_URL, ADMIN_API_KEY

# 2. Start all services
docker compose -f infra/docker-compose.serverB.yml up -d

# 3. Verify
curl http://localhost:8080/health
```

## One-Server Mode

Run validator + Atlas on the same machine:

```bash
# Keep validator RPC private (default: :8899 on loopback)
VALIDATOR_RPC_URL=http://127.0.0.1:8899

# Yellowstone gRPC on loopback too
YELLOWSTONE_GRPC_ENDPOINT=http://127.0.0.1:10000

# Separate disk for Postgres data vs ledger (recommended)
# Mount: /mnt/nvme1 = ledger, /mnt/nvme2 = postgres data
```

## Bare Metal (systemd)

```bash
# Build
cd /root/atlas
cargo build --release

# Install binaries
sudo cp target/release/atlas-{indexer,api,webhooks,intel} /usr/local/bin/

# Install service units
sudo cp infra/systemd/*.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now atlas-indexer atlas-api atlas-webhooks atlas-intel
```

## Database Migrations

Apply all migrations in order:
```bash
for f in infra/migrations/*.sql; do
    psql $DATABASE_URL -f $f
done
```

Or use sqlx-cli:
```bash
cargo install sqlx-cli
sqlx migrate run --source infra/migrations
```

## Backfill Historical Data

```bash
atlas-indexer backfill --from-slot 280000000 --to-slot 281000000 --batch 10
```

Progress is saved in `indexer_state.backfill_progress`.
Safe to restart — will re-process slots idempotently.

## Postgres Tuning (NVMe)

Add to `postgresql.conf`:
```
shared_buffers     = 4GB          # 25% of RAM
effective_cache_size = 12GB
wal_compression    = on
checkpoint_timeout = 15min
max_wal_size       = 4GB
work_mem           = 32MB
random_page_cost   = 1.1          # NVMe
log_min_duration_statement = 500  # slow query log
```

## Monitoring

- Indexer Prometheus metrics: `http://localhost:9100/metrics`
- API metrics: `http://localhost:8080/metrics`
- Key metrics to alert on:
  - `atlas_ingest_lag_ms > 5000` → indexer falling behind
  - `atlas_reconnects_total` rate → gRPC instability
  - `atlas_errors_total` rate → parsing/DB errors
  - `atlas_webhook_failures_total` → delivery problems

## Security Checklist

- [ ] Yellowstone gRPC endpoint NOT publicly exposed (firewall Server A)
- [ ] Internal RPC :8899 NOT publicly exposed
- [ ] `ADMIN_API_KEY` changed from default
- [ ] API keys managed via `api_keys` table (not env)
- [ ] Nginx rate limiting enabled
- [ ] HTTPS termination at Nginx (add certs to `infra/nginx/certs/`)
- [ ] No secrets in container environment (use secrets manager or env file with restricted perms)

## V2 Roadmap Items (not yet implemented)

- Streams: public gRPC endpoint for live tx/account subscriptions
- Sender-lite: `POST /v1/tx/send` broadcast endpoint
- Shred-lite: earlier-than-geyser tx detection
- Priority fee estimator: `GET /v1/fees/priority`
- DAS-lite: `getAssetsByOwner` etc. (depends on X1 NFT standards)
- LLM-backed explain: replace templates with API call using factsToon as context
- Validator intelligence: skip rate, vote latency, stake distribution endpoints
