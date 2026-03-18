use anyhow::Result;
use sqlx::{PgPool, Row};
use tracing::{debug, warn};
use serde_json::Value;

pub async fn process_event(pool: &PgPool, event: &Value) -> Result<()> {
    let sig = match event["sig"].as_str() {
        Some(s) if !s.is_empty() => s,
        _ => {
            warn!("Webhook event missing 'sig' field, skipping");
            return Ok(());
        }
    };

    let slot     = event["slot"].as_i64().unwrap_or(0);
    let pos      = event["pos"].as_i64().unwrap_or(0);
    let accounts: Vec<String> = serde_json::from_value(event["accounts"].clone()).unwrap_or_default();
    let programs: Vec<String> = serde_json::from_value(event["programs"].clone()).unwrap_or_default();

    // address_activity subscriptions
    if !accounts.is_empty() {
        let subs = sqlx::query(
            r#"SELECT id, url, secret, format, event_type
               FROM webhook_subscriptions
               WHERE is_active = true AND event_type = 'address_activity'
                 AND address = ANY($1)"#
        )
        .bind(&accounts as &[String])
        .fetch_all(pool)
        .await?;

        for sub in subs {
            let sub_id: uuid::Uuid = sub.try_get("id")?;
            let payload = build_payload("address_activity", sig, slot, pos, event);
            create_delivery(pool, sub_id, payload).await?;
            debug!("Queued address_activity delivery for sub {} sig={}", sub_id, sig);
        }
    }

    // program_activity subscriptions
    if !programs.is_empty() {
        let subs = sqlx::query(
            r#"SELECT id, url, secret, format, event_type
               FROM webhook_subscriptions
               WHERE is_active = true AND event_type = 'program_activity'
                 AND program_id = ANY($1)"#
        )
        .bind(&programs as &[String])
        .fetch_all(pool)
        .await?;

        for sub in subs {
            let sub_id: uuid::Uuid = sub.try_get("id")?;
            let payload = build_payload("program_activity", sig, slot, pos, event);
            create_delivery(pool, sub_id, payload).await?;
            debug!("Queued program_activity delivery for sub {} sig={}", sub_id, sig);
        }
    }

    // token_balance_changed subscriptions — match by token account owner
    let tags: Vec<String> = serde_json::from_value(event["tags"].clone()).unwrap_or_default();
    let has_token_change = tags.iter().any(|t| t == "transfer" || t == "mint" || t == "burn");
    if has_token_change && !accounts.is_empty() {
        let subs = sqlx::query(
            r#"SELECT id, url, secret, format, event_type
               FROM webhook_subscriptions
               WHERE is_active = true AND event_type = 'token_balance_changed'
                 AND (address = ANY($1) OR owner = ANY($1))"#
        )
        .bind(&accounts as &[String])
        .fetch_all(pool)
        .await?;

        for sub in subs {
            let sub_id: uuid::Uuid = sub.try_get("id")?;
            let payload = build_payload("token_balance_changed", sig, slot, pos, event);
            create_delivery(pool, sub_id, payload).await?;
            debug!("Queued token_balance_changed delivery for sub {} sig={}", sub_id, sig);
        }
    }

    Ok(())
}

fn build_payload(event_type: &str, sig: &str, slot: i64, pos: i64, event: &Value) -> Value {
    serde_json::json!({
        "v":      "webhook.v1",
        "chain":  "x1",
        "event":  event_type,
        "cursor": format!("{}:{}", slot, pos),
        "tx": {
            "signature":    sig,
            "slot":         slot,
            "pos":          pos,
            "block_time":   event["block_time"],
            "status":       event["status"],
            "tags":         event["tags"],
            "action_types": event["action_types"],
        }
    })
}

async fn create_delivery(pool: &PgPool, sub_id: uuid::Uuid, payload: Value) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO webhook_deliveries (subscription_id, payload_json, next_attempt_at)
           VALUES ($1, $2, now())"#
    )
    .bind(sub_id)
    .bind(payload)
    .execute(pool)
    .await?;
    Ok(())
}
