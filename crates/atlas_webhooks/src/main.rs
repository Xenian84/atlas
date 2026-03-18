use anyhow::Result;
use atlas_common::{AppConfig, logging};

mod listener;
mod worker;
mod delivery;
mod matcher;

#[tokio::main]
async fn main() -> Result<()> {
    logging::init("atlas-webhooks");
    let cfg = AppConfig::from_env()?;

    let pool = sqlx::PgPool::connect(&cfg.database_url).await?;
    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let redis_mgr    = redis_client.get_connection_manager().await?;

    tracing::info!("Atlas Webhooks worker starting");

    // Run listener + delivery worker concurrently
    tokio::try_join!(
        listener::run_listener(cfg.clone(), pool.clone(), redis_mgr.clone()),
        worker::run_delivery_worker(cfg, pool),
    )?;

    Ok(())
}
