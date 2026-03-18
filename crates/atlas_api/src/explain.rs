//! Transaction explanation engine.
//!
//! `explain_with_llm()` calls a configured LLM provider using the TOON-rendered
//! transaction as context.  Falls back to the deterministic template engine when
//! no provider is configured or the LLM call fails.

use atlas_types::facts::{TxFactsV1, Action};
use atlas_toon::render_txfacts;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use tracing::warn;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExplainResult {
    pub summary:    String,
    pub bullets:    Vec<String>,
    pub confidence: f64,
    /// Set to "llm" when the answer came from an LLM, "template" otherwise.
    pub source:     String,
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Call LLM if configured, otherwise fall back to template.
/// `http` and `cfg` come from AppState.
pub async fn explain_with_llm(
    facts: &TxFactsV1,
    http:  &reqwest::Client,
    cfg:   &atlas_common::AppConfig,
) -> ExplainResult {
    let provider = cfg.llm_provider.as_deref().unwrap_or("none");
    if provider == "none" || provider.is_empty() {
        return template_explain(facts);
    }

    let facts_toon = render_txfacts(facts);
    match call_llm(http, cfg, &facts_toon).await {
        Ok(result) => result,
        Err(e) => {
            warn!(err = %e, provider, "LLM explain failed, falling back to template");
            template_explain(facts)
        }
    }
}

/// Pure deterministic template (no network, always works).
pub fn explain(facts: &TxFactsV1) -> ExplainResult {
    template_explain(facts)
}

// ── LLM call ─────────────────────────────────────────────────────────────────

async fn call_llm(
    http: &reqwest::Client,
    cfg:  &atlas_common::AppConfig,
    facts_toon: &str,
) -> anyhow::Result<ExplainResult> {
    let base_url = cfg.llm_base_url.as_deref().unwrap_or("http://localhost:11434/v1");
    let model    = cfg.llm_model.as_deref().unwrap_or("llama3.2");
    let api_key  = cfg.llm_api_key.as_deref().unwrap_or("ollama");

    let system = "You are a blockchain transaction analyst for the X1 network (a Solana fork). \
        The user provides a transaction in TOON format (a compact structured notation). \
        Respond in JSON with exactly these fields:\n\
        - summary (string): one-sentence plain-English description of what happened\n\
        - bullets (array of strings): 2-5 key facts about the transaction\n\
        - confidence (number 0-1): how confident you are in the interpretation\n\
        Use XNT for the native currency. Be concise. Do not add commentary outside the JSON.";

    let user = format!(
        "Explain this X1 blockchain transaction:\n\n```\n{}\n```\n\nRespond with valid JSON only.",
        facts_toon
    );

    let body = json!({
        "model": model,
        "messages": [
            { "role": "system",    "content": system },
            { "role": "user",      "content": user   },
        ],
        "temperature": 0.2,
        "max_tokens":  512,
        "response_format": { "type": "json_object" }
    });

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let resp = http
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text   = resp.text().await.unwrap_or_default();
        anyhow::bail!("LLM API returned {}: {}", status, &text[..text.len().min(200)]);
    }

    let completion: Value = resp.json().await?;
    let content = completion["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("no content in LLM response"))?;

    let parsed: Value = serde_json::from_str(content)?;

    Ok(ExplainResult {
        summary:    parsed["summary"].as_str().unwrap_or("").to_string(),
        bullets:    parsed["bullets"].as_array()
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect())
            .unwrap_or_default(),
        confidence: parsed["confidence"].as_f64().unwrap_or(0.8),
        source:     "llm".to_string(),
    })
}

// ── Template engine (deterministic, no network) ───────────────────────────────

fn template_explain(facts: &TxFactsV1) -> ExplainResult {
    let fee_xnt    = facts.fee_lamports as f64 / 1_000_000_000.0;
    let status_str = if facts.is_success() { "succeeded" } else { "failed" };

    if facts.actions.is_empty() {
        return ExplainResult {
            summary:    format!("Transaction {} with no parsed actions (fee: {:.6} XNT).", status_str, fee_xnt),
            bullets:    vec![format!("Fee paid: {:.6} XNT", fee_xnt)],
            confidence: 0.5,
            source:     "template".to_string(),
        };
    }

    let dominant = &facts.actions[0];
    let summary  = build_summary(dominant, status_str, fee_xnt);
    let mut bullets = vec![format!("Transaction {}", status_str)];
    bullets.push(format!("Fee: {:.6} XNT", fee_xnt));

    for action in &facts.actions {
        bullets.push(describe_action(action));
    }

    if !facts.token_deltas.is_empty() {
        for delta in &facts.token_deltas {
            bullets.push(format!(
                "Token balance {}: {} {}",
                delta.direction_str(),
                delta.delta,
                delta.symbol.as_deref().unwrap_or(&abbrev_key(&delta.mint)),
            ));
        }
    }

    ExplainResult {
        summary,
        bullets,
        confidence: if facts.actions.len() == 1 { 0.9 } else { 0.75 },
        source:     "template".to_string(),
    }
}

fn build_summary(action: &Action, status: &str, fee_xnt: f64) -> String {
    let subj = abbrev_key(&action.s);
    match action.t.as_str() {
        "TRANSFER" => {
            let dest = action.x.as_deref().map(abbrev_key).unwrap_or_else(|| "unknown".to_string());
            format!("Wallet {} transferred funds to {} ({}).", subj, dest, status)
        }
        "SWAP" => {
            let pool = action.x.as_deref().map(abbrev_key).unwrap_or_else(|| "unknown".to_string());
            format!("Wallet {} swapped tokens on {} ({}).", subj, action.p, status)
        }
        "MINT"     => format!("Wallet {} minted tokens ({}).", subj, status),
        "BURN"     => format!("Wallet {} burned tokens ({}).", subj, status),
        "STAKE"    => {
            let validator = action.x.as_deref().map(abbrev_key).unwrap_or_else(|| "unknown".to_string());
            format!("Wallet {} delegated stake to {} ({}).", subj, validator, status)
        }
        "UNSTAKE"  => format!("Wallet {} deactivated/withdrew stake ({}).", subj, status),
        "DEPLOY"   => {
            let deployed = action.x.as_deref().map(abbrev_key).unwrap_or_else(|| "unknown".to_string());
            format!("Wallet {} deployed/upgraded program {} ({}).", subj, deployed, status)
        }
        "NFT_SALE" => format!("Wallet {} executed an NFT sale ({}).", subj, status),
        _          => format!("Transaction by {} ({}, fee {:.6} XNT).", subj, status, fee_xnt),
    }
}

fn describe_action(action: &Action) -> String {
    let subj = abbrev_key(&action.s);
    let dest = action.x.as_deref().map(abbrev_key).unwrap_or_default();
    match action.t.as_str() {
        "TRANSFER" => format!("{} → {} ({})", subj, dest, action.p),
        "SWAP"     => format!("{} swapped on {}", subj, action.p),
        "MINT"     => format!("{} minted ({})", subj, action.p),
        "BURN"     => format!("{} burned ({})", subj, action.p),
        "STAKE"    => format!("{} staked → {}", subj, dest),
        "UNSTAKE"  => format!("{} unstaked", subj),
        "DEPLOY"   => format!("{} deployed program {}", subj, dest),
        _          => format!("{} {} ({})", subj, action.t.to_lowercase(), action.p),
    }
}

fn abbrev_key(s: &str) -> String {
    if s.len() > 12 {
        format!("{}..{}", &s[..6], &s[s.len()-4..])
    } else {
        s.to_string()
    }
}

trait DeltaExt {
    fn direction_str(&self) -> &'static str;
}
impl DeltaExt for atlas_types::facts::TokenDelta {
    fn direction_str(&self) -> &'static str {
        match self.direction {
            atlas_types::facts::DeltaDirection::In   => "received",
            atlas_types::facts::DeltaDirection::Out  => "sent",
            atlas_types::facts::DeltaDirection::None => "unchanged",
        }
    }
}
