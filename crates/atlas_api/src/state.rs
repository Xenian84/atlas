use std::sync::Arc;
use sqlx::PgPool;
use redis::aio::ConnectionManager;
use tokio::sync::broadcast;
use atlas_common::AppConfig;
use crate::broadcast::TxEvent;

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

struct Inner {
    pub cfg:         AppConfig,
    pub pool:        PgPool,
    pub redis:       ConnectionManager,
    pub http:        reqwest::Client,
    pub tx_broadcast: broadcast::Sender<TxEvent>,
}

impl AppState {
    pub fn new(
        cfg:         AppConfig,
        pool:        PgPool,
        redis:       ConnectionManager,
        tx_broadcast: broadcast::Sender<TxEvent>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");
        Self(Arc::new(Inner { cfg, pool, redis, http, tx_broadcast }))
    }

    pub fn cfg(&self)          -> &AppConfig               { &self.0.cfg }
    pub fn pool(&self)         -> &PgPool                  { &self.0.pool }
    pub fn redis(&self)        -> ConnectionManager        { self.0.redis.clone() }
    pub fn http(&self)         -> &reqwest::Client         { &self.0.http }
    pub fn tx_broadcast(&self) -> broadcast::Sender<TxEvent> { self.0.tx_broadcast.clone() }
}

/// Type alias for the authenticated API key hash injected by auth_middleware.
#[derive(Clone)]
pub struct ApiKeyHash(pub String);
