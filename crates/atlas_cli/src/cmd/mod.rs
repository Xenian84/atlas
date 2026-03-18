pub mod status;
pub mod tx;
pub mod wallet;
pub mod stream;
pub mod pulse;
pub mod token;
pub mod block;
pub mod keys;

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

pub async fn api_get(client: &Client, url: &str) -> Result<Value> {
    let resp = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .with_context(|| format!("GET {url}"))?;

    let status = resp.status();
    let body   = resp.text().await?;

    if !status.is_success() {
        anyhow::bail!("HTTP {status}: {body}");
    }

    serde_json::from_str(&body).with_context(|| format!("parsing response from {url}"))
}

