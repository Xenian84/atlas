//! atlas status — full system health check.

use anyhow::Result;
use redis::Commands;

pub async fn run(api: &str, redis_url: &str) -> Result<()> {
    println!("Atlas System Status");
    println!("{}", "─".repeat(50));

    // ── API health ──────────────────────────────────────────────────────────
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()?;

    print!("  atlas-api       ");
    match client.get(format!("{api}/v1/network/pulse"))
        .header("Accept", "application/json")
        .send().await
    {
        Ok(r) if r.status().is_success() => println!("✓  running  ({api})"),
        Ok(r)  => println!("✗  HTTP {}", r.status()),
        Err(e) => println!("✗  unreachable — {e}"),
    }

    // ── Pulse data ──────────────────────────────────────────────────────────
    match super::api_get(&client, &format!("{api}/v1/network/pulse")).await {
        Ok(p) => {
            let slot  = p["slot"].as_i64().unwrap_or(0);
            let tps   = p["tps_1m"].as_i64().unwrap_or(0);
            let txs   = p["indexed_txs_24h"].as_i64().unwrap_or(0);
            let wals  = p["active_wallets_24h"].as_i64().unwrap_or(0);
            println!("  slot            ✓  #{slot}");
            println!("  tps_1m          ✓  {tps} tx/s");
            println!("  indexed_24h     ✓  {txs} transactions");
            println!("  active_wallets  ✓  {wals}");
        }
        Err(e) => println!("  pulse           ✗  {e}"),
    }

    // ── Redis streams ───────────────────────────────────────────────────────
    println!("{}", "─".repeat(50));
    let redis_client = redis::Client::open(redis_url)?;
    match redis_client.get_connection() {
        Ok(mut conn) => {
            for stream in &["atlas:shreds", "atlas:newtx", "atlas:slots", "atlas:entries"] {
                let len: redis::RedisResult<i64> = conn.xlen(stream);
                match len {
                    Ok(n)  => println!("  {stream:<20} ✓  {n} events"),
                    Err(e) => println!("  {stream:<20} ✗  {e}"),
                }
            }

            // Check shred stream is actively growing
            let len1: i64 = conn.xlen("atlas:shreds").unwrap_or(0);
            std::thread::sleep(std::time::Duration::from_secs(2));
            let len2: i64 = conn.xlen("atlas:shreds").unwrap_or(0);
            let rate = (len2 - len1) / 2;
            println!("  shred ingest rate         ~{rate} events/s");
        }
        Err(e) => println!("  redis           ✗  {e}"),
    }

    // ── Unix bridge socket ──────────────────────────────────────────────────
    println!("{}", "─".repeat(50));
    let sock = std::env::var("ATLAS_BRIDGE_SOCKET")
        .unwrap_or_else(|_| "/tmp/atlas-bridge.sock".to_string());
    use std::os::unix::fs::FileTypeExt;
    match std::fs::metadata(&sock) {
        Ok(m) if m.file_type().is_socket() => println!("  bridge socket   ✓  {sock}"),
        Ok(_)  => println!("  bridge socket   ✗  {sock} exists but is not a socket"),
        Err(_) => println!("  bridge socket   ✗  {sock} not found"),
    }

    println!("{}", "─".repeat(50));
    Ok(())
}
