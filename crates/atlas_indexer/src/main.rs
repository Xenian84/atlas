use anyhow::Result;
use clap::{Parser as ClapParser, Subcommand};
use atlas_common::{AppConfig, logging};
use atlas_types::facts::Commitment;

mod db;
mod stream;
mod backfill;
mod grpc_conv;
mod checkpoint;
mod das;
mod shred_consumer;
mod token_meta;
mod gap_detector;
mod webhook_worker;

#[derive(ClapParser)]
#[command(name = "atlas-indexer", about = "Atlas blockchain indexer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Stream live transactions from Yellowstone gRPC
    Stream,
    /// Backfill historical slots via RPC
    Backfill {
        #[arg(long)]
        from_slot: u64,
        #[arg(long)]
        to_slot:   u64,
        #[arg(long, default_value = "10")]
        batch:     usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    logging::init("atlas-indexer");
    let cfg = AppConfig::from_env()?;
    let cli = Cli::parse();

    atlas_common::metrics::install_prometheus(&cfg.indexer_metrics_bind)?;

    let pool = sqlx::PgPool::connect(&cfg.database_url).await?;

    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let redis_mgr    = redis_client.get_connection_manager().await?;

    // Resolve config file paths — use env overrides with fallback + warning
    let programs_path = std::env::var("ATLAS_PROGRAMS_CONFIG")
        .unwrap_or_else(|_| "config/programs.yml".to_string());
    let programs_cfg = atlas_parser::ProgramsConfig::from_yaml(&programs_path)
        .unwrap_or_else(|e| {
            tracing::warn!("Could not load programs config from '{}': {}. Using defaults.", programs_path, e);
            Default::default()
        });

    let spam_path = std::env::var("ATLAS_SPAM_CONFIG")
        .unwrap_or_else(|_| "config/spam.yml".to_string());
    let spam_cfg = atlas_parser::spam::SpamConfig::from_yaml(&spam_path)
        .unwrap_or_else(|e| {
            tracing::warn!("Could not load spam config from '{}': {}. Spam detection disabled.", spam_path, e);
            atlas_parser::spam::SpamConfig::empty()
        });

    // Thread commitment level from config into the parser
    let commitment = match cfg.indexer_commitment.as_str() {
        "processed" => Commitment::Processed,
        "finalized" => Commitment::Finalized,
        _           => Commitment::Confirmed,
    };

    let parser = std::sync::Arc::new(atlas_parser::Parser::new(programs_cfg, spam_cfg, commitment));

    match cli.command {
        Command::Stream => {
            // ── Atlas ShredStream consumer — sub-50ms path ────────────────
            // Concurrent task: reads atlas:shreds (written by atlas-shredstream
            // binary) and inserts shred-level records before confirmed fires.
            let shred_stream_key = std::env::var("ATLAS_SHRED_STREAM")
                .unwrap_or_else(|_| "atlas:shreds".to_string());
            let tx_broadcast_key = std::env::var("REDIS_NEWTX_STREAM")
                .unwrap_or_else(|_| "atlas:newtx".to_string());
            // IMPORTANT: shred consumer must use its OWN Redis client (separate TCP connection).
            // redis::ConnectionManager::clone() shares one underlying TCP connection, and the
            // shred consumer's XREADGROUP BLOCK 1000 command holds that connection for up to 1s,
            // which would delay every XADD the main stream tries to send.
            let shred_redis_client = redis::Client::open(cfg.redis_url.clone())?;
            let shred_pool  = pool.clone();
            let shred_redis = shred_redis_client.get_connection_manager().await?;
            tokio::spawn(async move {
                loop {
                    if let Err(e) = shred_consumer::run_shred_consumer(
                        shred_redis.clone(),
                        shred_pool.clone(),
                        shred_stream_key.clone(),
                        tx_broadcast_key.clone(),
                    ).await {
                        tracing::warn!("Shred consumer exited ({}), restarting in 5s", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            });
            // ─────────────────────────────────────────────────────────────

            // ── Gap detector — scans for slot gaps every 5 minutes ────────
            let gap_pool = pool.clone();
            let gap_rpc  = cfg.validator_rpc_url.clone();
            tokio::spawn(async move {
                gap_detector::run_gap_detector(gap_pool, gap_rpc).await;
            });

            // ── Webhook delivery worker ───────────────────────────────────
            let wh_pool = pool.clone();
            tokio::spawn(async move {
                webhook_worker::run_webhook_worker(wh_pool).await;
            });

            // ── Token metadata resolver — picks up new mints discovered by ──
            // the atlas-geyser plugin via token_owner_map, resolves their
            // name/symbol from Token-2022 extensions or Metaplex, and caches
            // in token_metadata. Runs every 60 seconds.
            let tm_pool   = pool.clone();
            let tm_rpc    = cfg.validator_rpc_url.clone();
            tokio::spawn(async move {
                token_meta::run_mint_resolver(tm_pool, tm_rpc).await;
            });

            if cfg.indexer_dual_stream {
                tracing::info!("Starting dual-commitment stream (processed fast-path + confirmed) from {}",
                    cfg.yellowstone_grpc_endpoint);
                stream::run_dual_stream(cfg, pool, redis_mgr, parser).await?;
            } else {
                tracing::info!("Starting live stream ({}) from {}",
                    cfg.indexer_commitment, cfg.yellowstone_grpc_endpoint);
                stream::run_stream(cfg, pool, redis_mgr, parser).await?;
            }
        }
        Command::Backfill { from_slot, to_slot, batch } => {
            tracing::info!("Backfilling slots {}..{}", from_slot, to_slot);
            backfill::run_backfill(cfg, pool, redis_mgr, parser, from_slot, to_slot, batch).await?;
        }
    }
    Ok(())
}
