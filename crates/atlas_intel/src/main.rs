use anyhow::Result;
use atlas_common::{AppConfig, logging};

mod trigger;
mod features;
mod scores;
mod store;

#[tokio::main]
async fn main() -> Result<()> {
    logging::init("atlas-intel");
    let cfg = AppConfig::from_env()?;

    let pool = sqlx::PgPool::connect(&cfg.database_url).await?;
    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let redis_mgr    = redis_client.get_connection_manager().await?;

    tracing::info!("Atlas Intelligence worker starting");

    trigger::run_trigger(cfg, pool, redis_mgr).await?;
    Ok(())
}
