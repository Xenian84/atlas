use anyhow::Result;
use chrono::Utc;
use sqlx::{PgPool, Row};
use tracing::{info, warn};
use metrics::counter;
use atlas_common::{AppConfig, metrics as M};

use crate::delivery;

const POLL_INTERVAL_MS: u64 = 1000;

pub async fn run_delivery_worker(cfg: AppConfig, pool: PgPool) -> Result<()> {
    let concurrency  = cfg.webhook_worker_concurrency;
    let max_attempts = cfg.webhook_max_attempts as i32;
    // Single shared client with connection pooling
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    info!("Webhook delivery worker started (concurrency={})", concurrency);

    loop {
        // Use FOR UPDATE SKIP LOCKED to prevent duplicate delivery under concurrent workers
        let rows = match sqlx::query(
            r#"SELECT d.id, d.subscription_id, d.attempt_count, d.payload_json,
                      s.url, s.secret, s.event_type, s.format
               FROM webhook_deliveries d
               JOIN webhook_subscriptions s ON s.id = d.subscription_id
               WHERE d.status = 'pending'
                 AND d.next_attempt_at <= now()
                 AND s.is_active = true
               ORDER BY d.next_attempt_at ASC
               LIMIT $1
               FOR UPDATE OF d SKIP LOCKED"#
        )
        .bind(concurrency as i64)
        .fetch_all(&pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("Delivery worker DB poll error: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
                continue;
            }
        };

        if rows.is_empty() {
            tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
            continue;
        }

        // Mark all fetched rows as in_progress immediately to prevent re-pickup
        let ids: Vec<i64> = rows.iter().map(|r| r.try_get::<i64, _>("id").unwrap_or(0)).collect();
        if let Err(e) = sqlx::query(
            "UPDATE webhook_deliveries SET status='in_progress' WHERE id = ANY($1)"
        )
        .bind(&ids as &[i64])
        .execute(&pool)
        .await
        {
            warn!("Failed to mark deliveries in_progress: {}", e);
        }

        let mut handles = vec![];
        for row in rows {
            let client         = client.clone();
            let pool           = pool.clone();
            let max_att        = max_attempts;

            let id:            i64             = row.try_get("id").unwrap_or(0);
            let url:           String          = row.try_get("url").unwrap_or_default();
            let secret:        String          = row.try_get("secret").unwrap_or_default();
            let event_type:    String          = row.try_get("event_type").unwrap_or_default();
            let payload:       serde_json::Value = row.try_get("payload_json").unwrap_or_default();
            let attempt_count: i32             = row.try_get("attempt_count").unwrap_or(0);

            handles.push(tokio::spawn(async move {
                let (success, err_msg) = delivery::send_delivery(
                    &client, &url, &secret, &payload, id, &event_type
                ).await.unwrap_or((false, "send error".to_string()));

                let new_count = attempt_count + 1;

                if success {
                    let _ = sqlx::query(
                        "UPDATE webhook_deliveries SET status='success', attempt_count=$1 WHERE id=$2"
                    ).bind(new_count).bind(id).execute(&pool).await;
                    counter!(M::WEBHOOK_DELIVERIES_TOTAL).increment(1);
                } else if new_count >= max_att {
                    let _ = sqlx::query(
                        "UPDATE webhook_deliveries SET status='failed', attempt_count=$1, last_error=$2 WHERE id=$3"
                    ).bind(new_count).bind(&err_msg).bind(id).execute(&pool).await;
                    counter!(M::WEBHOOK_FAILURES_TOTAL).increment(1);
                } else {
                    let delay   = delivery::next_delay_secs(new_count);
                    let next_at = Utc::now() + chrono::Duration::seconds(delay);
                    let _ = sqlx::query(
                        "UPDATE webhook_deliveries SET status='pending', attempt_count=$1, next_attempt_at=$2, last_error=$3 WHERE id=$4"
                    ).bind(new_count).bind(next_at).bind(&err_msg).bind(id).execute(&pool).await;
                }
            }));
        }

        // Wait with timeout so a hung task doesn't block the worker loop forever
        for h in handles {
            match tokio::time::timeout(std::time::Duration::from_secs(30), h).await {
                Ok(Ok(_))  => {},
                Ok(Err(e)) => warn!("Delivery task panicked: {}", e),
                Err(_)     => warn!("Delivery task timed out"),
            }
        }
    }
}
