//! Webhook delivery worker — polls webhook_deliveries for pending rows and
//! dispatches HTTP POST with exponential backoff.
//!
//! Delivery lifecycle:
//!   pending → (HTTP POST) → success | failed
//!   failed  → retry after backoff (up to MAX_ATTEMPTS)
//!   failed  → dead after MAX_ATTEMPTS
//!
//! Trigger: after every confirmed tx, check webhook_subscriptions for matches
//! and insert a webhook_deliveries row. This worker picks them up.

use anyhow::Result;
use reqwest::Client;
use sqlx::{PgPool, Row};
use tracing::{info, warn, debug};
use serde_json::Value;
use std::time::Duration;

const POLL_INTERVAL_SECS: u64 = 2;
const MAX_ATTEMPTS: i32       = 6;
// Backoff: 10s, 60s, 5m, 30m, 2h, 6h
const BACKOFF_SECS: [u64; 6]  = [10, 60, 300, 1800, 7200, 21600];

pub async fn run_webhook_worker(pool: PgPool) {
    let http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("webhook http client");

    info!("Webhook delivery worker started");

    loop {
        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;

        if let Err(e) = process_pending(&pool, &http).await {
            warn!("webhook worker error: {}", e);
        }
    }
}

async fn process_pending(pool: &PgPool, http: &Client) -> Result<()> {
    // Fetch up to 20 pending deliveries whose next_attempt_at is due
    let rows = sqlx::query(
        r#"SELECT d.id, d.subscription_id, d.payload_json, d.attempt_count,
                  s.url, s.secret
           FROM webhook_deliveries d
           JOIN webhook_subscriptions s ON s.id = d.subscription_id
           WHERE d.status = 'pending'
             AND d.next_attempt_at <= now()
             AND d.attempt_count < $1
             AND s.is_active = true
           ORDER BY d.next_attempt_at ASC
           LIMIT 20
           FOR UPDATE OF d SKIP LOCKED"#
    )
    .bind(MAX_ATTEMPTS)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let delivery_id: i64  = row.try_get("id")?;
        let url: String       = row.try_get("url")?;
        let secret: String    = row.try_get("secret")?;
        let payload: Value    = row.try_get("payload_json")?;
        let attempt: i32      = row.try_get("attempt_count")?;

        let result = dispatch(http, &url, &secret, &payload).await;

        match result {
            Ok(status) if status < 300 => {
                sqlx::query(
                    "UPDATE webhook_deliveries SET status='success', attempt_count=$2 WHERE id=$1"
                )
                .bind(delivery_id)
                .bind(attempt + 1)
                .execute(pool)
                .await?;
                debug!("webhook {} delivered to {}", delivery_id, url);
            }
            Ok(status) => {
                let next_secs = next_backoff(attempt);
                let new_status = if attempt + 1 >= MAX_ATTEMPTS { "failed" } else { "pending" };
                sqlx::query(
                    r#"UPDATE webhook_deliveries
                       SET status=$2, attempt_count=$3, last_error=$4,
                           next_attempt_at=now()+($5 || ' seconds')::interval
                       WHERE id=$1"#
                )
                .bind(delivery_id)
                .bind(new_status)
                .bind(attempt + 1)
                .bind(format!("HTTP {status}"))
                .bind(next_secs.to_string())
                .execute(pool)
                .await?;
                warn!("webhook {} got HTTP {} (attempt {}/{})", delivery_id, status, attempt+1, MAX_ATTEMPTS);
            }
            Err(e) => {
                let next_secs = next_backoff(attempt);
                let new_status = if attempt + 1 >= MAX_ATTEMPTS { "failed" } else { "pending" };
                sqlx::query(
                    r#"UPDATE webhook_deliveries
                       SET status=$2, attempt_count=$3, last_error=$4,
                           next_attempt_at=now()+($5 || ' seconds')::interval
                       WHERE id=$1"#
                )
                .bind(delivery_id)
                .bind(new_status)
                .bind(attempt + 1)
                .bind(e.to_string())
                .bind(next_secs.to_string())
                .execute(pool)
                .await?;
                warn!("webhook {} dispatch error (attempt {}/{}): {}", delivery_id, attempt+1, MAX_ATTEMPTS, e);
            }
        }
    }

    Ok(())
}

async fn dispatch(http: &Client, url: &str, secret: &str, payload: &Value) -> Result<u16> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let body = serde_json::to_vec(payload)?;

    // HMAC-SHA256 signature for webhook verification
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|e| anyhow::anyhow!("hmac key error: {}", e))?;
    mac.update(&body);
    let sig = hex::encode(mac.finalize().into_bytes());

    let resp = http.post(url)
        .header("Content-Type",     "application/json")
        .header("X-Atlas-Signature", format!("sha256={}", sig))
        .header("X-Atlas-Version",  "1")
        .body(body)
        .send()
        .await?;

    Ok(resp.status().as_u16())
}

fn next_backoff(attempt: i32) -> u64 {
    let idx = (attempt as usize).min(BACKOFF_SECS.len() - 1);
    BACKOFF_SECS[idx]
}

/// Insert a webhook_deliveries row for every matching subscription.
/// Called from persist_tx after indexing each confirmed transaction.
pub async fn trigger_webhooks(pool: &PgPool, facts: &atlas_types::facts::TxFactsV1) -> Result<()> {
    let addresses = facts.all_addresses();
    let programs  = &facts.programs;
    let tags      = &facts.tags;

    let payload = serde_json::json!({
        "sig":          &facts.sig,
        "slot":         facts.slot,
        "block_time":   facts.block_time,
        "status":       if facts.is_success() { "success" } else { "failed" },
        "tags":         tags,
        "programs":     programs,
        "accounts":     &addresses,
        "fee_lamports": facts.fee_lamports,
    });

    // Find subscriptions matching this tx by address, program, or event type
    let subs = sqlx::query(
        r#"SELECT id, event_type, address, program_id
           FROM webhook_subscriptions
           WHERE is_active = true
             AND (
               (event_type = 'address_activity' AND address = ANY($1))
               OR (event_type = 'program_activity' AND program_id = ANY($2))
               OR event_type = 'token_balance_changed'
             )"#
    )
    .bind(&addresses as &[String])
    .bind(programs as &[String])
    .fetch_all(pool)
    .await?;

    for sub in subs {
        let sub_id: uuid::Uuid = sub.try_get("id")?;
        let _ = sqlx::query(
            r#"INSERT INTO webhook_deliveries
               (subscription_id, payload_json, status, attempt_count, next_attempt_at)
               VALUES ($1, $2, 'pending', 0, now())"#
        )
        .bind(sub_id)
        .bind(&payload)
        .execute(pool)
        .await;
    }

    Ok(())
}
