use axum::{extract::{Path, State, Query}, http::HeaderMap, response::Response};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use atlas_types::facts::{TxFactsV1, TxSummary, TxStatus, Commitment};
use atlas_toon::render_txfacts;
use atlas_common::redis_ext;
use crate::{state::AppState, error::ApiError, negotiate::{negotiate, respond}, explain};

#[derive(Deserialize, Default)]
pub struct FormatQuery {
    pub format: Option<String>,
}

/// GET /v1/tx/:sig — full TxFactsV1
pub async fn get_tx(
    State(state): State<AppState>,
    Path(sig):    Path<String>,
    Query(q):     Query<FormatQuery>,
    headers:      HeaderMap,
) -> Result<Response, ApiError> {
    let facts = fetch_tx_facts(&state, &sig).await?;
    let format = negotiate(&headers, q.format.as_deref());
    Ok(respond(format, &facts, render_txfacts(&facts)))
}

/// GET /v1/tx/:sig/enhanced — actions + deltas only
pub async fn get_tx_enhanced(
    State(state): State<AppState>,
    Path(sig):    Path<String>,
    Query(q):     Query<FormatQuery>,
    headers:      HeaderMap,
) -> Result<Response, ApiError> {
    let facts   = fetch_tx_facts(&state, &sig).await?;
    let summary = TxSummary::from(&facts);
    let format  = negotiate(&headers, q.format.as_deref());
    // Use the full TOON renderer — same as /tx/:sig but client receives TxSummary JSON
    let toon    = render_txfacts(&facts);
    Ok(respond(format, &summary, toon))
}

/// POST /v1/tx/:sig/explain
pub async fn explain_tx(
    State(state): State<AppState>,
    Path(sig):    Path<String>,
) -> Result<axum::Json<serde_json::Value>, ApiError> {
    let facts      = fetch_tx_facts(&state, &sig).await?;
    let facts_toon = render_txfacts(&facts);
    let exp        = explain::explain_with_llm(&facts, state.http(), state.cfg()).await;

    Ok(axum::Json(json!({
        "facts":     facts,
        "explain":   exp,
        "factsToon": facts_toon,
    })))
}

// ── DB helper ─────────────────────────────────────────────────────────────────

pub async fn fetch_tx_facts(state: &AppState, sig: &str) -> Result<TxFactsV1, ApiError> {
    let cache_key = format!("tx:{}", sig);
    let mut redis = state.redis();

    if let Some(cached) = redis_ext::cache_get::<TxFactsV1>(&mut redis, &cache_key).await {
        metrics::counter!(atlas_common::metrics::CACHE_HIT_TOTAL).increment(1);
        return Ok(cached);
    }
    metrics::counter!(atlas_common::metrics::CACHE_MISS_TOTAL).increment(1);

    let row = sqlx::query(
        r#"SELECT sig, slot, pos, block_time, status, fee_lamports,
                  compute_consumed, compute_limit, priority_fee_micro_lamports,
                  programs, tags, accounts_json, actions_json,
                  token_deltas_json, sol_deltas_json, err_json, raw_ref, commitment
           FROM tx_store WHERE sig = $1"#
    )
    .bind(sig)
    .fetch_optional(state.pool())
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("tx {} not found", sig)))?;

    let mut facts = TxFactsV1::new(
        row.try_get::<String, _>("sig")?,
        row.try_get::<i64, _>("slot")? as u64,
        row.try_get::<i32, _>("pos")? as u32,
    );
    facts.block_time   = row.try_get("block_time")?;
    facts.status       = TxStatus::from_smallint(row.try_get::<i16, _>("status")?);
    facts.fee_lamports = row.try_get::<i64, _>("fee_lamports")? as u64;
    facts.compute_units.consumed = row.try_get::<Option<i32>, _>("compute_consumed")?.map(|v| v as u32);
    facts.compute_units.limit    = row.try_get::<Option<i32>, _>("compute_limit")?.map(|v| v as u32);
    facts.compute_units.price_micro_lamports = row.try_get::<Option<i64>, _>("priority_fee_micro_lamports")?.map(|v| v as u64);
    facts.commitment   = row.try_get::<String, _>("commitment")
        .ok()
        .and_then(|s| match s.as_str() {
            "processed" => Some(Commitment::Processed),
            "finalized" => Some(Commitment::Finalized),
            "confirmed" => Some(Commitment::Confirmed),
            _ => None,
        })
        .unwrap_or(Commitment::Confirmed);
    facts.programs     = row.try_get("programs")?;
    facts.tags         = row.try_get("tags")?;
    facts.accounts     = serde_json::from_value(row.try_get("accounts_json")?).unwrap_or_default();
    facts.actions      = serde_json::from_value(row.try_get("actions_json")?).unwrap_or_default();
    facts.token_deltas = serde_json::from_value(row.try_get("token_deltas_json")?).unwrap_or_default();
    facts.sol_deltas   = serde_json::from_value(row.try_get("sol_deltas_json")?).unwrap_or_default();
    facts.err          = row.try_get("err_json")?;
    facts.raw_ref      = row.try_get("raw_ref")?;

    let _ = redis_ext::cache_set(&mut redis, &cache_key, &facts, 120).await;
    Ok(facts)
}
