use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

use crate::{error::ApiError, state::AppState};

#[derive(Deserialize)]
pub struct TraceQuery {
    /// How many hops to expand (default 1, max 2)
    pub hops: Option<u8>,
    /// Unix timestamp lower bound
    pub from_ts: Option<i64>,
    /// Unix timestamp upper bound
    pub to_ts: Option<i64>,
    /// Minimum transfer amount in lamports
    pub min_amount: Option<i64>,
    /// Maximum transfer amount in lamports
    pub max_amount: Option<i64>,
    /// Filter to specific mint (omit = all tokens + SOL)
    pub mint: Option<String>,
    /// Exclude zero-value dust transfers
    pub hide_dust: Option<bool>,
    /// Max counterparties to return (default 50)
    pub limit: Option<i64>,
}

#[derive(Serialize, Clone)]
pub struct WalletNodeData {
    pub address: String,
    pub sol_balance: f64,
    pub token_count: i64,
    pub tx_count: i64,
    pub move_count: i64,
    pub first_seen: Option<i64>,
    pub last_seen: Option<i64>,
    pub labels: Vec<String>,
}

#[derive(Serialize)]
pub struct TraceEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub tx_count: i64,
    pub total_lamports: i64,
    pub mints: Vec<String>,
    pub first_ts: Option<i64>,
    pub last_ts: Option<i64>,
    pub direction: String, // "out" | "in" | "both"
}

#[derive(Serialize)]
pub struct CounterpartyRow {
    pub address: String,
    pub label: Option<String>,
    pub direction: String,
    pub tx_count: i64,
    pub total_lamports: i64,
    pub mint: Option<String>,
    pub first_ts: Option<i64>,
    pub last_ts: Option<i64>,
}

#[derive(Serialize)]
pub struct TraceResponse {
    pub root: String,
    pub nodes: Vec<WalletNodeData>,
    pub edges: Vec<TraceEdge>,
    pub transfers: Vec<CounterpartyRow>,
    pub total_outflow_lamports: i64,
    pub total_inflow_lamports: i64,
    pub total_transfers: i64,
    pub cps: i64, // unique counterparties
}

/// GET /v1/trace/:address
/// Returns the full counterparty graph for a wallet address.
pub async fn get_trace(
    Path(address): Path<String>,
    Query(q): Query<TraceQuery>,
    State(state): State<AppState>,
) -> Result<Json<TraceResponse>, ApiError> {
    let limit = q.limit.unwrap_or(50).min(200);
    let hide_dust = q.hide_dust.unwrap_or(false);
    let min_amount = if hide_dust { q.min_amount.unwrap_or(1_000) } else { q.min_amount.unwrap_or(0) };
    let max_amount = q.max_amount;

    // ── Counterparty aggregation from address_index ───────────────────────────
    // address_index has: address, sig, slot, block_time, is_signer, direction
    // We join with tx_store to get sol_delta and fee info.
    let mut where_clauses = vec!["ai.address = $1".to_string()];
    let mut param_idx = 2i32;

    if let Some(from) = q.from_ts {
        where_clauses.push(format!("ai.block_time >= ${}", param_idx));
        param_idx += 1;
        let _ = from;
    }
    if let Some(to) = q.to_ts {
        where_clauses.push(format!("ai.block_time <= ${}", param_idx));
        param_idx += 1;
        let _ = to;
    }

    // Build counterparty list: for each tx the root address appears in,
    // find the other top-level account (counterparty).
    // We use a simplified version: group address_index by sig, pick peer addresses.
    let counterparties: Vec<sqlx::postgres::PgRow> = sqlx::query(
        r#"
        SELECT
            peer.address                            AS cp_address,
            COUNT(DISTINCT ai.sig)::BIGINT          AS tx_count,
            SUM(ABS(ts.sol_delta))::BIGINT          AS total_lamports,
            MIN(ai.block_time)                      AS first_ts,
            MAX(ai.block_time)                      AS last_ts,
            -- Majority direction: net flow from root perspective
            CASE
              WHEN SUM(CASE WHEN ai.direction = 'out' THEN 1 ELSE -1 END) > 0
              THEN 'out'
              WHEN SUM(CASE WHEN ai.direction = 'in'  THEN 1 ELSE -1 END) > 0
              THEN 'in'
              ELSE 'both'
            END                                     AS direction
        FROM address_index ai
        JOIN address_index peer ON peer.sig = ai.sig AND peer.address != $1
        LEFT JOIN tx_store ts ON ts.sig = ai.sig AND ts.commitment = 'confirmed'
        WHERE ai.address = $1
          AND ($2::BIGINT IS NULL OR ai.block_time >= $2)
          AND ($3::BIGINT IS NULL OR ai.block_time <= $3)
          AND ($4::BIGINT IS NULL OR ABS(ts.sol_delta) >= $4)
          AND ($5::BIGINT IS NULL OR ABS(ts.sol_delta) <= $5)
        GROUP BY peer.address
        ORDER BY tx_count DESC
        LIMIT $6
        "#
    )
    .bind(&address)
    .bind(q.from_ts)
    .bind(q.to_ts)
    .bind(if min_amount > 0 { Some(min_amount) } else { None })
    .bind(max_amount)
    .bind(limit)
    .fetch_all(state.pool())
    .await
    .unwrap_or_default();

    // ── Root node stats ───────────────────────────────────────────────────────
    let root_stats = sqlx::query(
        r#"
        SELECT
            COUNT(DISTINCT sig)::BIGINT AS tx_count,
            MIN(block_time)             AS first_ts,
            MAX(block_time)             AS last_ts
        FROM address_index
        WHERE address = $1
        "#
    )
    .bind(&address)
    .fetch_optional(state.pool())
    .await
    .unwrap_or(None);

    let root_tx_count: i64 = root_stats.as_ref()
        .and_then(|r| r.try_get("tx_count").ok())
        .unwrap_or(0);
    let root_first_ts: Option<i64> = root_stats.as_ref()
        .and_then(|r| r.try_get("first_ts").ok())
        .flatten();
    let root_last_ts: Option<i64> = root_stats.as_ref()
        .and_then(|r| r.try_get("last_ts").ok())
        .flatten();

    // ── SOL balance from RPC (via validator proxy) ────────────────────────────
    let sol_balance = fetch_sol_balance(&state, &address).await;

    // ── Build response ────────────────────────────────────────────────────────
    let mut nodes: Vec<WalletNodeData> = vec![WalletNodeData {
        address: address.clone(),
        sol_balance,
        token_count: 0,
        tx_count: root_tx_count,
        move_count: root_tx_count,
        first_seen: root_first_ts,
        last_seen: root_last_ts,
        labels: vec![],
    }];

    let mut edges: Vec<TraceEdge> = vec![];
    let mut transfers: Vec<CounterpartyRow> = vec![];
    let mut total_out: i64 = 0;
    let mut total_in: i64 = 0;

    // ── Entity labels ─────────────────────────────────────────────────────────
    let cp_addresses: Vec<String> = counterparties
        .iter()
        .filter_map(|r| r.try_get::<String, _>("cp_address").ok())
        .collect();

    let label_map = fetch_labels(state.pool(), &cp_addresses).await;

    for (i, row) in counterparties.iter().enumerate() {
        let cp_addr: String = match row.try_get("cp_address") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let tx_count: i64 = row.try_get("tx_count").unwrap_or(0);
        let total_lamports: i64 = row.try_get::<Option<i64>, _>("total_lamports")
            .unwrap_or(None).unwrap_or(0);
        let first_ts: Option<i64> = row.try_get("first_ts").unwrap_or(None);
        let last_ts: Option<i64>  = row.try_get("last_ts").unwrap_or(None);
        let direction: String = row.try_get("direction").unwrap_or_else(|_| "both".to_string());

        if direction == "out" { total_out += total_lamports; }
        if direction == "in"  { total_in  += total_lamports; }

        let label = label_map.get(&cp_addr).cloned();

        nodes.push(WalletNodeData {
            address: cp_addr.clone(),
            sol_balance: 0.0,
            token_count: 0,
            tx_count,
            move_count: tx_count,
            first_seen: first_ts,
            last_seen: last_ts,
            labels: label.iter().cloned().collect(),
        });

        let (src, tgt) = match direction.as_str() {
            "in"  => (cp_addr.clone(), address.clone()),
            _     => (address.clone(), cp_addr.clone()),
        };

        edges.push(TraceEdge {
            id: format!("e{}", i),
            source: src,
            target: tgt,
            tx_count,
            total_lamports,
            mints: vec![],
            first_ts,
            last_ts,
            direction: direction.clone(),
        });

        transfers.push(CounterpartyRow {
            address: cp_addr,
            label,
            direction,
            tx_count,
            total_lamports,
            mint: None,
            first_ts,
            last_ts,
        });
    }

    let cps = counterparties.len() as i64;
    let total_transfers = transfers.iter().map(|t| t.tx_count).sum();

    Ok(Json(TraceResponse {
        root: address,
        nodes,
        edges,
        transfers,
        total_outflow_lamports: total_out,
        total_inflow_lamports: total_in,
        total_transfers,
        cps,
    }))
}

async fn fetch_sol_balance(state: &AppState, address: &str) -> f64 {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getBalance",
        "params": [address]
    });
    let resp = client
        .post(&state.cfg().validator_rpc_url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await;

    if let Ok(r) = resp {
        if let Ok(j) = r.json::<serde_json::Value>().await {
            if let Some(lamports) = j["result"]["value"].as_i64() {
                return lamports as f64 / 1_000_000_000.0;
            }
        }
    }
    0.0
}

async fn fetch_labels(pool: &sqlx::PgPool, addresses: &[String]) -> HashMap<String, String> {
    if addresses.is_empty() { return HashMap::new(); }
    let rows = sqlx::query(
        "SELECT address, name FROM entity_labels WHERE address = ANY($1) LIMIT 500"
    )
    .bind(addresses)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter().filter_map(|r| {
        let addr: String = r.try_get("address").ok()?;
        let name: String = r.try_get("name").ok()?;
        Some((addr, name))
    }).collect()
}
