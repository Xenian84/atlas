//! atlas block <slot> — block overview.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

pub async fn run(api: &str, slot: u64, key: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let b: serde_json::Value = client
        .get(format!("{api}/v1/block/{slot}"))
        .header("X-Api-Key", key)
        .send()
        .await
        .with_context(|| format!("GET {api}/v1/block/{slot}"))?
        .json()
        .await?;

    if let Some(err) = b["error"].as_str() {
        println!("Block #{slot}: {err}");
        return Ok(());
    }

    let tx_count     = b["tx_count"].as_i64().unwrap_or(0);
    let success      = b["success_count"].as_i64().unwrap_or(0);
    let failed       = b["failed_count"].as_i64().unwrap_or(0);
    let total_fees   = b["total_fees"].as_i64().unwrap_or(0);
    let block_time   = b["block_time"].as_i64();

    let time_str = block_time
        .and_then(|t| DateTime::<Utc>::from_timestamp(t, 0))
        .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("Block #{slot}");
    println!("{}", "─".repeat(60));
    println!("  time          {time_str}");
    println!("  transactions  {tx_count}  ({success} ok / {failed} failed)");
    println!("  total_fees    {} lamports", total_fees);

    if let Some(programs) = b["programs"].as_array() {
        println!();
        println!("  programs ({}):", programs.len());
        for prog in programs.iter().take(6) {
            let p = prog.as_str().unwrap_or("?");
            let short = if p.len() > 20 { &p[..8] } else { p };
            println!("    {short}…");
        }
    }

    if let Some(txs) = b["transactions"].as_array() {
        println!();
        println!("  transactions (first {}):", txs.len().min(10));
        for tx in txs.iter().take(10) {
            let sig    = tx["sig"].as_str().unwrap_or("?");
            let status = tx["status"].as_str().unwrap_or("?");
            let fee    = tx["fee_lamports"].as_i64().unwrap_or(0);
            let short  = if sig.len() > 20 { &sig[..20] } else { sig };
            println!("    {short}…  [{status}]  {fee}L");
        }
    }

    println!("{}", "─".repeat(60));
    Ok(())
}
