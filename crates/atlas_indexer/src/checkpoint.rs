use anyhow::Result;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, warn};

// One independent counter per commitment level — prevents processed from
// starving the confirmed/finalized checkpoint writers.
static LAST_CHECKPOINT_PROCESSED:  AtomicU64 = AtomicU64::new(0);
static LAST_CHECKPOINT_CONFIRMED:  AtomicU64 = AtomicU64::new(0);
static LAST_CHECKPOINT_FINALIZED:  AtomicU64 = AtomicU64::new(0);
static LAST_CHECKPOINT_OTHER:      AtomicU64 = AtomicU64::new(0);
const CHECKPOINT_INTERVAL: u64 = 100;

fn slot_counter(commitment: &str) -> &'static AtomicU64 {
    match commitment {
        "processed"  => &LAST_CHECKPOINT_PROCESSED,
        "confirmed"  => &LAST_CHECKPOINT_CONFIRMED,
        "finalized"  => &LAST_CHECKPOINT_FINALIZED,
        _            => &LAST_CHECKPOINT_OTHER,
    }
}

/// Update the indexer checkpoint every CHECKPOINT_INTERVAL slots.
/// The key stored in indexer_state reflects the actual commitment level.
pub async fn maybe_update(pool: &PgPool, slot: u64, commitment: &str) -> Result<()> {
    let counter = slot_counter(commitment);
    let last = counter.load(Ordering::Relaxed);
    if slot.saturating_sub(last) < CHECKPOINT_INTERVAL { return Ok(()); }

    match counter.compare_exchange(last, slot, Ordering::SeqCst, Ordering::Relaxed) {
        Err(_) => return Ok(()), // another caller for this commitment already advanced it
        Ok(_)  => {}
    }

    let key = format!("last_ingested_slot_{}", commitment);

    if let Err(e) = sqlx::query(
        r#"INSERT INTO indexer_state (key, value, updated_at) VALUES ($1, $2, now())
           ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()"#
    )
    .bind(&key)
    .bind(slot.to_string())
    .execute(pool)
    .await
    {
        warn!("Checkpoint write failed at slot {}: {}", slot, e);
        return Err(e.into());
    }

    debug!("Checkpoint: {} = slot {}", key, slot);
    Ok(())
}

pub async fn update_backfill_progress(pool: &PgPool, from: u64, to: u64, current: u64) -> Result<()> {
    let progress = serde_json::json!({ "from": from, "to": to, "current": current });

    if let Err(e) = sqlx::query(
        r#"INSERT INTO indexer_state (key, value, updated_at) VALUES ('backfill_progress', $1, now())
           ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()"#
    )
    .bind(progress.to_string())
    .execute(pool)
    .await
    {
        warn!("Backfill progress write failed: {}", e);
        return Err(e.into());
    }

    Ok(())
}
