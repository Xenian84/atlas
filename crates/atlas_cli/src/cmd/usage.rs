//! atlas usage — show API key request statistics.

use anyhow::{Context, Result};

pub async fn run(api: &str, admin_key: &str, key_prefix: Option<&str>, json: bool) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let url = format!("{api}/v1/keys");
    let resp: serde_json::Value = client
        .get(&url)
        .header("X-Api-Key", admin_key)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?
        .json()
        .await?;

    let empty = vec![];
    let keys = resp["keys"].as_array().unwrap_or(&empty);

    // Filter by prefix if requested
    let filtered: Vec<&serde_json::Value> = keys.iter().filter(|k| {
        match key_prefix {
            Some(pfx) => k["key_prefix"].as_str().unwrap_or("").starts_with(pfx),
            None => true,
        }
    }).collect();

    if json {
        let out: Vec<serde_json::Value> = filtered.iter().map(|k| {
            serde_json::json!({
                "key_prefix":    k["key_prefix"],
                "name":          k["name"],
                "tier":          k["tier"],
                "is_active":     k["is_active"],
                "rate_limit_rpm": k["rate_limit"],
                "created_at":    k["created_at"],
                "last_used_at":  k["last_used_at"],
            })
        }).collect();
        println!("{}", serde_json::json!({ "total": out.len(), "keys": out }));
        return Ok(());
    }

    println!("API Key Usage");
    println!("{}", "─".repeat(72));
    println!("  {:<12}  {:<20}  {:<10}  {:<6}  {}", "PREFIX", "NAME", "TIER", "RPM", "LAST USED");
    println!("{}", "─".repeat(72));

    for k in &filtered {
        let prefix   = k["key_prefix"].as_str().unwrap_or("?");
        let name     = k["name"].as_str().unwrap_or("(unnamed)");
        let tier     = k["tier"].as_str().unwrap_or("?");
        let rpm      = k["rate_limit"].as_i64().unwrap_or(0);
        let active   = k["is_active"].as_bool().unwrap_or(false);
        let last_use = k["last_used_at"].as_str().unwrap_or("never");
        let status   = if active { "" } else { "  [revoked]" };
        println!("  {prefix:<12}  {name:<20}  {tier:<10}  {rpm:<6}  {last_use}{status}");
    }

    println!("{}", "─".repeat(72));
    println!("  Total: {} key(s)", filtered.len());
    println!();
    println!("  Tip: atlas usage --json   for machine-readable output");
    Ok(())
}
