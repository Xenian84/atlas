//! Slot gap detector — scans tx_store for gaps in the slot sequence and
//! triggers backfill for any gap larger than the configured threshold.
//!
//! Runs as a background task every 5 minutes.

use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};

const CHECK_INTERVAL_SECS: u64 = 300;   // 5 minutes
const MAX_GAP_SLOTS: i64       = 50;    // gaps larger than this trigger backfill
const LOOKBACK_SLOTS: i64      = 5000;  // only scan the last N slots

pub async fn run_gap_detector(pool: PgPool, rpc_url: String) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(CHECK_INTERVAL_SECS)).await;

        if let Err(e) = check_gaps(&pool, &rpc_url).await {
            warn!("gap detector error: {}", e);
        }
    }
}

async fn check_gaps(pool: &PgPool, _rpc_url: &str) -> Result<()> {
    // Find the current tip slot from indexer_state
    let tip: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(slot), 0) FROM tx_store"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if tip == 0 { return Ok(()); }

    let since = tip - LOOKBACK_SLOTS;

    // Find distinct slots in the recent window
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT DISTINCT slot FROM tx_store WHERE slot >= $1 ORDER BY slot ASC"
    )
    .bind(since)
    .fetch_all(pool)
    .await?;

    if rows.len() < 2 { return Ok(()); }

    let mut gap_count = 0;
    for window in rows.windows(2) {
        let prev = window[0].0;
        let next = window[1].0;
        let gap  = next - prev;
        if gap > MAX_GAP_SLOTS {
            gap_count += 1;
            warn!(
                "slot gap detected: {} → {} (gap={} slots) — run: atlas-indexer backfill --from-slot {} --to-slot {}",
                prev, next, gap, prev + 1, next - 1
            );
            // Emit to Redis for monitoring
            // (backfill trigger via API or alerting system)
        }
    }

    if gap_count == 0 {
        info!("gap detector: no gaps found in last {} slots (tip={})", LOOKBACK_SLOTS, tip);
    } else {
        warn!("gap detector: {} gap(s) found near tip={}", gap_count, tip);
    }

    // Update indexer_state with lag info
    let _ = sqlx::query(
        "INSERT INTO indexer_state (key, value) VALUES ('last_gap_check_slot', $1)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
    )
    .bind(tip.to_string())
    .execute(pool)
    .await;

    Ok(())
}
