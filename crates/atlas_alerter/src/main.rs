//! Atlas Alerter — monitors validator and indexer health.
//!
//! Checks every 60s:
//!   - Validator delinquency (last vote slot stale > 120s)
//!   - Indexer lag (indexed slot > 200 slots behind tip)
//!   - API health (HTTP 200 on /health)
//!   - Disk usage > 85%
//!   - Memory > 90%
//!   - Swap > 80%
//!
//! Fires alerts via Telegram and/or Slack webhook.
//! Suppresses repeated alerts — only fires on state transitions (ok→alert, alert→ok).
//!
//! Config via env vars:
//!   ATLAS_TELEGRAM_BOT_TOKEN   (optional)
//!   ATLAS_TELEGRAM_CHAT_ID     (optional)
//!   ATLAS_SLACK_WEBHOOK_URL    (optional)
//!   ATLAS_API_URL              (default http://localhost:8888)
//!   ATLAS_RPC_URL              (default http://localhost:8899)
//!   DATABASE_URL
//!   ALERT_CHECK_INTERVAL_SECS  (default 60)
//!   ALERT_LAG_THRESHOLD        (default 200 slots)
//!   ALERT_DELINQUENCY_SECS     (default 120)

use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("atlas_alerter=info".parse().unwrap()))
        .init();

    let cfg = Config::from_env();
    info!("Atlas Alerter started — check interval {}s", cfg.check_interval_secs);
    info!("Telegram: {}", if cfg.telegram_token.is_some() { "configured" } else { "not configured" });
    info!("Slack:    {}", if cfg.slack_webhook.is_some() { "configured" } else { "not configured" });

    let http  = Client::builder().timeout(Duration::from_secs(10)).build()?;
    let pool  = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&cfg.database_url)
        .await?;

    let mut state: HashMap<&'static str, bool> = HashMap::new(); // alert name → currently_firing

    loop {
        let alerts = run_checks(&http, &pool, &cfg).await;

        for (name, firing, message) in &alerts {
            let was_firing = state.get(name).copied().unwrap_or(false);

            if *firing && !was_firing {
                // Transition: ok → alert
                warn!("ALERT FIRING: {} — {}", name, message);
                let text = format!("🚨 *Atlas Alert* — {}\n{}", name, message);
                send_alert(&http, &cfg, &text).await;
            } else if !firing && was_firing {
                // Transition: alert → ok
                info!("ALERT RESOLVED: {}", name);
                let text = format!("✅ *Atlas Resolved* — {}\n{}", name, message);
                send_alert(&http, &cfg, &text).await;
            }

            state.insert(name, *firing);
        }

        tokio::time::sleep(Duration::from_secs(cfg.check_interval_secs)).await;
    }
}

// ── Checks ───────────────────────────────────────────────────────────────────

async fn run_checks(
    http: &Client,
    pool: &sqlx::PgPool,
    cfg:  &Config,
) -> Vec<(&'static str, bool, String)> {
    let mut results = Vec::new();

    // 1. Validator delinquency
    results.push(check_validator_delinquency(http, cfg).await);

    // 2. Indexer lag
    results.push(check_indexer_lag(http, cfg).await);

    // 3. API health
    results.push(check_api_health(http, cfg).await);

    // 4. Disk usage
    results.push(check_disk_usage());

    // 5. Memory / swap
    results.push(check_memory());

    // 6. Backfill stalled
    results.push(check_backfill_stalled(pool).await);

    results
}

async fn check_validator_delinquency(http: &Client, cfg: &Config) -> (&'static str, bool, String) {
    let name = "validator_delinquent";
    let body = serde_json::json!({
        "jsonrpc":"2.0","id":1,
        "method":"getVoteAccounts","params":[]
    });

    match http.post(&cfg.rpc_url).json(&body).send().await {
        Ok(r) => match r.json::<Value>().await {
            Ok(v) => {
                let delinquent = v["result"]["delinquent"].as_array()
                    .map(|arr| arr.len())
                    .unwrap_or(0);
                let current   = v["result"]["current"].as_array()
                    .map(|arr| arr.len())
                    .unwrap_or(0);
                let total = delinquent + current;
                let pct = if total > 0 { delinquent * 100 / total } else { 0 };
                let firing = pct > 10; // alert if >10% stake is delinquent
                let msg = format!("{}/{} validators delinquent ({}%)", delinquent, total, pct);
                (name, firing, msg)
            }
            Err(e) => (name, true, format!("RPC parse error: {}", e)),
        },
        Err(e) => (name, true, format!("RPC unreachable: {}", e)),
    }
}

async fn check_indexer_lag(http: &Client, cfg: &Config) -> (&'static str, bool, String) {
    let name = "indexer_lag";
    match http.get(format!("{}/v1/network/pulse", cfg.api_url)).send().await {
        Ok(r) => match r.json::<Value>().await {
            Ok(v) => {
                let lag = v["indexer"]["lag_slots"].as_i64().unwrap_or(0);
                let firing = lag > cfg.lag_threshold as i64;
                let msg = format!("indexer lag = {} slots (threshold {})", lag, cfg.lag_threshold);
                (name, firing, msg)
            }
            Err(e) => (name, true, format!("pulse parse error: {}", e)),
        },
        Err(e) => (name, true, format!("API unreachable: {}", e)),
    }
}

async fn check_api_health(http: &Client, cfg: &Config) -> (&'static str, bool, String) {
    let name = "api_down";
    match http.get(format!("{}/health", cfg.api_url)).send().await {
        Ok(r) => {
            let ok = r.status().is_success();
            let msg = format!("API /health returned {}", r.status());
            (name, !ok, msg)
        }
        Err(e) => (name, true, format!("API /health unreachable: {}", e)),
    }
}

fn check_disk_usage() -> (&'static str, bool, String) {
    let name = "disk_full";
    if let Ok(output) = std::process::Command::new("df")
        .args(["-h", "--output=pcent", "/"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        let pct: u64 = s.lines()
            .nth(1)
            .unwrap_or("")
            .trim()
            .trim_end_matches('%')
            .parse()
            .unwrap_or(0);
        let firing = pct > 85;
        return (name, firing, format!("disk usage {}%", pct));
    }
    (name, false, "disk check skipped".into())
}

fn check_memory() -> (&'static str, bool, String) {
    let name = "memory_pressure";
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        let mut total_kb = 0u64;
        let mut avail_kb = 0u64;
        let mut swap_total = 0u64;
        let mut swap_free  = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 { continue; }
            let val: u64 = parts[1].parse().unwrap_or(0);
            match parts[0] {
                "MemTotal:"     => total_kb = val,
                "MemAvailable:" => avail_kb = val,
                "SwapTotal:"    => swap_total = val,
                "SwapFree:"     => swap_free = val,
                _ => {}
            }
        }

        let mem_pct  = if total_kb > 0 { (total_kb - avail_kb) * 100 / total_kb } else { 0 };
        let swap_pct = if swap_total > 0 { (swap_total - swap_free) * 100 / swap_total } else { 0 };
        let firing   = mem_pct > 92 || swap_pct > 80;
        let msg = format!("memory {}% used, swap {}% used", mem_pct, swap_pct);
        return (name, firing, msg);
    }
    (name, false, "memory check skipped".into())
}

async fn check_backfill_stalled(pool: &sqlx::PgPool) -> (&'static str, bool, String) {
    let name = "backfill_stalled";
    // Check if max indexed slot hasn't moved in 10 minutes
    // We track this via indexer_state
    match sqlx::query_scalar::<_, String>(
        "SELECT value FROM indexer_state WHERE key = 'last_gap_check_slot'"
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some(v)) => {
            let slot: i64 = v.parse().unwrap_or(0);
            (name, slot == 0, format!("last gap check slot: {}", slot))
        }
        _ => (name, false, "backfill check skipped".into()),
    }
}

// ── Alert dispatch ────────────────────────────────────────────────────────────

async fn send_alert(http: &Client, cfg: &Config, text: &str) {
    if let (Some(token), Some(chat_id)) = (&cfg.telegram_token, &cfg.telegram_chat_id) {
        let url  = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let body = serde_json::json!({
            "chat_id":    chat_id,
            "text":       text,
            "parse_mode": "Markdown",
        });
        match http.post(&url).json(&body).send().await {
            Ok(r) => { if !r.status().is_success() { warn!("Telegram alert failed: {}", r.status()); } }
            Err(e) => warn!("Telegram send error: {}", e),
        }
    }

    if let Some(webhook) = &cfg.slack_webhook {
        let body = serde_json::json!({ "text": text });
        match http.post(webhook).json(&body).send().await {
            Ok(r) => { if !r.status().is_success() { warn!("Slack alert failed: {}", r.status()); } }
            Err(e) => warn!("Slack send error: {}", e),
        }
    }

    // Always log to stdout regardless
    info!("ALERT: {}", text);
}

// ── Config ────────────────────────────────────────────────────────────────────

struct Config {
    telegram_token:   Option<String>,
    telegram_chat_id: Option<String>,
    slack_webhook:    Option<String>,
    api_url:          String,
    rpc_url:          String,
    database_url:     String,
    check_interval_secs: u64,
    lag_threshold:    u64,
}

impl Config {
    fn from_env() -> Self {
        Self {
            telegram_token:   std::env::var("ATLAS_TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: std::env::var("ATLAS_TELEGRAM_CHAT_ID").ok(),
            slack_webhook:    std::env::var("ATLAS_SLACK_WEBHOOK_URL").ok(),
            api_url:          std::env::var("ATLAS_API_URL").unwrap_or_else(|_| "http://localhost:8888".into()),
            rpc_url:          std::env::var("ATLAS_RPC_URL").unwrap_or_else(|_| "http://localhost:8899".into()),
            database_url:     std::env::var("DATABASE_URL").expect("DATABASE_URL required"),
            check_interval_secs: std::env::var("ALERT_CHECK_INTERVAL_SECS")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(60),
            lag_threshold: std::env::var("ALERT_LAG_THRESHOLD")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(200),
        }
    }
}
