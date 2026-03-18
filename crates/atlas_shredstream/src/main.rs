//! Atlas ShredStream — sub-50ms transaction visibility.
//!
//! Listens on a Unix domain socket for entry batches forwarded by the tachyon
//! validator bridge (atlas_bridge.rs). Decodes entries from bincode, extracts
//! transactions, and publishes compact events to the Redis `atlas:shreds` stream
//! so atlas_indexer can index them before confirmed commitment fires.
//!
//! Data flow:
//!   tachyon validator (window_service / replay)
//!     → Unix socket /tmp/atlas-bridge.sock
//!     → atlas-shredstream (this binary)
//!     → Redis XADD atlas:shreds
//!     → atlas_indexer shred consumer task
//!     → DB (commitment = "processed_shred")

use anyhow::Result;
use atlas_common::{AppConfig, logging};
use tracing::info;

mod frame;
mod receiver;
mod publisher;

#[tokio::main]
async fn main() -> Result<()> {
    logging::init("atlas-shredstream");
    let cfg = AppConfig::from_env()?;

    let socket_path = std::env::var("ATLAS_BRIDGE_SOCKET")
        .unwrap_or_else(|_| "/tmp/atlas-bridge.sock".to_string());

    let redis_stream = std::env::var("ATLAS_SHRED_STREAM")
        .unwrap_or_else(|_| "atlas:shreds".to_string());

    info!("Atlas ShredStream starting");
    info!("  Socket  : {}", socket_path);
    info!("  Redis   : {}", cfg.redis_url);
    info!("  Stream  : {}", redis_stream);

    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let redis_mgr    = redis_client.get_connection_manager().await?;

    // Receive entry batches from the validator bridge and publish to Redis
    receiver::run_receiver(socket_path, redis_mgr, redis_stream).await
}
