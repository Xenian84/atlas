//! Shred consumer — reads from the Redis `atlas:shreds` stream and writes
//! a lightweight "shred" record to the DB + broadcast stream at sub-50ms latency.
//!
//! This is the fastest path in Atlas:
//!   tachyon banking stage
//!     → atlas_bridge.rs (Unix socket, bincode)
//!     → atlas-shredstream binary (receiver.rs, Redis XADD)
//!     → atlas:shreds stream  ← WE ARE HERE
//!     → transactions table (commitment = 'shred')
//!     → atlas:newtx broadcast
//!
//! When the confirmed gRPC stream later fires for the same tx, the upsert in
//! stream.rs upgrades the row from commitment='shred' to commitment='confirmed'
//! and fills in all enriched fields (parsed instructions, DAS updates, etc.).

use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use sqlx::PgPool;
use tracing::{info, warn, error};
use metrics::counter;

const GROUP:    &str = "atlas-indexer-shred";
const CONSUMER: &str = "indexer-0";
const BLOCK:    usize = 100;         // max messages per XREADGROUP call
const BLOCK_MS: usize = 1_000;      // block up to 1s waiting for new messages

pub async fn run_shred_consumer(
    mut redis:    ConnectionManager,
    pool:         PgPool,
    shred_stream: String,
    tx_stream:    String,         // atlas:newtx — where we publish the early event
) -> Result<()> {
    // Create consumer group idempotently (start from latest = "$")
    let result: redis::RedisResult<String> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(&shred_stream)
        .arg(GROUP)
        .arg("$")
        .arg("MKSTREAM")
        .query_async(&mut redis)
        .await;

    match result {
        Ok(_) => info!("Shred consumer group '{}' created on '{}'", GROUP, shred_stream),
        Err(e) if e.to_string().contains("BUSYGROUP") => {
            info!("Shred consumer group '{}' already exists", GROUP);
        }
        Err(e) => return Err(e.into()),
    }

    info!("Shred consumer listening on {} (group={}, consumer={})",
        shred_stream, GROUP, CONSUMER);

    loop {
        let msgs: redis::RedisResult<redis::streams::StreamReadReply> = redis::cmd("XREADGROUP")
            .arg("GROUP").arg(GROUP).arg(CONSUMER)
            .arg("COUNT").arg(BLOCK)
            .arg("BLOCK").arg(BLOCK_MS)
            .arg("STREAMS").arg(&shred_stream).arg(">")
            .query_async(&mut redis)
            .await;

        let msgs = match msgs {
            Ok(m)  => m,
            Err(e) => {
                warn!("XREADGROUP error: {} — retrying in 2s", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                continue;
            }
        };

        let mut ack_ids: Vec<String> = Vec::new();

        for key in msgs.keys {
            for msg in key.ids {
                let id = msg.id.clone();
                let raw: String = match msg.map.get("data")
                    .and_then(|v| if let redis::Value::Data(b) = v {
                        String::from_utf8(b.clone()).ok()
                    } else { None })
                {
                    Some(s) => s,
                    None    => { ack_ids.push(id); continue; }
                };

                let event: serde_json::Value = match serde_json::from_str(&raw) {
                    Ok(v)  => v,
                    Err(e) => {
                        warn!("Shred event JSON parse error: {}", e);
                        ack_ids.push(id);
                        continue;
                    }
                };

                process_shred_event(&pool, &mut redis, &tx_stream, &event).await;
                counter!("shred_consumer.events_processed").increment(1);
                ack_ids.push(id);
            }
        }

        // Acknowledge processed messages
        if !ack_ids.is_empty() {
            let _: redis::RedisResult<u64> = redis::cmd("XACK")
                .arg(&shred_stream)
                .arg(GROUP)
                .arg(&ack_ids)
                .query_async(&mut redis)
                .await;
        }
    }
}

async fn process_shred_event(
    pool:      &PgPool,
    redis:     &mut ConnectionManager,
    tx_stream: &str,
    event:     &serde_json::Value,
) {
    let sig = match event["sig"].as_str() {
        Some(s) => s,
        None    => return,
    };
    let slot       = event["slot"].as_u64().unwrap_or(0);
    let entry_idx  = event["entry_idx"].as_u64().unwrap_or(0);
    let latency_us = event["latency_us"].as_i64().unwrap_or(0);
    let accounts   = serde_json::to_string(&event["accounts"]).unwrap_or_default();
    let programs   = serde_json::to_string(&event["programs"]).unwrap_or_default();

    // ── Write shred-level record to transactions table ─────────────────────
    // Uses INSERT ... ON CONFLICT DO NOTHING so a later confirmed upsert wins.
    // Insert a minimal shred-level row. tx_store uses ON CONFLICT DO NOTHING
    // so when the confirmed gRPC path arrives later, it upserts the full row.
    let db_result = sqlx::query(
        r#"
        INSERT INTO tx_store (
            sig, slot, pos, block_time, status, fee_lamports,
            programs, tags, accounts_json, actions_json,
            token_deltas_json, sol_deltas_json,
            commitment, created_at
        )
        VALUES (
            $1, $2, $3, NULL, 0, 0,
            $4, ARRAY[]::text[], $5::jsonb, '[]'::jsonb,
            '[]'::jsonb, '[]'::jsonb,
            'shred', NOW()
        )
        ON CONFLICT (sig) DO NOTHING
        "#
    )
    .bind(sig)
    .bind(slot as i64)
    .bind(entry_idx as i32)
    .bind(&serde_json::from_str::<Vec<String>>(&programs).unwrap_or_default())
    .bind(&accounts)
    .execute(pool)
    .await;

    if let Err(e) = db_result {
        error!("Shred DB insert failed for {}: {}", sig, e);
        return;
    }

    // ── Publish early broadcast event ──────────────────────────────────────
    let broadcast = serde_json::json!({
        "sig":        sig,
        "slot":       slot,
        "entry_idx":  entry_idx,
        "commitment": "shred",
        "accounts":   event["accounts"],
        "programs":   event["programs"],
        "latency_us": latency_us,
    });

    let data = broadcast.to_string();
    let r: redis::RedisResult<String> = redis.xadd(tx_stream, "*", &[("data", data.as_str())]).await;
    if let Err(e) = r {
        warn!("Shred XADD {} failed for {}: {}", tx_stream, sig, e);
    }
}
