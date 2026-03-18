use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::AsyncCommands;
use sqlx::PgPool;
use tracing::{info, warn, error};
use atlas_common::AppConfig;

use crate::matcher;

/// Listen on atlas:newtx Redis stream and create webhook_deliveries rows.
pub async fn run_listener(
    _cfg:  AppConfig,
    pool:  PgPool,
    mut redis: ConnectionManager,
) -> Result<()> {
    let stream   = "atlas:newtx";
    let group    = "webhooks";
    let consumer = format!("webhooks-{}", gethostname());

    // Create consumer group if it doesn't exist (use $ so we start from now on first run)
    let _: redis::RedisResult<()> = redis::cmd("XGROUP")
        .arg("CREATE").arg(stream).arg(group).arg("$").arg("MKSTREAM")
        .query_async(&mut redis).await;

    info!("Webhook listener: reading from {} as consumer {}", stream, consumer);

    // First, drain the PEL (pending entries from a previous consumer incarnation)
    drain_pending(&pool, &mut redis, stream, group, &consumer).await;

    loop {
        // Read new messages (">")
        let opts = StreamReadOptions::default()
            .group(group, &consumer)
            .count(100)
            .block(5000);

        let reply: redis::RedisResult<StreamReadReply> = redis
            .xread_options(&[stream], &[">"], &opts)
            .await;

        let reply = match reply {
            Ok(r)  => r,
            Err(e) => {
                // XREADGROUP returns Nil when the blocking read times out (no messages)
                if e.kind() == redis::ErrorKind::TypeError {
                    continue;
                }
                warn!("XREADGROUP error: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        for stream_key in &reply.keys {
            for entry in &stream_key.ids {
                let data = match entry.map.get("data") {
                    Some(v) => match redis::from_redis_value::<String>(v) {
                        Ok(s)  => s,
                        Err(e) => { warn!("Bad stream entry data: {}", e); continue; }
                    },
                    None => { warn!("Stream entry missing 'data' field: {:?}", entry.id); continue; }
                };

                let event: serde_json::Value = match serde_json::from_str(&data) {
                    Ok(v)  => v,
                    Err(e) => { warn!("Bad event JSON: {}", e); ack(&mut redis, stream, group, &entry.id).await; continue; }
                };

                if let Err(e) = matcher::process_event(&pool, &event).await {
                    error!("Webhook matching error: {:#}", e);
                }

                ack(&mut redis, stream, group, &entry.id).await;
            }
        }
    }
}

/// Drain pending entries (PEL) from a previous run before switching to ">".
async fn drain_pending(
    pool:     &PgPool,
    redis:    &mut ConnectionManager,
    stream:   &str,
    group:    &str,
    consumer: &str,
) {
    let opts = StreamReadOptions::default()
        .group(group, consumer)
        .count(100);

    loop {
        let reply: redis::RedisResult<StreamReadReply> = redis
            .xread_options(&[stream], &["0"], &opts)
            .await;

        let reply = match reply {
            Ok(r)  => r,
            Err(_) => return,
        };

        let mut any = false;
        for stream_key in &reply.keys {
            for entry in &stream_key.ids {
                any = true;
                if let Some(v) = entry.map.get("data") {
                    if let Ok(data) = redis::from_redis_value::<String>(v) {
                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&data) {
                            let _ = matcher::process_event(pool, &event).await;
                        }
                    }
                }
                ack(redis, stream, group, &entry.id).await;
            }
        }

        if !any { break; }
    }
}

async fn ack(redis: &mut ConnectionManager, stream: &str, group: &str, id: &str) {
    let _: redis::RedisResult<()> = redis::cmd("XACK")
        .arg(stream).arg(group).arg(id)
        .query_async(redis).await;
}

fn gethostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "1".to_string())
        .trim()
        .to_string()
}
