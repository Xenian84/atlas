---
name: atlas-deploy
description: Deploy, start, stop, or diagnose Atlas services on Server B. Use when setting up a new server, running migrations, managing systemd services, or debugging production issues.
---

# Atlas Deploy Skill

## Server topology
- **Server A** — X1 Tachyon validator ONLY. Never run Atlas services here.
- **Server B** — Full Atlas stack. This is where you deploy.

## First-time setup on a new server

```bash
# 1. Clone
git clone https://github.com/Xenian84/atlas.git && cd atlas

# 2. Install Rust (if not present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 3. Install system deps
apt-get install -y postgresql redis-server pkg-config libssl-dev

# 4. Create DB and user
sudo -u postgres psql -c "CREATE USER atlas WITH PASSWORD 'atlas';"
sudo -u postgres psql -c "CREATE DATABASE atlas OWNER atlas;"

# 5. Copy and edit env
cp .env.example .env   # edit VALIDATOR_RPC_URL to point at Server A

# 6. Run migrations
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run --source infra/migrations --database-url postgres://atlas:atlas@localhost/atlas

# 7. Build all binaries
cargo build --release

# 8. Build explorer
cd apps/explorer && npm install && npm run build && cd ../..
```

## Environment variables (`.env`)
```bash
# Point at the Tachyon validator on Server A
VALIDATOR_RPC_URL=http://<SERVER_A_IP>:8899
YELLOWSTONE_GRPC_ENDPOINT=http://<SERVER_A_IP>:10000

# Local services on Server B
DATABASE_URL=postgres://atlas:atlas@localhost:5432/atlas
REDIS_URL=redis://127.0.0.1:6379

# Security — CHANGE THESE
ADMIN_API_KEY=<generate-a-strong-key>

# Explorer
ATLAS_API_URL=http://localhost:8080
ATLAS_API_KEY=<same-as-admin-or-a-dedicated-key>
```

## Starting services

```bash
# API
./target/release/atlas-api &

# Indexer (streams from validator)
./target/release/atlas-indexer &

# Webhooks
./target/release/atlas-webhooks &

# Intel
./target/release/atlas-intel &

# Explorer
cd apps/explorer && npm start &
```

## Systemd service files
Create `/etc/systemd/system/atlas-api.service`:
```ini
[Unit]
Description=Atlas API
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=root
WorkingDirectory=/root/atlas
EnvironmentFile=/root/atlas/.env
ExecStart=/root/atlas/target/release/atlas-api
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```
Repeat for `atlas-indexer`, `atlas-webhooks`, `atlas-intel`.

```bash
systemctl daemon-reload
systemctl enable atlas-api atlas-indexer atlas-webhooks atlas-intel
systemctl start  atlas-api atlas-indexer atlas-webhooks atlas-intel
```

## Health checks
```bash
# API
curl http://localhost:8080/health

# Postgres
psql postgres://atlas:atlas@localhost/atlas -c "SELECT COUNT(*) FROM tx_store;"

# Redis stream depth
redis-cli XLEN atlas:newtx

# Indexer checkpoint
redis-cli GET atlas:checkpoint:confirmed

# Explorer
curl http://localhost:3000
```

## Running migrations after an update
```bash
cd /root/atlas
git pull
sqlx migrate run --source infra/migrations --database-url $DATABASE_URL
cargo build --release
systemctl restart atlas-api atlas-indexer
```

## Geyser plugin (Server A only)
The Geyser plugin runs ON Server A alongside the validator.
Build and register it once:
```bash
cargo build --release -p atlas_geyser_v2
# Add to validator startup:
# --geyser-plugin-config /path/to/geyser_config.json
```
The plugin writes account updates directly to PostgreSQL on Server B via the `DATABASE_URL` in its config.
