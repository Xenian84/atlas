//! atlas token <mint> — token overview, holders, transfers.

use anyhow::{Context, Result};

pub async fn run(api: &str, mint: &str, key: &str, holders: bool) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let tok: serde_json::Value = client
        .get(format!("{api}/v1/token/{mint}"))
        .header("X-Api-Key", key)
        .send()
        .await
        .with_context(|| format!("GET {api}/v1/token/{mint}"))?
        .json()
        .await?;

    let name    = tok["name"].as_str().unwrap_or("");
    let symbol  = tok["symbol"].as_str().unwrap_or("");
    let decimals= tok["decimals"].as_i64().unwrap_or(0);
    let supply  = tok["supply"].as_i64().unwrap_or(0);
    let holder_count = tok["holders"].as_i64().unwrap_or(0);
    let transfers_24h = tok["transfers_24h"].as_i64().unwrap_or(0);
    let logo    = tok["logo_uri"].as_str().unwrap_or("");
    let is_nft  = tok["is_nft"].as_bool().unwrap_or(false);

    println!("Token");
    println!("{}", "─".repeat(60));
    println!("  mint          {mint}");
    if !name.is_empty()   { println!("  name          {name}"); }
    if !symbol.is_empty() { println!("  symbol        {symbol}"); }
    println!("  decimals      {decimals}");
    println!("  supply        {supply}");
    println!("  is_nft        {is_nft}");
    println!("  holders       {holder_count}");
    println!("  transfers_24h {transfers_24h}");
    if !logo.is_empty()   { println!("  logo_uri      {logo}"); }

    if let Some(identity) = tok["identity"].as_object() {
        let n = identity.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if !n.is_empty() {
            println!("  identity      {n}");
        }
    }

    if holders {
        println!();
        let h: serde_json::Value = client
            .get(format!("{api}/v1/token/{mint}/holders?limit=10"))
            .header("X-Api-Key", key)
            .send()
            .await?
            .json()
            .await?;

        println!("  top holders:");
        if let Some(arr) = h["holders"].as_array() {
            for (i, holder) in arr.iter().enumerate() {
                let owner  = holder["owner"].as_str().unwrap_or("?");
                let amount = holder["amount"].as_str().unwrap_or("0");
                let short  = if owner.len() > 16 { &owner[..16] } else { owner };
                println!("    {:>2}. {short}…  {amount}", i + 1);
            }
        }
    }

    println!("{}", "─".repeat(60));
    Ok(())
}
