//! atlas wallet <address> — unified wallet overview from Atlas DB (no RPC fallback).

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

pub async fn run(api: &str, rpc: &str, address: &str, key: &str, json: bool) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    if json {
        let resp: serde_json::Value = client
            .get(format!("{api}/v1/wallet/{address}"))
            .header("Accept", "application/json")
            .header("X-Api-Key", key)
            .send().await?
            .json().await?;
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    // Call the unified /v1/wallet/:addr endpoint
    let resp = client
        .get(format!("{api}/v1/wallet/{address}"))
        .header("Accept", "application/json")
        .header("X-Api-Key", key)
        .send()
        .await
        .with_context(|| format!("GET {api}/v1/wallet/{address}"))?;

    if resp.status() == 404 {
        // Account not in our DB at all — try on-chain as absolute last resort
        println!("Wallet (on-chain lookup — not yet indexed)");
        println!("{}", "─".repeat(60));
        println!("  address   {address}");
        println!("  note      No transaction history indexed yet.");
        fetch_onchain(rpc, address).await;
        println!("{}", "─".repeat(60));
        return Ok(());
    }

    let w: serde_json::Value = resp.json().await?;

    let lamports  = w["balance"]["lamports"].as_i64().unwrap_or(0);
    let xnt       = lamports as f64 / 1_000_000_000.0;
    let tx_count  = w["tx_count"].as_i64().unwrap_or(0);
    let first_seen= w["first_seen"].as_i64();
    let last_seen = w["last_seen"].as_i64();
    let owner     = w["account"]["owner"].as_str().unwrap_or("?");
    let executable= w["account"]["executable"].as_bool().unwrap_or(false);
    let lag       = w["account"]["updated_slot"].as_i64().unwrap_or(0);

    let fmt_ts = |ts: Option<i64>| -> String {
        ts.and_then(|t| DateTime::<Utc>::from_timestamp(t, 0))
          .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
          .unwrap_or_else(|| "never".to_string())
    };

    println!("Wallet");
    println!("{}", "─".repeat(60));
    println!("  address       {address}");
    println!("  balance       {xnt:.6} XNT  ({lamports} lamports)");
    println!("  owner         {owner}");
    if executable { println!("  executable    true (this is a program)"); }
    println!("  tx_count      {tx_count}");
    println!("  first_seen    {}", fmt_ts(first_seen));
    println!("  last_seen     {}", fmt_ts(last_seen));
    println!("  balance_slot  #{lag}");

    if let Some(identity) = w["identity"].as_object() {
        let name = identity.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let cat  = identity.get("category").and_then(|v| v.as_str()).unwrap_or("");
        if !name.is_empty() {
            println!();
            println!("  identity      {name} [{cat}]");
        }
    }

    if let Some(tokens) = w["tokens"].as_array() {
        if !tokens.is_empty() {
            println!();
            println!("  token holdings ({}):", tokens.len());
            for tok in tokens.iter().take(8) {
                let symbol  = tok["symbol"].as_str().unwrap_or("");
                let name    = tok["name"].as_str().unwrap_or("");
                let amount  = tok["amount"].as_str().unwrap_or("0");
                let decimals= tok["decimals"].as_u64().unwrap_or(0) as u32;
                let display = format_token_amount(amount, decimals);
                let label   = if !symbol.is_empty() { symbol } else if !name.is_empty() { name } else { "unknown" };
                println!("    {label:<10}  {display}");
            }
            if tokens.len() > 8 {
                println!("    … and {} more", tokens.len() - 8);
            }
        }
    }

    if let Some(profile) = w["profile"].as_object() {
        let wtype = profile.get("wallet_type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let risk  = profile.get("risk_score").and_then(|v| v.as_i64()).unwrap_or(0);
        let auto  = profile.get("automation_score").and_then(|v| v.as_i64()).unwrap_or(0);
        if wtype != "unknown" {
            println!();
            println!("  profile       type={wtype}  risk={risk}  automation={auto}");
        }
    }

    println!("{}", "─".repeat(60));
    Ok(())
}

async fn fetch_onchain(rpc: &str, address: &str) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getAccountInfo",
        "params": [address, {"encoding": "base64"}]
    });
    if let Ok(resp) = client.post(rpc).json(&body).send().await {
        if let Ok(v) = resp.json::<serde_json::Value>().await {
            if let Some(val) = v["result"]["value"].as_object() {
                let lamports = val.get("lamports").and_then(|v| v.as_u64()).unwrap_or(0);
                let xnt      = lamports as f64 / 1_000_000_000.0;
                let owner    = val.get("owner").and_then(|v| v.as_str()).unwrap_or("?");
                println!("  balance   {xnt:.6} XNT  ({lamports} lamports)  [source: RPC]");
                println!("  owner     {owner}");
                return;
            }
        }
    }
    println!("  (account not found on-chain)");
}

fn format_token_amount(amount_str: &str, decimals: u32) -> String {
    let raw: u128 = amount_str.parse().unwrap_or(0);
    if decimals == 0 { return raw.to_string(); }
    let divisor = 10u128.pow(decimals);
    let whole   = raw / divisor;
    let frac    = raw % divisor;
    format!("{}.{:0>width$}", whole, frac, width = decimals as usize)
        .trim_end_matches('0').trim_end_matches('.').to_string()
}
