//! GET /v1/network/pulse — compact network health snapshot.
//!
//! Designed for AI agent heartbeat consumption: returns a sub-500-token
//! TOON document summarising current X1 network activity.
//!
//! ## Response (TOON, default)
//! ```
//! pulse:
//!  chain:         x1
//!  slot:          285000000
//!  block_time:    1709000000
//!  tps_1m:        1240
//!  active_wallets_24h: 8500
//!  indexed_txs_24h:    72000
//!
//! top_programs[5]{program,calls}:
//!  TokenkegQ...,45000
//!  ...
//!
//! top_tags[5]{tag,count}:
//!  transfer,30000
//!  swap,12000
//! ```

use axum::{extract::State, http::HeaderMap, response::Response};
use serde_json::json;
use sqlx::Row;
use crate::{state::AppState, error::ApiError, negotiate::{negotiate, respond}};

pub async fn network_pulse(
    State(state): State<AppState>,
    headers:      HeaderMap,
) -> Result<Response, ApiError> {
    let pool = state.pool();

    // ── slot from indexer_state table ──────────────────────────────────────
    let slot_row = sqlx::query(
        "SELECT value FROM indexer_state WHERE key = 'last_ingested_slot_confirmed'"
    )
    .fetch_optional(pool).await;
    let current_slot: i64 = slot_row.ok().flatten()
        .and_then(|r| r.try_get::<String, _>("value").ok())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    // ── 24h activity ─────────────────────────────────────────────────────────
    let since_24h = chrono::Utc::now().timestamp() - 86400;

    let tx_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tx_store WHERE block_time >= $1"
    )
    .bind(since_24h)
    .fetch_one(pool).await.unwrap_or(0);

    let wallets_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM intelligence_wallet_profiles"
    )
    .fetch_one(pool).await.unwrap_or(0);

    // Use created_at for TPS — block_time can lag behind real time when the
    // indexer is catching up, which would cause tps_1m to always read 0.
    let tps_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tx_store WHERE created_at >= now() - interval '1 minute'"
    )
    .fetch_one(pool).await.unwrap_or(0);
    let tps = tps_count / 60;

    // ── top programs — last 2000 rows (slot DESC index → instant) ───────────
    let prog_rows = sqlx::query(
        r#"SELECT unnest(programs) AS program, COUNT(*) AS calls
           FROM (SELECT programs FROM tx_store ORDER BY slot DESC LIMIT 2000) recent
           GROUP BY program
           ORDER BY calls DESC
           LIMIT 5"#
    )
    .fetch_all(pool).await.unwrap_or_default();

    // ── top tags — same 2000-row sample ─────────────────────────────────────
    let tag_rows = sqlx::query(
        r#"SELECT unnest(tags) AS tag, COUNT(*) AS cnt
           FROM (SELECT tags FROM tx_store ORDER BY slot DESC LIMIT 2000) recent
           GROUP BY tag
           ORDER BY cnt DESC
           LIMIT 5"#
    )
    .fetch_all(pool).await.unwrap_or_default();

    // ── latest block time ───────────────────────────────────────────────────
    let latest_bt: i64 = sqlx::query(
        "SELECT MAX(block_time) AS bt FROM tx_store"
    )
    .fetch_one(pool).await
    .ok()
    .and_then(|r| r.try_get::<Option<i64>, _>("bt").ok().flatten())
    .unwrap_or(0);

    // ── indexer lag ─────────────────────────────────────────────────────────
    let indexed_slot: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(slot), 0) FROM tx_store"
    ).fetch_one(pool).await.unwrap_or(0);

    let network_slot: i64 = sqlx::query_scalar(
        "SELECT value::bigint FROM indexer_state WHERE key = 'last_ingested_slot_confirmed'"
    ).fetch_optional(pool).await.unwrap_or(None).unwrap_or(current_slot);

    let indexed_accounts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM accounts"
    ).fetch_one(pool).await.unwrap_or(0);

    let indexed_tokens: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM token_metadata"
    ).fetch_one(pool).await.unwrap_or(0);

    let pending_webhooks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM webhook_deliveries WHERE status = 'pending'"
    ).fetch_one(pool).await.unwrap_or(0);

    let lag_slots = (network_slot - indexed_slot).max(0);

    // ── assemble response ───────────────────────────────────────────────────
    let pulse = json!({
        "chain":               "x1",
        "slot":                current_slot,
        "block_time":          latest_bt,
        "tps_1m":              tps,
        "active_wallets_24h":  wallets_24h,
        "indexed_txs_24h":     tx_24h,
        "indexer": {
            "indexed_slot":     indexed_slot,
            "lag_slots":        lag_slots,
            "indexed_accounts": indexed_accounts,
            "indexed_tokens":   indexed_tokens,
            "pending_webhooks": pending_webhooks,
        },
        "top_programs": prog_rows.iter().map(|r| json!({
            "program": r.try_get::<String, _>("program").unwrap_or_default(),
            "calls":   r.try_get::<i64, _>("calls").unwrap_or(0),
        })).collect::<Vec<_>>(),
        "top_tags": tag_rows.iter().map(|r| json!({
            "tag":   r.try_get::<String, _>("tag").unwrap_or_default(),
            "count": r.try_get::<i64, _>("cnt").unwrap_or(0),
        })).collect::<Vec<_>>(),
    });

    let toon = render_pulse_toon(&pulse);
    let fmt  = negotiate(&headers, None);
    Ok(respond(fmt, &pulse, toon))
}

fn render_pulse_toon(p: &serde_json::Value) -> String {
    let mut out = String::new();
    out.push_str("pulse:\n");
    out.push_str(&format!(" chain:              {}\n", p["chain"].as_str().unwrap_or("x1")));
    out.push_str(&format!(" slot:               {}\n", p["slot"].as_i64().unwrap_or(0)));
    out.push_str(&format!(" block_time:         {}\n", p["block_time"].as_i64().unwrap_or(0)));
    out.push_str(&format!(" tps_1m:             {}\n", p["tps_1m"].as_i64().unwrap_or(0)));
    out.push_str(&format!(" active_wallets_24h: {}\n", p["active_wallets_24h"].as_i64().unwrap_or(0)));
    out.push_str(&format!(" indexed_txs_24h:    {}\n", p["indexed_txs_24h"].as_i64().unwrap_or(0)));
    out.push('\n');

    let progs = p["top_programs"].as_array().map(|v| v.as_slice()).unwrap_or_default();
    out.push_str(&format!("top_programs[{}]{{program,calls}}:\n", progs.len()));
    for prog in progs {
        out.push_str(&format!(
            " {},{}\n",
            &prog["program"].as_str().unwrap_or("")[..prog["program"].as_str().unwrap_or("").len().min(20)],
            prog["calls"].as_i64().unwrap_or(0)
        ));
    }
    out.push('\n');

    let tags = p["top_tags"].as_array().map(|v| v.as_slice()).unwrap_or_default();
    out.push_str(&format!("top_tags[{}]{{tag,count}}:\n", tags.len()));
    for tag in tags {
        out.push_str(&format!(
            " {},{}\n",
            tag["tag"].as_str().unwrap_or(""),
            tag["count"].as_i64().unwrap_or(0)
        ));
    }

    out
}

/// GET /v1/network/tps — per-minute TPS over the last hour from indexed tx_store.
/// Returns Atlas-native data (no validator RPC needed) for the TPS chart.
pub async fn network_tps(
    State(state): State<AppState>,
) -> Result<axum::Json<serde_json::Value>, ApiError> {
    use sqlx::Row;
    let pool = state.pool();

    let rows = sqlx::query(
        r#"SELECT
             date_trunc('minute', created_at) AS minute,
             COUNT(*)                          AS tx_count
           FROM tx_store
           WHERE created_at > now() - interval '60 minutes'
             AND commitment = 'confirmed'
           GROUP BY 1
           ORDER BY 1 ASC"#
    )
    .fetch_all(pool)
    .await?;

    let samples: Vec<serde_json::Value> = rows.iter().map(|r| {
        let minute: chrono::DateTime<chrono::Utc> = r.try_get("minute").unwrap_or_default();
        let tx_count: i64 = r.try_get("tx_count").unwrap_or(0);
        json!({
            "time":  minute.format("%H:%M").to_string(),
            "ts":    minute.timestamp(),
            "tps":   tx_count / 60,
        })
    }).collect();

    Ok(axum::Json(json!({ "samples": samples })))
}
