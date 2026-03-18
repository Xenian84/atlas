use anyhow::Result;
use atlas_common::{AppConfig, logging};

mod state;
mod middleware;
mod negotiate;
mod handlers;
mod rpc;
mod error;
mod explain;
mod broadcast;

pub use state::AppState;
pub use error::ApiError;

#[tokio::main]
async fn main() -> Result<()> {
    logging::init("atlas-api");
    let cfg = AppConfig::from_env()?;

    let pool = sqlx::PgPool::connect(&cfg.database_url).await?;
    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let redis_mgr    = redis_client.get_connection_manager().await?;

    // Broadcast channel for live WebSocket stream
    let tx_broadcast = broadcast::create_broadcast();
    broadcast::start_reader(redis_mgr.clone(), tx_broadcast.clone());

    let state = AppState::new(cfg.clone(), pool, redis_mgr, tx_broadcast);

    let app = handlers::build_router(state);

    let addr: std::net::SocketAddr = cfg.api_bind.parse()?;
    tracing::info!("Atlas API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
