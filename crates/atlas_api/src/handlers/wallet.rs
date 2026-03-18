/// Wallet API — identity, balances, history, transfers, funded-by.
///
/// GET  /v1/wallet/:addr/identity
/// POST /v1/wallet/batch-identity
/// GET  /v1/wallet/:addr/balances
/// GET  /v1/wallet/:addr/history
/// GET  /v1/wallet/:addr/transfers
/// GET  /v1/wallet/:addr/funded-by
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tracing::warn;
use crate::{error::ApiError, state::AppState};

// ── GET /v1/wallet/:addr/identity ────────────────────────────────────────────

pub async fn get_identity(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query(
        "SELECT address, name, category, entity_type, verified, url, notes
         FROM entity_labels WHERE address = $1"
    )
    .bind(&addr)
    .fetch_optional(state.pool())
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("no identity for {}", addr)))?;

    Ok(Json(row_to_identity(&row)))
}

// ── POST /v1/wallet/batch-identity ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct BatchIdentityRequest {
    pub addresses: Vec<String>,
}

pub async fn batch_identity(
    State(state): State<AppState>,
    Json(req):    Json<BatchIdentityRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.addresses.is_empty() {
        return Ok(Json(json!([])));
    }
    let addrs: Vec<&str> = req.addresses.iter().map(String::as_str).take(100).collect();

    let rows = sqlx::query(
        "SELECT address, name, category, entity_type, verified, url, notes
         FROM entity_labels WHERE address = ANY($1)"
    )
    .bind(&addrs as &[&str])
    .fetch_all(state.pool())
    .await?;

    let items: Vec<Value> = rows.iter().map(|r| row_to_identity(r)).collect();
    Ok(Json(json!(items)))
}

fn row_to_identity(row: &sqlx::postgres::PgRow) -> Value {
    json!({
        "address":     row.try_get::<String, _>("address").unwrap_or_default(),
        "name":        row.try_get::<String, _>("name").unwrap_or_default(),
        "category":    row.try_get::<String, _>("category").unwrap_or_default(),
        "type":        row.try_get::<String, _>("entity_type").unwrap_or_default(),
        "verified":    row.try_get::<bool, _>("verified").unwrap_or(false),
        "url":         row.try_get::<Option<String>, _>("url").ok().flatten(),
        "notes":       row.try_get::<Option<String>, _>("notes").ok().flatten(),
    })
}

// ── GET /v1/wallet/:addr/balances ─────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct BalancesQuery {
    #[serde(default = "default_true")]
    pub show_native:      bool,
    #[serde(default)]
    pub show_zero_balance: bool,
    #[serde(default)]
    pub show_nfts:        bool,
    #[serde(default = "default_100")]
    pub limit:            u32,
    #[serde(default = "default_1")]
    pub page:             u32,
}
fn default_true() -> bool { true }
fn default_100()  -> u32  { 100 }
fn default_1()    -> u32  { 1 }

pub async fn get_balances(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
    Query(q):     Query<BalancesQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit  = (q.limit.min(100)) as i64;
    let offset = ((q.page.saturating_sub(1)) as i64) * limit;

    // Fetch XNT balance from validator RPC
    let xnt_balance_lamports = if q.show_native {
        fetch_sol_balance(state.http(), &state.cfg().validator_rpc_url, &addr).await
    } else {
        0u64
    };

    // Fetch token accounts from our index
    let type_filter = if q.show_nfts { "" } else { "AND a.asset_type != 'nft'" };
    let zero_filter = if q.show_zero_balance { "" } else { "AND t.amount > 0" };

    let query = format!(
        r#"SELECT t.token_account, t.mint, t.amount::text, t.decimals,
                  a.name, a.symbol, a.image_uri, a.asset_type
           FROM token_account_index t
           LEFT JOIN asset_index a ON a.mint = t.mint
           WHERE t.owner = $1 {} {}
           ORDER BY t.amount DESC
           LIMIT $2 OFFSET $3"#,
        type_filter, zero_filter
    );

    let rows = sqlx::query(&query)
        .bind(&addr)
        .bind(limit)
        .bind(offset)
        .fetch_all(state.pool())
        .await?;

    // Fetch USD prices from configured price oracle for all mints in one batch call
    let mints: Vec<String> = rows.iter()
        .filter_map(|r| r.try_get::<String, _>("mint").ok())
        .collect();
    let prices = fetch_token_prices(state.http(), &state.cfg().price_api_url, &mints).await;

    let mut total_usd = 0.0f64;
    let balances: Vec<Value> = rows.iter().map(|r| {
        let amount_str: String = r.try_get("amount").unwrap_or_default();
        let decimals: i16      = r.try_get("decimals").unwrap_or(0);
        let mint: String       = r.try_get("mint").unwrap_or_default();
        let amount: f64        = amount_str.parse::<f64>().unwrap_or(0.0)
                                  / 10f64.powi(decimals as i32);
        let price_per_token    = prices.get(&mint).copied();
        let usd_value          = price_per_token.map(|p| p * amount);
        if let Some(usd) = usd_value { total_usd += usd; }

        json!({
            "token_account":  r.try_get::<String, _>("token_account").unwrap_or_default(),
            "mint":           mint,
            "name":           r.try_get::<Option<String>, _>("name").ok().flatten(),
            "symbol":         r.try_get::<Option<String>, _>("symbol").ok().flatten(),
            "image":          r.try_get::<Option<String>, _>("image_uri").ok().flatten(),
            "asset_type":     r.try_get::<String, _>("asset_type").unwrap_or("unknown".into()),
            "amount":         amount,
            "raw_amount":     amount_str,
            "decimals":       decimals,
            "price_per_token": price_per_token,
            "usd_value":      usd_value,
        })
    }).collect();

    let xnt_balance = xnt_balance_lamports as f64 / 1_000_000_000.0;

    Ok(Json(json!({
        "address": addr,
        "native": if q.show_native { json!({
            "symbol":   "XNT",
            "balance":  xnt_balance,
            "lamports": xnt_balance_lamports,
            "usd_value": null,
        }) } else { Value::Null },
        "tokens":          balances,
        "total_usd_value": total_usd,
        "page":            q.page,
        "limit":           limit,
    })))
}

// ── GET /v1/wallet/:addr/history ──────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct HistoryQuery {
    #[serde(default = "default_50")]
    pub limit:  u32,
    pub before: Option<String>,
    pub r#type: Option<String>,
}
fn default_50() -> u32 { 50 }

pub async fn get_history(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
    Query(q):     Query<HistoryQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.min(100) as i64;

    // Parse cursor
    let (before_slot, before_pos) = match q.before.as_deref() {
        Some(c) => {
            let parts: Vec<&str> = c.split(':').collect();
            if parts.len() == 2 {
                let slot = parts[0].parse::<i64>().unwrap_or(i64::MAX);
                let pos  = parts[1].parse::<i32>().unwrap_or(i32::MAX);
                (Some(slot), Some(pos))
            } else { (None, None) }
        }
        None => (None, None),
    };

    let rows = match q.r#type.as_deref().filter(|t| *t != "all") {
        Some(tx_type) => sqlx::query(
            r#"SELECT ai.sig, ai.slot, ai.pos, ai.block_time, ai.status, ai.tags, ai.action_types,
                      t.fee_lamports, t.compute_consumed, t.sol_deltas_json, t.token_deltas_json,
                      t.actions_json
               FROM address_index ai
               JOIN tx_store t ON t.sig = ai.sig
               WHERE ai.address = $1
                 AND ($2::bigint IS NULL OR (ai.slot, ai.pos) < ($2, $3::int))
                 AND $4 = ANY(ai.action_types)
               ORDER BY ai.slot DESC, ai.pos DESC
               LIMIT $5"#
        )
        .bind(&addr).bind(before_slot).bind(before_pos).bind(tx_type).bind(limit)
        .fetch_all(state.pool()).await?,

        None => sqlx::query(
            r#"SELECT ai.sig, ai.slot, ai.pos, ai.block_time, ai.status, ai.tags, ai.action_types,
                      t.fee_lamports, t.compute_consumed, t.sol_deltas_json, t.token_deltas_json,
                      t.actions_json
               FROM address_index ai
               JOIN tx_store t ON t.sig = ai.sig
               WHERE ai.address = $1
                 AND ($2::bigint IS NULL OR (ai.slot, ai.pos) < ($2, $3::int))
               ORDER BY ai.slot DESC, ai.pos DESC
               LIMIT $4"#
        )
        .bind(&addr).bind(before_slot).bind(before_pos).bind(limit)
        .fetch_all(state.pool()).await?,
    };

    let items: Vec<Value> = rows.iter().map(|r| {
        let slot: i64     = r.try_get("slot").unwrap_or(0);
        let pos: i32      = r.try_get("pos").unwrap_or(0);
        let sol_deltas: Value = r.try_get("sol_deltas_json").unwrap_or(json!([]));
        let tok_deltas: Value = r.try_get("token_deltas_json").unwrap_or(json!([]));

        // Summarize balance changes for this address
        let sol_change = sol_deltas.as_array()
            .and_then(|a| a.iter().find(|d| d["address"].as_str() == Some(&addr)))
            .and_then(|d| d["delta"].as_i64());

        json!({
            "signature":       r.try_get::<String, _>("sig").unwrap_or_default(),
            "slot":            slot,
            "pos":             pos,
            "block_time":      r.try_get::<Option<i64>, _>("block_time").ok().flatten(),
            "status":          r.try_get::<i16, _>("status").map(|s| if s == 1 { "success" } else { "failed" }).unwrap_or("unknown"),
            "tags":            r.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
            "action_types":    r.try_get::<Vec<String>, _>("action_types").unwrap_or_default(),
            "fee_lamports":    r.try_get::<i64, _>("fee_lamports").unwrap_or(0),
            "compute_consumed":r.try_get::<Option<i32>, _>("compute_consumed").ok().flatten(),
            "sol_change_lamports": sol_change,
            "token_changes":   tok_deltas.as_array()
                .map(|a| a.iter()
                    .filter(|d| d["owner"].as_str() == Some(&addr))
                    .cloned().collect::<Vec<_>>())
                .unwrap_or_default(),
            "cursor": format!("{}:{}", slot, pos),
        })
    }).collect();

    let next_cursor = if items.len() == limit as usize {
        items.last().and_then(|i| i["cursor"].as_str().map(String::from))
    } else { None };

    Ok(Json(json!({
        "address":     addr,
        "data":        items,
        "pagination":  { "next": next_cursor },
    })))
}

// ── GET /v1/wallet/:addr/transfers ────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct TransfersQuery {
    #[serde(default = "default_50")]
    pub limit:  u32,
    pub cursor: Option<String>,
}

pub async fn get_transfers(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
    Query(q):     Query<TransfersQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.min(100) as i64;

    let (before_slot, before_pos) = parse_cursor(q.cursor.as_deref());

    // Pull token transfer rows from token_balance_index
    let rows = sqlx::query(
        r#"SELECT tbi.sig, tbi.slot, tbi.pos, tbi.mint, tbi.delta::text, tbi.direction,
                  ai.name, ai.symbol, ai.decimals
           FROM token_balance_index tbi
           LEFT JOIN asset_index ai ON ai.mint = tbi.mint
           WHERE tbi.owner = $1
             AND ($2::bigint IS NULL OR (tbi.slot, tbi.pos) < ($2, $3::int))
           ORDER BY tbi.slot DESC, tbi.pos DESC
           LIMIT $4"#
    )
    .bind(&addr)
    .bind(before_slot)
    .bind(before_pos)
    .bind(limit)
    .fetch_all(state.pool())
    .await?;

    let sigs: Vec<String> = rows.iter()
        .filter_map(|r| r.try_get::<String, _>("sig").ok())
        .collect::<std::collections::HashSet<_>>()
        .into_iter().collect();

    // Fetch counterparty (the other side of the transfer) from tx actions
    let tx_rows = if !sigs.is_empty() {
        sqlx::query("SELECT sig, actions_json, block_time FROM tx_store WHERE sig = ANY($1)")
            .bind(&sigs as &[String])
            .fetch_all(state.pool())
            .await
            .unwrap_or_default()
    } else { vec![] };

    let tx_map: std::collections::HashMap<String, (Value, Option<i64>)> = tx_rows.iter()
        .filter_map(|r| {
            let sig: String = r.try_get("sig").ok()?;
            let actions: Value = r.try_get("actions_json").ok()?;
            let bt: Option<i64> = r.try_get("block_time").ok().flatten();
            Some((sig, (actions, bt)))
        }).collect();

    let items: Vec<Value> = rows.iter().map(|r| {
        let sig: String  = r.try_get("sig").unwrap_or_default();
        let slot: i64    = r.try_get("slot").unwrap_or(0);
        let pos: i32     = r.try_get("pos").unwrap_or(0);
        let mint: String = r.try_get("mint").unwrap_or_default();
        let delta: String = r.try_get("delta").unwrap_or_default();
        let direction: i16 = r.try_get("direction").unwrap_or(0);
        let decimals: i16 = r.try_get("decimals").unwrap_or(0);
        let symbol: Option<String> = r.try_get("symbol").ok().flatten();

        let amount = delta.parse::<f64>().unwrap_or(0.0).abs()
                     / 10f64.powi(decimals as i32);

        // Find counterparty from actions
        let (counterparty, block_time) = tx_map.get(&sig)
            .map(|(actions, bt)| {
                let cp = actions.as_array()
                    .and_then(|a| a.iter().find(|act| {
                        act["t"].as_str() == Some("TRANSFER") &&
                        (act["from"].as_str() == Some(&addr) || act["to"].as_str() == Some(&addr))
                    }))
                    .map(|act| {
                        if act["from"].as_str() == Some(&addr) {
                            act["to"].as_str().map(String::from)
                        } else {
                            act["from"].as_str().map(String::from)
                        }
                    })
                    .flatten();
                (cp, *bt)
            })
            .unwrap_or((None, None));

        // Resolve counterparty name
        json!({
            "signature":    sig,
            "slot":         slot,
            "block_time":   block_time,
            "direction":    if direction == 1 { "in" } else { "out" },
            "mint":         mint,
            "symbol":       symbol,
            "amount":       amount,
            "raw_delta":    delta,
            "counterparty": counterparty,
            "cursor":       format!("{}:{}", slot, pos),
        })
    }).collect();

    let next_cursor = if items.len() == limit as usize {
        items.last().and_then(|i| i["cursor"].as_str().map(String::from))
    } else { None };

    Ok(Json(json!({
        "address":    addr,
        "data":       items,
        "pagination": { "next": next_cursor },
    })))
}

// ── GET /v1/wallet/:addr/funded-by ────────────────────────────────────────────

pub async fn get_funded_by(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
) -> Result<Json<Value>, ApiError> {
    // Find the earliest incoming XNT transfer to this address
    let row = sqlx::query(
        r#"SELECT ai.sig, ai.slot, ai.block_time,
                  t.sol_deltas_json, t.actions_json
           FROM address_index ai
           JOIN tx_store t ON t.sig = ai.sig
           WHERE ai.address = $1
           ORDER BY ai.slot ASC, ai.pos ASC
           LIMIT 1"#
    )
    .bind(&addr)
    .fetch_optional(state.pool())
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("no transactions found for {}", addr)))?;

    let sig:       String       = row.try_get("sig").unwrap_or_default();
    let slot:      i64          = row.try_get("slot").unwrap_or(0);
    let block_time: Option<i64> = row.try_get("block_time").ok().flatten();
    let actions:   Value        = row.try_get("actions_json").unwrap_or(json!([]));
    let sol_deltas: Value       = row.try_get("sol_deltas_json").unwrap_or(json!([]));

    // Find the sender — the "from" in the first XNT transfer to us
    let funder = actions.as_array()
        .and_then(|a| a.iter().find(|act| {
            act["t"].as_str() == Some("TRANSFER") && act["to"].as_str() == Some(&addr)
        }))
        .and_then(|act| act["from"].as_str().map(String::from));

    // Amount received
    let amount_lamports = sol_deltas.as_array()
        .and_then(|a| a.iter().find(|d| d["address"].as_str() == Some(&addr)))
        .and_then(|d| d["delta"].as_i64())
        .unwrap_or(0);

    let funder_addr = funder.as_deref().unwrap_or("");

    // Look up funder identity
    let funder_identity = if !funder_addr.is_empty() {
        sqlx::query("SELECT name, category, entity_type FROM entity_labels WHERE address = $1")
            .bind(funder_addr)
            .fetch_optional(state.pool())
            .await
            .ok()
            .flatten()
            .map(|r| json!({
                "name":     r.try_get::<String, _>("name").unwrap_or_default(),
                "category": r.try_get::<String, _>("category").unwrap_or_default(),
                "type":     r.try_get::<String, _>("entity_type").unwrap_or_default(),
            }))
    } else { None };

    if funder.is_none() {
        return Err(ApiError::NotFound("could not determine funding source".into()));
    }

    Ok(Json(json!({
        "address":          addr,
        "funder":           funder,
        "funder_identity":  funder_identity,
        "amount_lamports":  amount_lamports,
        "amount_xnt":       amount_lamports as f64 / 1_000_000_000.0,
        "slot":             slot,
        "block_time":       block_time,
        "signature":        sig,
    })))
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn parse_cursor(c: Option<&str>) -> (Option<i64>, Option<i32>) {
    match c {
        Some(s) => {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() == 2 {
                (parts[0].parse().ok(), parts[1].parse().ok())
            } else { (None, None) }
        }
        None => (None, None),
    }
}

/// Fetch USD prices for a batch of mints from the XDex token price oracle.
///
/// XDex API (per-token, concurrent):
///   GET {base_url}?network=X1+Mainnet&token_address={MINT}
///   → {"success":true,"data":{"price":1.23,"price_currency":"USD"}}
///
/// Up to 20 mints are queried concurrently. Set ATLAS_PRICE_API_URL to override the endpoint.
/// Returns a map of mint → price_in_usd. Silently returns empty map on any failure.
async fn fetch_token_prices(
    http:     &reqwest::Client,
    base_url: &str,
    mints:    &[String],
) -> std::collections::HashMap<String, f64> {
    if mints.is_empty() || base_url.is_empty() {
        return std::collections::HashMap::new();
    }

    let base = base_url.trim_end_matches('/');
    // Deduplicate and cap at 20 concurrent requests
    let unique_mints: Vec<&String> = {
        let mut seen = std::collections::HashSet::new();
        mints.iter().filter(|m| seen.insert(m.as_str())).take(20).collect()
    };

    let futures: Vec<_> = unique_mints.iter().map(|mint| {
        let url = format!("{}?network=X1+Mainnet&token_address={}", base, mint);
        let client = http.clone();
        let mint = mint.to_string();
        async move {
            let resp = client
                .get(&url)
                .timeout(std::time::Duration::from_secs(3))
                .send()
                .await
                .ok()?;
            let json: Value = resp.json().await.ok()?;
            if json["success"].as_bool() != Some(true) {
                return None;
            }
            let price = json["data"]["price"].as_f64()?;
            Some((mint, price))
        }
    }).collect();

    let results = futures::future::join_all(futures).await;
    results.into_iter().flatten().collect()
}

async fn fetch_sol_balance(http: &reqwest::Client, rpc_url: &str, addr: &str) -> u64 {
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getBalance",
        "params": [addr, {"commitment": "confirmed"}]
    });
    match http.post(rpc_url).json(&body).send().await {
        Ok(r) => r.json::<Value>().await
            .ok()
            .and_then(|v| v["result"]["value"].as_u64())
            .unwrap_or(0),
        Err(e) => {
            warn!("getBalance failed for {}: {}", addr, e);
            0
        }
    }
}
