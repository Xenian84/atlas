//! GET /v1/block/:slot — block overview.

use axum::{extract::{Path, State}, Json};
use serde_json::{json, Value};
use sqlx::Row;
use crate::{error::ApiError, state::AppState};

pub async fn get_block(
    State(state): State<AppState>,
    Path(slot):   Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let pool = state.pool();

    // Block summary — two separate queries to avoid LATERAL UNNEST row-multiplication.
    // LATERAL UNNEST(programs) would multiply tx rows by program count, inflating
    // tx_count and total_fees incorrectly.
    let summary = sqlx::query(
        r#"SELECT COUNT(*)                               AS tx_count,
                  COUNT(*) FILTER (WHERE status = 1)    AS success_count,
                  COUNT(*) FILTER (WHERE status = 2)    AS failed_count,
                  COALESCE(SUM(fee_lamports), 0)::bigint AS total_fees,
                  MIN(block_time)                        AS block_time
           FROM tx_store
           WHERE slot = $1 AND commitment = 'confirmed'"#
    )
    .bind(slot)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("block {} not found or not indexed", slot)))?;

    let tx_count: i64 = summary.try_get("tx_count").unwrap_or(0);
    if tx_count == 0 {
        return Err(ApiError::NotFound(format!("block {} not found or not indexed", slot)));
    }

    let success_count: i64         = summary.try_get("success_count").unwrap_or(0);
    let failed_count:  i64         = summary.try_get("failed_count").unwrap_or(0);
    let total_fees:    i64         = summary.try_get("total_fees").unwrap_or(0);
    let block_time:    Option<i64> = summary.try_get("block_time").unwrap_or(None);

    // Collect distinct programs in a second query to avoid multiplying rows.
    let prog_rows = sqlx::query(
        "SELECT DISTINCT UNNEST(programs) AS prog FROM tx_store WHERE slot = $1 AND commitment = 'confirmed'"
    )
    .bind(slot)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let programs: Vec<String> = prog_rows.iter()
        .filter_map(|r| r.try_get::<String, _>("prog").ok())
        .take(10)
        .collect();

    // Recent transactions (first 20) ordered by position in block.
    let txs = sqlx::query(
        "SELECT sig, pos, status, fee_lamports, tags FROM tx_store \
         WHERE slot = $1 AND commitment = 'confirmed' ORDER BY pos ASC LIMIT 20"
    )
    .bind(slot)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let transactions: Vec<Value> = txs.iter().map(|r| json!({
        "sig":          r.try_get::<String, _>("sig").unwrap_or_default(),
        "pos":          r.try_get::<i32, _>("pos").unwrap_or(0),
        "status":       if r.try_get::<i16, _>("status").unwrap_or(0) == 1 { "success" } else { "failed" },
        "fee_lamports": r.try_get::<i64, _>("fee_lamports").unwrap_or(0),
        "tags":         r.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
    })).collect();

    Ok(Json(json!({
        "slot":          slot,
        "block_time":    block_time,
        "tx_count":      tx_count,
        "success_count": success_count,
        "failed_count":  failed_count,
        "total_fees":    total_fees,
        "programs":      programs,
        "transactions":  transactions,
    })))
}
