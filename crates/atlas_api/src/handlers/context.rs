//! GET /v1/wallet/:addr/context
//!
//! Returns everything known about a wallet in a single TOON document,
//! optimised for direct injection into an LLM prompt.
//!
//! Includes:
//!  - identity label (if known)
//!  - XNT balance (live from validator RPC)
//!  - token balances (from token_account_index)
//!  - intelligence scores + type
//!  - recent transactions (last 10, as TOON table)
//!  - top programs & tokens
//!  - related wallets

use axum::{extract::{State, Path}, http::HeaderMap, response::Response};
use serde_json::{json, Value};
use sqlx::Row;
use crate::{state::AppState, error::ApiError, negotiate::{negotiate, respond}};

pub async fn wallet_context(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
    headers:      HeaderMap,
) -> Result<Response, ApiError> {
    let pool = state.pool();

    // ── identity ────────────────────────────────────────────────────────────
    let label: Option<String> = sqlx::query(
        "SELECT name FROM entity_labels WHERE address = $1"
    )
    .bind(&addr)
    .fetch_optional(pool).await?
    .and_then(|r| r.try_get("name").ok());

    // ── XNT balance from validator RPC ──────────────────────────────────────
    let xnt_lamports: i64 = {
        let rpc = &state.cfg().validator_rpc_url;
        let body = json!({
            "jsonrpc":"2.0","id":1,
            "method":"getBalance",
            "params":[addr, {"commitment":"confirmed"}]
        });
        async {
            let resp = state.http().post(rpc).json(&body).send().await.ok()?;
            let val: Value = resp.json().await.ok()?;
            val["result"]["value"].as_i64()
        }.await.unwrap_or(0)
    };

    // ── token balances ───────────────────────────────────────────────────────
    let token_rows = sqlx::query(
        r#"SELECT ta.mint, ta.amount, el.name AS symbol
           FROM token_account_index ta
           LEFT JOIN entity_labels el ON el.address = ta.mint
           WHERE ta.owner = $1
           ORDER BY ta.amount DESC
           LIMIT 10"#
    )
    .bind(&addr)
    .fetch_all(pool).await.unwrap_or_default();

    // ── intelligence profile (7d window) ────────────────────────────────────
    let profile_row = sqlx::query(
        r#"SELECT wallet_type, confidence,
                  automation_score, sniper_score, whale_score, risk_score,
                  top_programs_json, top_tokens_json
           FROM intelligence_wallet_profiles
           WHERE address = $1 AND "window" = '7d'"#
    )
    .bind(&addr)
    .fetch_optional(pool).await?;

    // ── recent txs ───────────────────────────────────────────────────────────
    let tx_rows = sqlx::query(
        r#"SELECT ai.sig, ai.slot, ai.block_time, t.fee_lamports, t.tags, t.actions_json
           FROM address_index ai
           JOIN tx_store t ON t.sig = ai.sig
           WHERE ai.address = $1
           ORDER BY ai.slot DESC, ai.pos DESC
           LIMIT 10"#
    )
    .bind(&addr)
    .fetch_all(pool).await.unwrap_or_default();

    // ── related wallets ──────────────────────────────────────────────────────
    let related_rows = sqlx::query(
        r#"SELECT CASE WHEN src = $1 THEN dst ELSE src END AS peer, reason, weight
           FROM intelligence_wallet_edges
           WHERE src = $1 OR dst = $1
           ORDER BY weight DESC LIMIT 5"#
    )
    .bind(&addr)
    .fetch_all(pool).await.unwrap_or_default();

    // ── assemble JSON for TOON rendering ────────────────────────────────────
    let ctx = json!({
        "address":    addr,
        "label":      label,
        "xnt_balance_lamports": xnt_lamports,
        "xnt_balance": format!("{:.6}", xnt_lamports as f64 / 1e9),
        "token_balances": token_rows.iter().map(|r| json!({
            "mint":   r.try_get::<String, _>("mint").unwrap_or_default(),
            "symbol": r.try_get::<Option<String>, _>("symbol").ok().flatten(),
            "amount": r.try_get::<i64, _>("amount").unwrap_or(0),
        })).collect::<Vec<_>>(),
        "intel": profile_row.as_ref().map(|r| json!({
            "type":       r.try_get::<String, _>("wallet_type").unwrap_or("unknown".into()),
            "confidence": r.try_get::<f64, _>("confidence").unwrap_or(0.0),
            "automation": r.try_get::<i32, _>("automation_score").unwrap_or(0),
            "sniper":     r.try_get::<i32, _>("sniper_score").unwrap_or(0),
            "whale":      r.try_get::<i32, _>("whale_score").unwrap_or(0),
            "risk":       r.try_get::<i32, _>("risk_score").unwrap_or(0),
        })).unwrap_or(json!(null)),
        "recent_txs": tx_rows.iter().map(|r| json!({
            "sig":       r.try_get::<String, _>("sig").unwrap_or_default(),
            "slot":      r.try_get::<i64, _>("slot").unwrap_or(0),
            "block_time":r.try_get::<Option<i64>, _>("block_time").ok().flatten(),
            "fee":       r.try_get::<i64, _>("fee_lamports").unwrap_or(0),
            "tags":      r.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
        })).collect::<Vec<_>>(),
        "related": related_rows.iter().map(|r| json!({
            "address": r.try_get::<String, _>("peer").unwrap_or_default(),
            "reason":  r.try_get::<String, _>("reason").unwrap_or_default(),
            "weight":  r.try_get::<f64, _>("weight").unwrap_or(0.0),
        })).collect::<Vec<_>>(),
    });

    let toon = render_context_toon(&ctx);
    let fmt  = negotiate(&headers, None);
    Ok(respond(fmt, &ctx, toon))
}

fn render_context_toon(c: &Value) -> String {
    let mut out = String::new();
    let addr = c["address"].as_str().unwrap_or("");

    out.push_str(&format!("wallet: {}\n", addr));
    if let Some(label) = c["label"].as_str() {
        out.push_str(&format!(" label:   {}\n", label));
    }
    out.push_str(&format!(
        " balance: {} XNT ({} lamports)\n",
        c["xnt_balance"].as_str().unwrap_or("0.000000"),
        c["xnt_balance_lamports"].as_i64().unwrap_or(0)
    ));
    out.push('\n');

    // token balances
    let tokens = c["token_balances"].as_array().map(|v| v.as_slice()).unwrap_or_default();
    if !tokens.is_empty() {
        out.push_str(&format!("token_balances[{}]{{mint,symbol,amount}}:\n", tokens.len()));
        for t in tokens {
            out.push_str(&format!(
                " {},{},{}\n",
                &t["mint"].as_str().unwrap_or("")
                    [..t["mint"].as_str().unwrap_or("").len().min(20)],
                t["symbol"].as_str().unwrap_or("-"),
                t["amount"].as_i64().unwrap_or(0),
            ));
        }
        out.push('\n');
    }

    // intel
    if let Some(intel) = c["intel"].as_object() {
        out.push_str("intel:\n");
        out.push_str(&format!(" type:       {}\n", intel["type"].as_str().unwrap_or("unknown")));
        out.push_str(&format!(" confidence: {:.2}\n", intel["confidence"].as_f64().unwrap_or(0.0)));
        out.push_str(&format!(" automation: {}\n", intel["automation"].as_i64().unwrap_or(0)));
        out.push_str(&format!(" sniper:     {}\n", intel["sniper"].as_i64().unwrap_or(0)));
        out.push_str(&format!(" whale:      {}\n", intel["whale"].as_i64().unwrap_or(0)));
        out.push_str(&format!(" risk:       {}\n", intel["risk"].as_i64().unwrap_or(0)));
        out.push('\n');
    }

    // recent txs
    let txs = c["recent_txs"].as_array().map(|v| v.as_slice()).unwrap_or_default();
    out.push_str(&format!("recent_txs[{}]{{sig,slot,time,fee,tags}}:\n", txs.len()));
    for tx in txs {
        let sig  = tx["sig"].as_str().unwrap_or("");
        let abbr = if sig.len() > 16 { format!("{}..{}", &sig[..8], &sig[sig.len()-4..]) } else { sig.to_string() };
        let tags = tx["tags"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("|"))
            .unwrap_or_default();
        out.push_str(&format!(
            " {},{},{},{},{}\n",
            abbr,
            tx["slot"].as_i64().unwrap_or(0),
            tx["block_time"].as_i64().unwrap_or(0),
            tx["fee"].as_i64().unwrap_or(0),
            tags,
        ));
    }
    out.push('\n');

    // related
    let related = c["related"].as_array().map(|v| v.as_slice()).unwrap_or_default();
    if !related.is_empty() {
        out.push_str(&format!("related[{}]{{address,reason,weight}}:\n", related.len()));
        for r in related {
            let peer = r["address"].as_str().unwrap_or("");
            let abbr = if peer.len() > 16 { format!("{}..{}", &peer[..8], &peer[peer.len()-4..]) } else { peer.to_string() };
            out.push_str(&format!(
                " {},{},{:.1}\n",
                abbr,
                r["reason"].as_str().unwrap_or(""),
                r["weight"].as_f64().unwrap_or(0.0),
            ));
        }
    }

    out
}

