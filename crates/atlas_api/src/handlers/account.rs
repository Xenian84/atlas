//! GET /v1/wallet/:addr — unified wallet overview.
//!
//! Returns in a single response:
//!   - XNT balance (accounts table, kept live by gRPC post_balance extraction;
//!                  RPC fallback only for wallets completely absent from index)
//!   - Transaction count + first/last seen (from address_index)
//!   - Token holdings with metadata (from token_account_index + token_metadata)
//!   - Identity label (from entity_labels)
//!   - Intelligence profile summary (from intelligence_wallet_profiles)

use axum::{extract::{Path, State}, Json};
use serde_json::{json, Value};
use sqlx::Row;
use crate::{error::ApiError, state::AppState};

pub async fn get_wallet(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
) -> Result<Json<Value>, ApiError> {
    let pool = state.pool();

    // ── XNT balance ───────────────────────────────────────────────────────────
    // Primary: accounts table — kept perpetually current by the confirmed gRPC
    // stream extracting post_balances from every confirmed transaction.
    // Fallback: RPC — only for wallets that have never transacted and are
    // therefore absent from our index entirely.
    let account_row = sqlx::query(
        "SELECT lamports, owner, executable, space, updated_slot FROM accounts WHERE address = $1"
    )
    .bind(&addr)
    .fetch_optional(pool)
    .await?;

    let (lamports, owner, executable, space, updated_slot) = match account_row {
        Some(r) => (
            r.try_get::<i64, _>("lamports").unwrap_or(0),
            r.try_get::<String, _>("owner")
              .unwrap_or_else(|_| "11111111111111111111111111111111".into()),
            r.try_get::<bool, _>("executable").unwrap_or(false),
            r.try_get::<i64, _>("space").unwrap_or(0),
            r.try_get::<i64, _>("updated_slot").unwrap_or(0),
        ),
        // Wallet not in our index — RPC fallback for live balance.
        None => {
            let rpc_url = state.cfg().validator_rpc_url.clone();
            match fetch_rpc_balance(state.http(), &rpc_url, &addr).await {
                Ok((lam, own, exe, sp)) => (lam, own, exe, sp, 0),
                Err(_) => (0, "11111111111111111111111111111111".into(), false, 0, 0),
            }
        }
    };

    let xnt = lamports as f64 / 1_000_000_000.0;

    // ── Transaction history summary ───────────────────────────────────────────
    let hist = sqlx::query(
        r#"SELECT COUNT(*) AS tx_count,
                  MIN(block_time) AS first_seen,
                  MAX(block_time) AS last_seen
           FROM address_index WHERE address = $1"#
    )
    .bind(&addr)
    .fetch_optional(pool)
    .await?;

    let (tx_count, first_seen, last_seen) = match hist {
        Some(r) => (
            r.try_get::<i64, _>("tx_count").unwrap_or(0),
            r.try_get::<Option<i64>, _>("first_seen").unwrap_or(None),
            r.try_get::<Option<i64>, _>("last_seen").unwrap_or(None),
        ),
        None => (0, None, None),
    };

    // ── Token holdings (top 20 by amount) ────────────────────────────────────
    let token_rows = sqlx::query(
        r#"SELECT tai.mint, tai.amount, tai.token_account,
                  tm.name, tm.symbol, tm.decimals, tm.logo_uri
           FROM token_account_index tai
           LEFT JOIN token_metadata tm ON tm.mint = tai.mint
           WHERE tai.owner = $1
             AND tai.amount > 0
           ORDER BY tai.amount DESC
           LIMIT 20"#
    )
    .bind(&addr)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let tokens: Vec<Value> = token_rows.iter().map(|r| {
        let mint    = r.try_get::<String, _>("mint").unwrap_or_default();
        let amount  = r.try_get::<sqlx::types::Decimal, _>("amount")
            .map(|d| d.to_string()).unwrap_or_else(|_| "0".into());
        let decimals = r.try_get::<i16, _>("decimals").unwrap_or(0) as u8;
        let name    = r.try_get::<Option<String>, _>("name").ok().flatten().unwrap_or_default();
        let symbol  = r.try_get::<Option<String>, _>("symbol").ok().flatten().unwrap_or_default();
        let logo    = r.try_get::<Option<String>, _>("logo_uri").ok().flatten();
        let tok_acc = r.try_get::<String, _>("token_account").unwrap_or_default();
        json!({
            "mint":          mint,
            "token_account": tok_acc,
            "amount":        amount,
            "decimals":      decimals,
            "name":          name,
            "symbol":        symbol,
            "logo_uri":      logo,
        })
    }).collect();

    // ── Identity label ────────────────────────────────────────────────────────
    let identity = sqlx::query(
        "SELECT name, category, entity_type, verified FROM entity_labels WHERE address = $1"
    )
    .bind(&addr)
    .fetch_optional(pool)
    .await?
    .map(|r| json!({
        "name":        r.try_get::<String, _>("name").unwrap_or_default(),
        "category":    r.try_get::<String, _>("category").unwrap_or_default(),
        "entity_type": r.try_get::<String, _>("entity_type").unwrap_or_default(),
        "verified":    r.try_get::<bool, _>("verified").unwrap_or(false),
    }));

    // ── Intelligence profile (7d window) ─────────────────────────────────────
    let profile = sqlx::query(
        r#"SELECT wallet_type, confidence, automation_score, sniper_score,
                  whale_score, risk_score
           FROM intelligence_wallet_profiles
           WHERE address = $1 AND "window" = '7d'"#
    )
    .bind(&addr)
    .fetch_optional(pool)
    .await?
    .map(|r| json!({
        "wallet_type":      r.try_get::<String, _>("wallet_type").unwrap_or_default(),
        "confidence":       r.try_get::<sqlx::types::Decimal, _>("confidence").map(|d| d.to_string()).unwrap_or_default(),
        "automation_score": r.try_get::<i32, _>("automation_score").unwrap_or(0),
        "sniper_score":     r.try_get::<i32, _>("sniper_score").unwrap_or(0),
        "whale_score":      r.try_get::<i32, _>("whale_score").unwrap_or(0),
        "risk_score":       r.try_get::<i32, _>("risk_score").unwrap_or(0),
    }));

    Ok(Json(json!({
        "address":      addr,
        "balance": {
            "xnt":      xnt,
            "lamports": lamports,
        },
        "account": {
            "owner":       owner,
            "executable":  executable,
            "space":       space,
            "updated_slot": updated_slot,
        },
        "tx_count":   tx_count,
        "first_seen": first_seen,
        "last_seen":  last_seen,
        "tokens":     tokens,
        "identity":   identity,
        "profile":    profile,
    })))
}

/// Fetch live balance + account info from validator RPC.
/// Used as fallback for wallets not yet in the accounts index.
async fn fetch_rpc_balance(
    client: &reqwest::Client,
    rpc_url: &str,
    addr: &str,
) -> anyhow::Result<(i64, String, bool, i64)> {
    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getAccountInfo",
            "params": [addr, {"encoding": "base64", "commitment": "confirmed"}]
        }))
        .send().await?
        .json().await?;

    let value = resp["result"]["value"].as_object();
    match value {
        Some(v) => {
            let lamports = v.get("lamports").and_then(|l| l.as_i64()).unwrap_or(0);
            let owner    = v.get("owner").and_then(|o| o.as_str())
                            .unwrap_or("11111111111111111111111111111111").to_string();
            let exe      = v.get("executable").and_then(|e| e.as_bool()).unwrap_or(false);
            let space    = v.get("data").and_then(|d| d.as_array())
                            .map(|arr| arr.first().and_then(|s| s.as_str())
                                .map(|s| (s.len() * 3 / 4) as i64).unwrap_or(0))
                            .unwrap_or(0);
            Ok((lamports, owner, exe, space))
        }
        None => Ok((0, "11111111111111111111111111111111".into(), false, 0)),
    }
}
