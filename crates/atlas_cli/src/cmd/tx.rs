//! atlas tx <sig> — look up a transaction.

use anyhow::Result;
use chrono::{DateTime, Utc};

pub async fn run(api: &str, sig: &str, json: bool) -> Result<()> {
    let key = std::env::var("ATLAS_API_KEY")
        .unwrap_or_else(|_| "atlas-admin-key-change-in-production".to_string());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp = client.get(format!("{api}/v1/tx/{sig}"))
        .header("Accept", "application/json")
        .header("X-Api-Key", &key)
        .send().await?;
    let tx: serde_json::Value = resp.json().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&tx)?);
        return Ok(());
    }

    let slot       = tx["slot"].as_i64().unwrap_or(0);
    let block_time = tx["block_time"].as_i64().unwrap_or(0);
    let commitment = tx["commitment"].as_str().unwrap_or("unknown");
    let fee        = tx["fee"].as_i64().unwrap_or(0);
    let err        = tx["err"].as_str().map(|s| s.to_string())
                        .or_else(|| tx["err"].as_bool().map(|b| b.to_string()))
                        .unwrap_or_else(|| "none".to_string());

    let time_str = if block_time > 0 {
        DateTime::<Utc>::from_timestamp(block_time, 0)
            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| block_time.to_string())
    } else {
        "unknown".to_string()
    };

    println!("Transaction");
    println!("{}", "─".repeat(60));
    println!("  sig         {sig}");
    println!("  slot        #{slot}");
    println!("  time        {time_str}");
    println!("  commitment  {commitment}");
    println!("  fee         {fee} lamports");
    println!("  error       {err}");

    if let Some(accounts) = tx["accounts"].as_array() {
        println!();
        println!("  accounts ({}):", accounts.len());
        for acc in accounts.iter().take(6) {
            let addr = acc["address"].as_str().unwrap_or(acc.as_str().unwrap_or("?"));
            println!("    {addr}");
        }
        if accounts.len() > 6 {
            println!("    … and {} more", accounts.len() - 6);
        }
    }

    if let Some(programs) = tx["programs"].as_array() {
        println!();
        println!("  programs:");
        for prog in programs {
            let p = prog.as_str().unwrap_or("?");
            println!("    {p}");
        }
    }

    if let Some(tags) = tx["tags"].as_array() {
        let tag_list: Vec<&str> = tags.iter()
            .filter_map(|t| t.as_str())
            .collect();
        println!();
        println!("  tags        {}", tag_list.join(", "));
    }

    println!("{}", "─".repeat(60));
    Ok(())
}
