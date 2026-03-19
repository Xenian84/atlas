---
name: atlas-api
description: Add, fix, or debug Atlas API endpoints. Use when working on crates/atlas_api — adding routes, handlers, middleware, RPC proxying, or API key auth.
---

# Atlas API Skill

## Stack
- **Framework**: Axum (Rust)
- **Entry point**: `crates/atlas_api/src/main.rs`
- **State**: `AppState` in `crates/atlas_api/src/state.rs` — holds `PgPool`, `redis::aio::ConnectionManager`, `reqwest::Client`, `AppConfig`
- **Auth**: `X-API-Key` header checked in middleware; `ADMIN_API_KEY` env var grants admin access
- **Port**: `8080` (configurable via `ATLAS_PORT`)

## Adding a new endpoint

1. Create `crates/atlas_api/src/handlers/<name>.rs`
2. Add `pub mod <name>;` to `crates/atlas_api/src/handlers/mod.rs`
3. Register the route in `crates/atlas_api/src/main.rs` inside the router builder
4. Use `extract::State(state): State<AppState>` to access DB/Redis

### Handler template
```rust
use axum::{extract::{Path, State}, Json};
use crate::{state::AppState, error::ApiError};
use serde::Serialize;

#[derive(Serialize)]
pub struct MyResponse { /* fields */ }

pub async fn get_my_thing(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MyResponse>, ApiError> {
    let row = sqlx::query_as!(MyRow, "SELECT ... WHERE id = $1", id)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(MyResponse { /* ... */ }))
}
```

## Key files
```
crates/atlas_api/src/
├── main.rs          Router setup, AppState construction, startup
├── state.rs         AppState struct
├── config.rs        AppConfig (reads env vars)
├── error.rs         ApiError — maps DB/RPC errors to HTTP status
├── negotiate.rs     Content-type negotiation (JSON vs TOON)
├── rpc.rs           Proxy to validator JSON-RPC
└── handlers/
    ├── mod.rs       Route registration
    ├── address.rs   /v1/address/:addr — tx history, cursor pagination
    ├── tx.rs        /v1/tx/:sig — enhanced tx facts
    ├── intel.rs     /v1/intel/:addr — wallet scores + profile
    ├── webhooks.rs  /v1/webhooks — subscription CRUD
    ├── keys.rs      /v1/keys — API key management
    ├── network.rs   /v1/network/pulse — live network stats
    └── trace.rs     /v1/trace/:addr — wallet counterparty graph
```

## Environment variables
```
VALIDATOR_RPC_URL       http://127.0.0.1:8899
DATABASE_URL            postgres://atlas:atlas@localhost:5432/atlas
REDIS_URL               redis://127.0.0.1:6379
ADMIN_API_KEY           atlas-dev-key-change-me
ATLAS_PORT              8080
```

## Database schema (key tables)
- `tx_store` — indexed transactions (sig, slot, block_time, status, fee, tags, actions_json, programs)
- `address_index` — (address, slot, pos, sig) — cursor-paginated tx lookup
- `wallet_scores` — intel scores per address
- `api_keys` — (id, key_hash, key_prefix, name, tier, owner_email, last_used_at)
- `webhook_subscriptions` — (id, owner_key, url, event_types, filters)
- `webhook_deliveries` — outbound delivery log

## Common debugging
```bash
# Check API is running
curl http://localhost:8080/health

# Test with admin key
curl -H "X-API-Key: atlas-dev-key-change-me" http://localhost:8080/v1/network/pulse

# Create API key
curl -X POST -H "X-API-Key: atlas-dev-key-change-me" \
  -H "Content-Type: application/json" \
  -d '{"name":"test","tier":"free"}' \
  http://localhost:8080/v1/keys
```
