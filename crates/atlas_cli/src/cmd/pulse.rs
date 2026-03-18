//! atlas pulse — network health snapshot.

use anyhow::Result;
use chrono::{DateTime, Utc};

pub async fn run(api: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let p = super::api_get(&client, &format!("{api}/v1/network/pulse")).await?;

    let slot  = p["slot"].as_i64().unwrap_or(0);
    let bt    = p["block_time"].as_i64().unwrap_or(0);
    let tps   = p["tps_1m"].as_i64().unwrap_or(0);
    let txs   = p["indexed_txs_24h"].as_i64().unwrap_or(0);
    let wals  = p["active_wallets_24h"].as_i64().unwrap_or(0);

    let time_str = if bt > 0 {
        DateTime::<Utc>::from_timestamp(bt, 0)
            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| bt.to_string())
    } else {
        "unknown".to_string()
    };

    println!("X1 Network Pulse");
    println!("{}", "─".repeat(40));
    println!("  chain           x1");
    println!("  slot            #{slot}");
    println!("  block_time      {time_str}");
    println!("  tps (1m)        {tps} tx/s");
    println!("  txs (24h)       {txs}");
    println!("  wallets (24h)   {wals}");

    if let Some(progs) = p["top_programs"].as_array() {
        println!();
        println!("  top programs:");
        for prog in progs.iter().take(5) {
            let name  = prog["program"].as_str().unwrap_or("");
            let calls = prog["calls"].as_i64().unwrap_or(0);
            let short = if name.len() > 20 { &name[..8] } else { name };
            println!("    {short:<20} {calls} calls");
        }
    }

    if let Some(tags) = p["top_tags"].as_array() {
        println!();
        println!("  top tags:");
        for tag in tags.iter().take(5) {
            let name  = tag["tag"].as_str().unwrap_or("");
            let count = tag["count"].as_i64().unwrap_or(0);
            println!("    {name:<20} {count}");
        }
    }

    println!("{}", "─".repeat(40));
    Ok(())
}
