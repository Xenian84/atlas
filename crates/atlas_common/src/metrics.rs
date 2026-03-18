use anyhow::Result;
use metrics_exporter_prometheus::PrometheusBuilder;

/// Install Prometheus metrics recorder and start scrape endpoint.
pub fn install_prometheus(bind_addr: &str) -> Result<()> {
    let addr: std::net::SocketAddr = bind_addr.parse()?;
    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()?;
    tracing::info!("Prometheus metrics on http://{}/metrics", bind_addr);
    Ok(())
}

// ── Metric name constants ──────────────────────────────────────────────────────
pub const INGEST_LAG_MS:             &str = "atlas_ingest_lag_ms";
pub const TX_PER_SEC:                &str = "atlas_tx_per_sec";
pub const DB_WRITE_MS:               &str = "atlas_db_write_ms";
pub const ADDRESS_ROWS_PER_SEC:      &str = "atlas_address_rows_per_sec";
pub const ERRORS_TOTAL:              &str = "atlas_errors_total";
pub const RECONNECTS_TOTAL:          &str = "atlas_reconnects_total";
pub const WEBHOOK_DELIVERIES_TOTAL:  &str = "atlas_webhook_deliveries_total";
pub const WEBHOOK_FAILURES_TOTAL:    &str = "atlas_webhook_failures_total";
pub const INTEL_PROFILES_COMPUTED:   &str = "atlas_intel_profiles_computed";
pub const API_REQUESTS_TOTAL:        &str = "atlas_api_requests_total";
pub const CACHE_HIT_TOTAL:           &str = "atlas_cache_hit_total";
pub const CACHE_MISS_TOTAL:          &str = "atlas_cache_miss_total";
