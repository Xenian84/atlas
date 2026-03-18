//! POST /v1/txs/batch — bulk transaction lookup (up to 100 sigs).

use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use crate::{error::ApiError, state::AppState};

#[derive(Deserialize)]
pub struct BatchTxRequest {
    pub signatures: Vec<String>,
}

pub async fn batch_get_txs(
    State(state): State<AppState>,
    Json(req):    Json<BatchTxRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.signatures.is_empty() {
        return Ok(Json(json!({ "transactions": [], "found": 0, "missing": [] })));
    }
    if req.signatures.len() > 100 {
        return Err(ApiError::BadRequest("max 100 signatures per batch".into()));
    }

    let rows = sqlx::query(
        r#"SELECT sig, slot, pos, block_time, status, fee_lamports,
                  compute_consumed, compute_limit, priority_fee_micro_lamports,
                  programs, tags, commitment, err_json
           FROM tx_store WHERE sig = ANY($1)"#
    )
    .bind(&req.signatures as &[String])
    .fetch_all(state.pool())
    .await?;

    let found_sigs: std::collections::HashSet<String> = rows.iter()
        .filter_map(|r| r.try_get::<String, _>("sig").ok())
        .collect();

    let missing: Vec<&String> = req.signatures.iter()
        .filter(|s| !found_sigs.contains(*s))
        .collect();

    let transactions: Vec<Value> = rows.iter().map(|r| json!({
        "sig":          r.try_get::<String, _>("sig").unwrap_or_default(),
        "slot":         r.try_get::<i64, _>("slot").unwrap_or(0),
        "pos":          r.try_get::<i32, _>("pos").unwrap_or(0),
        "block_time":   r.try_get::<Option<i64>, _>("block_time").ok().flatten(),
        "status":       if r.try_get::<i16, _>("status").unwrap_or(0) == 1 { "success" } else { "failed" },
        "fee_lamports": r.try_get::<i64, _>("fee_lamports").unwrap_or(0),
        "compute_consumed": r.try_get::<Option<i32>, _>("compute_consumed").ok().flatten(),
        "compute_limit":    r.try_get::<Option<i32>, _>("compute_limit").ok().flatten(),
        "priority_fee_micro_lamports": r.try_get::<Option<i64>, _>("priority_fee_micro_lamports").ok().flatten(),
        "programs":     r.try_get::<Vec<String>, _>("programs").unwrap_or_default(),
        "tags":         r.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
        "commitment":   r.try_get::<String, _>("commitment").unwrap_or_default(),
        "err":          r.try_get::<Option<Value>, _>("err_json").ok().flatten(),
    })).collect();

    Ok(Json(json!({
        "transactions": transactions,
        "found":        found_sigs.len(),
        "missing":      missing,
    })))
}
