use anyhow::Result;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use redis::aio::ConnectionManager;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::AsyncCommands;
use sqlx::PgPool;
use tracing::{info, warn};
use atlas_common::AppConfig;

use crate::{features, scores, store};

/// Debounce map: address -> last_enqueued Instant
static DEBOUNCE: std::sync::OnceLock<DashMap<String, Instant>> = std::sync::OnceLock::new();

fn debounce_map() -> &'static DashMap<String, Instant> {
    DEBOUNCE.get_or_init(DashMap::new)
}

pub async fn run_trigger(
    cfg:       AppConfig,
    pool:      PgPool,
    mut redis: ConnectionManager,
) -> Result<()> {
    let stream   = "atlas:newtx";
    let group    = "intel";
    let consumer = format!("intel-{}", gethostname());
    let cooldown = Duration::from_secs(cfg.intel_recompute_cooldown_secs);
    let windows  = cfg.intel_window_list().into_iter().map(String::from).collect::<Vec<_>>();

    // Create consumer group if it doesn't exist
    let _: redis::RedisResult<()> = redis::cmd("XGROUP")
        .arg("CREATE").arg(stream).arg(group).arg("$").arg("MKSTREAM")
        .query_async(&mut redis).await;

    info!("Intel trigger: listening on atlas:newtx as consumer {}", consumer);

    // Drain PEL from previous run
    drain_pending(&pool, &mut redis, stream, group, &consumer, &cooldown, &windows).await;

    loop {
        let opts = StreamReadOptions::default()
            .group(group, &consumer)
            .count(200)
            .block(5000);

        let reply: redis::RedisResult<StreamReadReply> = redis
            .xread_options(&[stream], &[">"], &opts)
            .await;

        let reply = match reply {
            Ok(r) => r,
            Err(e) => {
                if e.kind() == redis::ErrorKind::TypeError {
                    continue; // timeout / nil reply
                }
                warn!("Intel XREADGROUP error: {}", e);
                continue;
            }
        };

        let mut addresses: Vec<String> = vec![];
        let mut msg_ids:   Vec<String> = vec![];

        for stream_key in &reply.keys {
            for entry in &stream_key.ids {
                msg_ids.push(entry.id.clone());
                if let Some(v) = entry.map.get("data") {
                    if let Ok(data) = redis::from_redis_value::<String>(v) {
                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&data) {
                            let accs: Vec<String> = serde_json::from_value(event["accounts"].clone())
                                .unwrap_or_default();
                            addresses.extend(accs);
                        }
                    }
                }
            }
        }

        // XACK all processed messages
        for id in &msg_ids {
            let _: redis::RedisResult<()> = redis::cmd("XACK")
                .arg(stream).arg(group).arg(id.as_str())
                .query_async(&mut redis).await;
        }

        addresses.sort(); addresses.dedup();

        process_addresses(&pool, &addresses, &cooldown, &windows).await;
    }
}

async fn process_addresses(
    pool:     &PgPool,
    addresses: &[String],
    cooldown:  &Duration,
    windows:   &[String],
) {
    let debounce = debounce_map();
    for addr in addresses {
        let now = Instant::now();
        let should_process = debounce.get(addr)
            .map(|t| now.duration_since(*t) > *cooldown)
            .unwrap_or(true);

        if should_process {
            debounce.insert(addr.clone(), now);
            for window in windows {
                if let Err(e) = compute_profile(pool, addr, window).await {
                    warn!("Intel compute error addr={} window={}: {:#}", addr, window, e);
                }
            }
        }
    }
}

/// Drain pending entries from a previous run.
async fn drain_pending(
    pool:     &PgPool,
    redis:    &mut ConnectionManager,
    stream:   &str,
    group:    &str,
    consumer: &str,
    cooldown: &Duration,
    windows:  &[String],
) {
    let opts = StreamReadOptions::default()
        .group(group, consumer)
        .count(200);

    loop {
        let reply: redis::RedisResult<StreamReadReply> = redis
            .xread_options(&[stream], &["0"], &opts)
            .await;

        let reply = match reply {
            Ok(r)  => r,
            Err(_) => return,
        };

        let mut addresses: Vec<String> = vec![];
        let mut msg_ids:   Vec<String> = vec![];
        let mut any = false;

        for stream_key in &reply.keys {
            for entry in &stream_key.ids {
                any = true;
                msg_ids.push(entry.id.clone());
                if let Some(v) = entry.map.get("data") {
                    if let Ok(data) = redis::from_redis_value::<String>(v) {
                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&data) {
                            let accs: Vec<String> = serde_json::from_value(event["accounts"].clone())
                                .unwrap_or_default();
                            addresses.extend(accs);
                        }
                    }
                }
            }
        }

        for id in &msg_ids {
            let _: redis::RedisResult<()> = redis::cmd("XACK")
                .arg(stream).arg(group).arg(id.as_str())
                .query_async(redis).await;
        }

        addresses.sort(); addresses.dedup();
        process_addresses(pool, &addresses, cooldown, windows).await;

        if !any { break; }
    }
}

async fn compute_profile(pool: &PgPool, address: &str, window: &str) -> Result<()> {
    let result = features::extract(pool, address, window).await?;
    let sc     = scores::compute(&result.features);
    store::upsert_profile(pool, address, window, &result, &sc).await?;

    // Update co-occurrence edges on the 7d window pass (avoids redundant runs)
    if window == "7d" {
        if let Err(e) = store::upsert_edges(pool, address).await {
            warn!("Edge upsert failed addr={}: {:#}", address, e);
        }
    }

    metrics::counter!(atlas_common::metrics::INTEL_PROFILES_COMPUTED).increment(1);
    Ok(())
}

fn gethostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "1".to_string())
        .trim()
        .to_string()
}
