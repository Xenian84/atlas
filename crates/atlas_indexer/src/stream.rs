use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use dashmap::DashMap;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};
use metrics::{counter, histogram};
use sqlx::PgPool;
use redis::aio::ConnectionManager;
use atlas_common::{AppConfig, metrics as M, redis_ext};
use atlas_parser::Parser;
use atlas_types::facts::TxFactsV1;
use yellowstone_grpc_client::GeyserGrpcClient;
use crate::das;
use yellowstone_grpc_proto::geyser::{
    SubscribeRequest, SubscribeRequestFilterTransactions,
    SubscribeRequestFilterAccounts,
    SubscribeRequestPing,
    CommitmentLevel,
};
use futures::SinkExt;

use crate::{db, grpc_conv, checkpoint};

/// Single commitment stream with retry loop.
/// `commitment_str`: "processed" | "confirmed" | "finalized"
/// `redis_stream`:   Redis stream key to publish new-tx events (e.g. "atlas:newtx")
pub async fn run_stream(
    cfg:          AppConfig,
    pool:         PgPool,
    mut redis:    ConnectionManager,
    parser:       Arc<Parser>,
) -> Result<()> {
    run_stream_inner(cfg, pool, redis, parser, None, None).await
}

/// Dual-commitment stream: spawns a `processed` fast-path task alongside the
/// primary `confirmed` task. Both share the same DB pool and parser.
/// The processed task publishes to `atlas:processed`; confirmed to `atlas:newtx`.
/// The DB upsert naturally upgrades processed → confirmed via ON CONFLICT.
pub async fn run_dual_stream(
    cfg:    AppConfig,
    pool:   PgPool,
    redis:  ConnectionManager,
    parser: Arc<Parser>,
) -> Result<()> {
    info!("Starting dual-commitment stream: processed (fast-path) + confirmed (upgrade)");

    let cfg2    = cfg.clone();
    let pool2   = pool.clone();
    let parser2 = parser.clone();

    // Acquire a second independent Redis connection for the processed task
    let redis_client2 = redis::Client::open(cfg.redis_url.clone())?;
    let redis2 = redis_client2.get_connection_manager().await?;

    // Spawn processed fast-path (non-critical — if it fails, confirmed still runs)
    tokio::spawn(async move {
        if let Err(e) = run_stream_inner(
            cfg2, pool2, redis2, parser2,
            Some("processed"),
            Some("atlas:processed"),
        ).await {
            error!("Processed fast-path stream exited: {:#}", e);
        }
    });

    // Primary confirmed stream (blocking — this is the source of truth)
    run_stream_inner(cfg, pool, redis, parser, Some("confirmed"), Some("atlas:newtx")).await
}

async fn run_stream_inner(
    cfg:          AppConfig,
    pool:         PgPool,
    mut redis:    ConnectionManager,
    parser:       Arc<Parser>,
    commitment_override: Option<&'static str>,
    redis_stream_override: Option<&'static str>,
) -> Result<()> {
    let commitment_str  = commitment_override.unwrap_or(&cfg.indexer_commitment);
    let redis_stream    = redis_stream_override.unwrap_or("atlas:newtx");
    let mut backoff_secs = 1u64;
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()?;

    loop {
        match stream_once(&cfg, &pool, &mut redis, &parser, &http, commitment_str, redis_stream).await {
            Ok(()) => {
                info!("[{}] Stream ended cleanly, reconnecting...", commitment_str);
            }
            Err(e) => {
                error!("[{}] Stream error: {:#}, reconnecting in {}s", commitment_str, e, backoff_secs);
                counter!(M::ERRORS_TOTAL).increment(1);
                counter!(M::RECONNECTS_TOTAL).increment(1);
                sleep(Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(60);
                continue;
            }
        }
        backoff_secs = 1;
    }
}

async fn stream_once(
    cfg:            &AppConfig,
    pool:           &PgPool,
    redis:          &mut ConnectionManager,
    parser:         &Arc<Parser>,
    http:           &reqwest::Client,
    commitment_str: &str,
    redis_stream:   &str,
) -> Result<()> {
    let mut client = GeyserGrpcClient::build_from_shared(cfg.yellowstone_grpc_endpoint.clone())?
        .x_token(cfg.yellowstone_grpc_x_token.clone())?
        .connect()
        .await?;

    info!("[{}] Connected to Yellowstone gRPC at {}", commitment_str, cfg.yellowstone_grpc_endpoint);

    let commitment = match commitment_str {
        "processed" => CommitmentLevel::Processed,
        "finalized" => CommitmentLevel::Finalized,
        _           => CommitmentLevel::Confirmed,
    };

    let mut tx_filters = HashMap::new();
    tx_filters.insert(
        "all".to_string(),
        SubscribeRequestFilterTransactions {
            vote:             Some(false), // exclude vote txs
            failed:           None,        // None = include ALL (success + failed)
            signature:        None,
            account_include:  vec![],
            account_exclude:  vec![],
            account_required: vec![],
        },
    );

    // No separate account subscription — we extract post_balances directly from
    // the confirmed transaction stream (see UpdateOneof::Transaction handler below).
    // This avoids the multi-million account startup dump that would flood gRPC.

    let request = SubscribeRequest {
        transactions: tx_filters,
        blocks_meta:  {
            let mut m = HashMap::new();
            m.insert("all".to_string(), Default::default());
            m
        },
        commitment: Some(commitment as i32),
        ..Default::default()
    };

    // Keep grpc_sink alive for the lifetime of the stream; we also use it to
    // respond to server Ping messages (required by Yellowstone v7+).
    let (mut grpc_sink, mut stream) = client.subscribe_with_request(Some(request)).await?;

    // Slot -> block_time cache (populated from BlockMeta updates)
    let block_times: DashMap<u64, i64> = DashMap::new();

    let mut pos:       u32 = 0;
    let mut last_slot: u64 = 0;

    use futures::StreamExt;
    while let Some(msg) = stream.next().await {
        let update = match msg {
            Ok(u)  => u,
            Err(e) => {
                warn!("Stream message error: {}", e);
                break;
            }
        };

        use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
        match update.update_oneof {
            // Yellowstone v7+ requires client to respond to server Pings.
            // Without this the server stops delivering transaction events.
            Some(UpdateOneof::Ping(_)) => {
                if let Err(e) = grpc_sink.send(SubscribeRequest {
                    ping: Some(SubscribeRequestPing { id: 1 }),
                    ..Default::default()
                }).await {
                    warn!("[{}] Failed to send pong: {}", commitment_str, e);
                    break;
                }
                continue;
            }
            Some(UpdateOneof::Pong(_)) => continue,
            Some(UpdateOneof::Account(_)) => continue,
            Some(UpdateOneof::BlockMeta(bm)) => {
                if let Some(t) = bm.block_time {
                    let ts = t.timestamp;
                    let slot = bm.slot;
                    block_times.insert(slot, ts);
                    // Retroactively patch any txs we stored before BlockMeta arrived.
                    // This happens during catchup: tx arrives before BlockMeta for same slot.
                    if let Err(e) = sqlx::query(
                        "UPDATE tx_store SET block_time = $1 WHERE slot = $2 AND block_time IS NULL"
                    )
                    .bind(ts)
                    .bind(slot as i64)
                    .execute(pool)
                    .await {
                        warn!("[{}] BlockMeta back-patch failed slot {}: {}", commitment_str, slot, e);
                    }
                }
                continue;
            }
            Some(UpdateOneof::Transaction(tx_update)) => {
                let slot = tx_update.slot;
                if slot != last_slot {
                    pos = 0;
                    last_slot = slot;
                }

                let tx_info = match &tx_update.transaction {
                    Some(t) => t,
                    None => {
                        warn!("Transaction update missing transaction field at slot {}", slot);
                        continue;
                    }
                };

                // Account balances are now kept current by the atlas-geyser
                // plugin running inside the validator — no extraction needed here.

                // block_time comes from the BlockMeta cache.
                // If BlockMeta hasn't arrived yet for this slot (catchup race),
                // we store NULL and the BlockMeta handler retroactively patches it.
                let block_time = block_times.get(&slot).map(|t| *t);

                let raw = match grpc_conv::convert_grpc_tx(slot, pos, tx_info, block_time) {
                    Ok(Some(r)) => r,
                    Ok(None)    => continue, // vote tx
                    Err(e) => {
                        warn!("Failed to convert gRPC tx: {}", e);
                        counter!(M::ERRORS_TOTAL).increment(1);
                        continue;
                    }
                };

                pos += 1;

                let start = std::time::Instant::now();
                let mut facts = parser.parse(&raw);
                // Set commitment from the actual stream level so DB reflects correct state.
                facts.commitment = match commitment_str {
                    "processed" => atlas_types::facts::Commitment::Processed,
                    "finalized" => atlas_types::facts::Commitment::Finalized,
                    _           => atlas_types::facts::Commitment::Confirmed,
                };

                if let Err(e) = persist_tx(pool, redis, &facts, http, &cfg.validator_rpc_url, redis_stream).await {
                    error!("[{}] Failed to persist tx {}: {:#}", commitment_str, facts.sig, e);
                    counter!(M::ERRORS_TOTAL).increment(1);
                } else {
                    let elapsed_ms = start.elapsed().as_millis() as f64;
                    histogram!(M::DB_WRITE_MS).record(elapsed_ms);
                    if let Some(bt) = facts.block_time {
                        let lag = chrono::Utc::now().timestamp() - bt;
                        histogram!(M::INGEST_LAG_MS).record((lag * 1000) as f64);
                    }
                    counter!(M::TX_PER_SEC).increment(1);

                    if let Err(e) = checkpoint::maybe_update(pool, slot, commitment_str).await {
                        warn!("[{}] Checkpoint update failed at slot {}: {:#}", commitment_str, slot, e);
                    }
                }
            }
            _ => continue,
        }
    }

    Ok(())
}

pub async fn persist_tx_static(pool: &PgPool, redis: &mut ConnectionManager, facts: &TxFactsV1) -> Result<()> {
    let http = reqwest::Client::new();
    persist_tx(pool, redis, facts, &http, "http://127.0.0.1:8899", "atlas:newtx").await
}

async fn persist_tx(
    pool:         &PgPool,
    redis:        &mut ConnectionManager,
    facts:        &TxFactsV1,
    http:         &reqwest::Client,
    rpc_url:      &str,
    redis_stream: &str,
) -> Result<()> {
    db::upsert_tx(pool, facts).await?;
    db::upsert_address_index_batch(pool, facts).await?;
    db::upsert_token_balance_index(pool, facts).await?;
    db::upsert_program_activity(pool, facts).await?;

    // Account state — update XNT balance for all accounts touched by this tx.
    // Non-fatal: missing an update just means a slightly stale balance.
    if let Err(e) = db::upsert_account_balances(pool, facts).await {
        warn!("account balance upsert failed: {}", e);
    }

    // Token account ownership — track which mint each wallet holds.
    if let Err(e) = db::upsert_token_account_index(pool, facts).await {
        warn!("token_account_index upsert failed: {}", e);
    }

    // DAS indexer + token metadata — non-fatal, only run on confirmed path
    if redis_stream == "atlas:newtx" {
        if let Err(e) = das::index_assets(pool, http, rpc_url, facts).await {
            warn!("DAS indexer error: {}", e);
        }
        // Ensure token metadata is cached for every mint touched by this tx
        let mints: Vec<String> = facts.token_deltas.iter()
            .map(|d| d.mint.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        if !mints.is_empty() {
            if let Err(e) = crate::token_meta::ensure_token_metadata(pool, http, rpc_url, &mints).await {
                warn!("token_meta resolver error: {}", e);
            }
            // Refresh supply for mints that had significant delta (mint/burn)
            let mint_burn_mints: Vec<String> = facts.token_deltas.iter()
                .filter(|d| {
                    let delta: i128 = d.delta.parse().unwrap_or(0);
                    delta.abs() > 1_000_000 // significant change = likely mint or burn
                })
                .map(|d| d.mint.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            if !mint_burn_mints.is_empty() {
                crate::token_meta::refresh_token_supply(pool, http, rpc_url, &mint_burn_mints).await;
            }
        }
    }

    let event = serde_json::json!({
        "sig":          facts.sig,
        "slot":         facts.slot,
        "pos":          facts.pos,
        "block_time":   facts.block_time,
        "commitment":   facts.commitment.as_str(),
        "status":       if facts.is_success() { "success" } else { "failed" },
        "tags":         facts.tags,
        "action_types": facts.action_types(),
        "accounts":     facts.all_addresses().into_iter().take(10).collect::<Vec<_>>(),
        "programs":     facts.programs.iter().take(5).collect::<Vec<_>>(),
    });
    redis_ext::xadd_json(redis, redis_stream, &event).await?;

    // Webhook dispatch — only on confirmed path, non-fatal
    if redis_stream == "atlas:newtx" {
        if let Err(e) = crate::webhook_worker::trigger_webhooks(pool, facts).await {
            warn!("webhook trigger error: {}", e);
        }
    }

    Ok(())
}

// Account state is now kept current by the atlas-geyser plugin loaded
// directly into the validator. No flush_accounts needed here.

