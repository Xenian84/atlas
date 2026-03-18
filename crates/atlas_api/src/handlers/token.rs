//! Token API endpoints.
//!
//! GET /v1/token/:mint           — token overview (metadata + stats)
//! GET /v1/token/:mint/holders   — top holders by balance
//! GET /v1/token/:mint/transfers — recent transfer history

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use crate::{error::ApiError, state::AppState};

// ── GET /v1/token/:mint ───────────────────────────────────────────────────────

pub async fn get_token(
    State(state): State<AppState>,
    Path(mint):   Path<String>,
) -> Result<Json<Value>, ApiError> {
    let pool = state.pool();

    let meta = sqlx::query(
        "SELECT name, symbol, decimals, supply, logo_uri, uri, is_nft, updated_at FROM token_metadata WHERE mint = $1"
    )
    .bind(&mint)
    .fetch_optional(pool)
    .await?;

    let (name, symbol, decimals, supply, logo_uri, uri, is_nft) = match meta {
        Some(r) => (
            r.try_get::<String, _>("name").unwrap_or_default(),
            r.try_get::<String, _>("symbol").unwrap_or_default(),
            r.try_get::<i16, _>("decimals").unwrap_or(0),
            r.try_get::<i64, _>("supply").unwrap_or(0),
            r.try_get::<Option<String>, _>("logo_uri").ok().flatten(),
            r.try_get::<Option<String>, _>("uri").ok().flatten(),
            r.try_get::<bool, _>("is_nft").unwrap_or(false),
        ),
        None => (String::new(), String::new(), 0, 0, None, None, false),
    };

    // Holder count from token_account_index
    let holders: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM token_account_index WHERE mint = $1 AND amount > 0"
    )
    .bind(&mint)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 24h transfer count — use token_balance_index directly joined to tx_store
    // to avoid a full table scan on tx_store (block_time has no index).
    let since_24h = chrono::Utc::now().timestamp() - 86400;
    let transfers_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM token_balance_index tbi
         JOIN tx_store ts ON ts.sig = tbi.sig AND ts.commitment = 'confirmed'
         WHERE tbi.mint = $1 AND ts.block_time >= $2"
    )
    .bind(&mint)
    .bind(since_24h)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Identity label if known program
    let identity = sqlx::query(
        "SELECT name, category FROM entity_labels WHERE address = $1"
    )
    .bind(&mint)
    .fetch_optional(pool)
    .await?
    .map(|r| json!({
        "name":     r.try_get::<String, _>("name").unwrap_or_default(),
        "category": r.try_get::<String, _>("category").unwrap_or_default(),
    }));

    Ok(Json(json!({
        "mint":          mint,
        "name":          name,
        "symbol":        symbol,
        "decimals":      decimals,
        "supply":        supply,
        "logo_uri":      logo_uri,
        "uri":           uri,
        "is_nft":        is_nft,
        "holders":       holders,
        "transfers_24h": transfers_24h,
        "identity":      identity,
    })))
}

// ── GET /v1/token/:mint/holders ───────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct PageQuery {
    pub limit:  Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_token_holders(
    State(state): State<AppState>,
    Path(mint):   Path<String>,
    Query(q):     Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    let pool   = state.pool();
    let limit  = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);

    let rows = sqlx::query(
        r#"SELECT owner, token_account, amount, decimals
           FROM token_account_index
           WHERE mint = $1 AND amount > 0
           ORDER BY amount DESC
           LIMIT $2 OFFSET $3"#
    )
    .bind(&mint)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM token_account_index WHERE mint = $1 AND amount > 0"
    )
    .bind(&mint)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let holders: Vec<Value> = rows.iter().map(|r| json!({
        "owner":         r.try_get::<String, _>("owner").unwrap_or_default(),
        "token_account": r.try_get::<String, _>("token_account").unwrap_or_default(),
        "amount":        r.try_get::<sqlx::types::Decimal, _>("amount").map(|d| d.to_string()).unwrap_or_else(|_| "0".into()),
        "decimals":      r.try_get::<i16, _>("decimals").unwrap_or(0),
    })).collect();

    Ok(Json(json!({
        "mint":    mint,
        "total":   total,
        "holders": holders,
        "pagination": { "limit": limit, "offset": offset },
    })))
}

// ── GET /v1/token/:mint/transfers ─────────────────────────────────────────────

pub async fn get_token_transfers(
    State(state): State<AppState>,
    Path(mint):   Path<String>,
    Query(q):     Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    let pool   = state.pool();
    let limit  = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);

    let rows = sqlx::query(
        r#"SELECT tbi.owner, tbi.sig, tbi.slot, tbi.delta, tbi.direction,
                  ts.block_time
           FROM token_balance_index tbi
           LEFT JOIN tx_store ts ON ts.sig = tbi.sig
           WHERE tbi.mint = $1
           ORDER BY tbi.slot DESC, tbi.pos DESC
           LIMIT $2 OFFSET $3"#
    )
    .bind(&mint)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let transfers: Vec<Value> = rows.iter().map(|r| json!({
        "sig":        r.try_get::<String, _>("sig").unwrap_or_default(),
        "slot":       r.try_get::<i64, _>("slot").unwrap_or(0),
        "block_time": r.try_get::<Option<i64>, _>("block_time").ok().flatten(),
        "owner":      r.try_get::<String, _>("owner").unwrap_or_default(),
        "delta":      r.try_get::<sqlx::types::Decimal, _>("delta").map(|d| d.to_string()).unwrap_or_else(|_| "0".into()),
        "direction":  if r.try_get::<i16, _>("direction").unwrap_or(0) == 1 { "in" } else { "out" },
    })).collect();

    Ok(Json(json!({
        "mint":      mint,
        "transfers": transfers,
        "pagination": { "limit": limit, "offset": offset },
    })))
}
