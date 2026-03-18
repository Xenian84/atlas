//! atlas keys — API key management (admin only).

use anyhow::{Context, Result};

pub async fn run_list(api: &str, key: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp: serde_json::Value = client
        .get(format!("{api}/v1/keys"))
        .header("X-Api-Key", key)
        .send()
        .await
        .with_context(|| "GET /v1/keys")?
        .json()
        .await?;

    let count = resp["count"].as_i64().unwrap_or(0);
    println!("API Keys ({count})");
    println!("{}", "─".repeat(72));

    if let Some(keys) = resp["keys"].as_array() {
        for k in keys {
            let prefix   = k["key_prefix"].as_str().unwrap_or("?");
            let name     = k["name"].as_str().unwrap_or("?");
            let tier     = k["tier"].as_str().unwrap_or("?");
            let rpm      = k["rate_limit"].as_i64().unwrap_or(0);
            let active   = k["is_active"].as_bool().unwrap_or(false);
            let created  = k["created_at"].as_str().unwrap_or("?");
            let last_use = k["last_used_at"].as_str().unwrap_or("never");
            let status   = if active { "active" } else { "revoked" };
            println!("  {prefix}…  [{status}]  {name}  tier={tier}  rpm={rpm}");
            println!("    created={created}  last_used={last_use}");
        }
    }

    println!("{}", "─".repeat(72));
    Ok(())
}

pub async fn run_create(api: &str, key: &str, name: &str, tier: &str, rpm: i32) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp: serde_json::Value = client
        .post(format!("{api}/v1/keys"))
        .header("X-Api-Key", key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "name":       name,
            "tier":       tier,
            "rate_limit": rpm,
        }))
        .send()
        .await
        .with_context(|| "POST /v1/keys")?
        .json()
        .await?;

    if let Some(err) = resp["error"].as_str() {
        println!("Error: {err}");
        return Ok(());
    }

    let api_key = resp["api_key"].as_str().unwrap_or("?");
    let warning = resp["warning"].as_str().unwrap_or("");

    println!("API Key Created");
    println!("{}", "─".repeat(60));
    println!("  name       {name}");
    println!("  tier       {tier}");
    println!("  rate_limit {rpm} rpm");
    println!();
    println!("  API_KEY = {api_key}");
    println!();
    println!("  ⚠  {warning}");
    println!("{}", "─".repeat(60));
    Ok(())
}

pub async fn run_revoke(api: &str, admin_key: &str, id: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp: serde_json::Value = client
        .delete(format!("{api}/v1/keys/{id}"))
        .header("X-Api-Key", admin_key)
        .send()
        .await
        .with_context(|| format!("DELETE /v1/keys/{id}"))?
        .json()
        .await?;

    println!("Key {id}: {}", resp["status"].as_str().unwrap_or("unknown"));
    Ok(())
}
