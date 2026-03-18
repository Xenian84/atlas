//! Atlas MCP Server — Model Context Protocol over HTTP+SSE.
//!
//! Implements the MCP 2024-11-05 specification so Claude, OpenClaw agents,
//! and any MCP-compatible LLM can directly call Atlas as a tool provider.
//!
//! ## Endpoints
//!  POST /mcp          — JSON-RPC 2.0 request/response (stateless calls)
//!  GET  /mcp/sse      — SSE stream for MCP streaming transport (optional)
//!
//! ## Tools exposed
//!  get_transaction(sig)                    → TxFactsV1 as TOON
//!  get_wallet_context(address)             → full wallet context as TOON
//!  get_wallet_profile(address, window)     → intelligence scores
//!  get_assets_by_owner(address)            → DAS asset list
//!  get_related_wallets(address, limit)     → co-occurrence edges
//!  explain_transaction(sig)                → LLM/template explanation
//!  network_pulse()                         → live network stats
//!  get_address_history(address, limit)     → recent tx list as TOON

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use serde_json::{json, Value};
use tokio_stream::StreamExt as _;
use futures::stream;
use crate::{state::AppState, error::ApiError};
use super::tx::fetch_tx_facts;

// ── Tool manifest ──────────────────────────────────────────────────────────────

fn tool_list() -> Value {
    json!([
        {
            "name": "get_transaction",
            "description": "Fetch full details of an X1 transaction by signature. Returns TxFactsV1 with actions, token deltas, XNT balance changes, tags and programs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sig": { "type": "string", "description": "Transaction signature (base58)" }
                },
                "required": ["sig"]
            }
        },
        {
            "name": "explain_transaction",
            "description": "Generate a natural-language explanation of an X1 transaction. Uses LLM when configured, otherwise template engine.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sig": { "type": "string", "description": "Transaction signature" }
                },
                "required": ["sig"]
            }
        },
        {
            "name": "get_wallet_context",
            "description": "Get complete context for an X1 wallet: identity label, XNT balance, token balances, intelligence scores, recent transactions, and related wallets. Returns compact TOON format ideal for LLM prompts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "address": { "type": "string", "description": "Wallet public key (base58)" }
                },
                "required": ["address"]
            }
        },
        {
            "name": "get_wallet_profile",
            "description": "Get wallet intelligence profile: bot/sniper/whale/human classification with confidence score and activity features.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "address": { "type": "string", "description": "Wallet public key" },
                    "window":  { "type": "string", "enum": ["24h","7d","30d","all"], "description": "Time window (default 7d)" }
                },
                "required": ["address"]
            }
        },
        {
            "name": "get_address_history",
            "description": "Fetch recent transaction history for an X1 address. Returns a TOON table of transactions with tags, fees, and actions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "address": { "type": "string", "description": "Wallet public key" },
                    "limit":   { "type": "integer", "minimum": 1, "maximum": 50, "description": "Number of transactions (default 20)" }
                },
                "required": ["address"]
            }
        },
        {
            "name": "get_assets_by_owner",
            "description": "List NFTs, tokens and digital assets owned by an X1 wallet using the DAS (Digital Asset Standard) API.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "address": { "type": "string", "description": "Wallet public key" },
                    "limit":   { "type": "integer", "minimum": 1, "maximum": 100 }
                },
                "required": ["address"]
            }
        },
        {
            "name": "get_related_wallets",
            "description": "Find wallets that frequently co-appear in transactions with this address. Useful for clustering and identifying connected accounts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "address": { "type": "string" },
                    "limit":   { "type": "integer", "minimum": 1, "maximum": 50, "description": "Max results (default 10)" }
                },
                "required": ["address"]
            }
        },
        {
            "name": "network_pulse",
            "description": "Get real-time X1 network statistics: current slot, TPS, active wallets (24h), indexed transactions, top programs and top transaction types.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }
    ])
}

// ── MCP JSON-RPC handler ───────────────────────────────────────────────────────

/// POST /mcp — stateless MCP JSON-RPC 2.0
pub async fn mcp_handler(
    State(state): State<AppState>,
    Json(req):    Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let id     = req.get("id").cloned().unwrap_or(json!(null));
    let method = req["method"].as_str().unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(json!({}));

    let result = match method {
        // ── MCP lifecycle ──────────────────────────────────────────────────
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": {}
            },
            "serverInfo": {
                "name":    "atlas-x1",
                "version": "2.0.0"
            }
        }),

        "tools/list" => json!({ "tools": tool_list() }),

        // ── Tool dispatch ──────────────────────────────────────────────────
        "tools/call" => {
            let tool_name = params["name"].as_str().unwrap_or("");
            let args      = params.get("arguments").cloned().unwrap_or(json!({}));
            dispatch_tool(&state, tool_name, args).await?
        }

        // ── Notifications (fire-and-forget, no response needed) ────────────
        m if m.starts_with("notifications/") => {
            return Ok(Json(json!({})));
        }

        _ => {
            return Ok(Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", method) }
            })));
        }
    };

    Ok(Json(json!({
        "jsonrpc": "2.0",
        "id":      id,
        "result":  result
    })))
}

/// GET /mcp/sse — SSE transport (returns capabilities on connect, then waits)
pub async fn mcp_sse_handler(
    State(_state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let init_event = Event::default()
        .event("endpoint")
        .data(json!({ "uri": "/mcp" }).to_string());

    let s = stream::once(async move { Ok::<_, std::convert::Infallible>(init_event) })
        .chain(stream::pending());

    Sse::new(s).keep_alive(KeepAlive::default())
}

// ── Tool implementations ───────────────────────────────────────────────────────

async fn dispatch_tool(
    state: &AppState,
    name:  &str,
    args:  Value,
) -> Result<Value, ApiError> {
    match name {
        "get_transaction" => {
            let sig   = args["sig"].as_str().unwrap_or("").to_string();
            let facts = fetch_tx_facts(state, &sig).await?;
            let toon  = atlas_toon::render_txfacts(&facts);
            Ok(tool_result(&toon))
        }

        "explain_transaction" => {
            let sig   = args["sig"].as_str().unwrap_or("").to_string();
            let facts = fetch_tx_facts(state, &sig).await?;
            let exp   = crate::explain::explain_with_llm(&facts, state.http(), state.cfg()).await;
            let toon  = atlas_toon::render_txfacts(&facts);
            let text  = format!(
                "Summary: {}\n\nKey facts:\n{}\n\nConfidence: {:.0}% (source: {})\n\nFull transaction (TOON):\n{}",
                exp.summary,
                exp.bullets.iter().map(|b| format!("• {}", b)).collect::<Vec<_>>().join("\n"),
                exp.confidence * 100.0,
                exp.source,
                toon,
            );
            Ok(tool_result(&text))
        }

        "get_wallet_context" => {
            let addr = args["address"].as_str().unwrap_or("").to_string();
            // Reuse the context handler logic by calling the DB directly
            let toon = build_wallet_context_toon(state, &addr).await?;
            Ok(tool_result(&toon))
        }

        "get_wallet_profile" => {
            let addr   = args["address"].as_str().unwrap_or("").to_string();
            let window = args["window"].as_str().unwrap_or("7d");
            let row = sqlx::query(
                r#"SELECT wallet_type, confidence,
                          automation_score, sniper_score, whale_score, risk_score,
                          features_json, top_programs_json, top_tokens_json, updated_at
                   FROM intelligence_wallet_profiles
                   WHERE address = $1 AND "window" = $2"#
            )
            .bind(&addr)
            .bind(window)
            .fetch_optional(state.pool())
            .await?;

            let text = match row {
                None => format!("No profile found for {} (window={}). The wallet may not have been indexed yet.", addr, window),
                Some(r) => {
                    use sqlx::Row;
                    format!(
                        "profile:\n address:     {}\n window:      {}\n type:        {}\n confidence:  {:.2}\n\nscores:\n automation: {}\n sniper:     {}\n whale:      {}\n risk:       {}",
                        addr, window,
                        r.try_get::<String,_>("wallet_type").unwrap_or_default(),
                        r.try_get::<f64,_>("confidence").unwrap_or(0.0),
                        r.try_get::<i32,_>("automation_score").unwrap_or(0),
                        r.try_get::<i32,_>("sniper_score").unwrap_or(0),
                        r.try_get::<i32,_>("whale_score").unwrap_or(0),
                        r.try_get::<i32,_>("risk_score").unwrap_or(0),
                    )
                }
            };
            Ok(tool_result(&text))
        }

        "get_address_history" => {
            use sqlx::Row;
            let addr  = args["address"].as_str().unwrap_or("").to_string();
            let limit = args["limit"].as_i64().unwrap_or(20).min(50);

            let rows = sqlx::query(
                r#"SELECT ai.sig, ai.slot, t.block_time, t.fee_lamports, t.tags, t.status
                   FROM address_index ai
                   JOIN tx_store t ON t.sig = ai.sig
                   WHERE ai.address = $1
                   ORDER BY ai.slot DESC, ai.pos DESC
                   LIMIT $2"#
            )
            .bind(&addr)
            .bind(limit)
            .fetch_all(state.pool()).await?;

            let mut toon = format!("txs[{}]{{sig,slot,time,status,fee,tags}}:\n", rows.len());
            for r in &rows {
                let sig  = r.try_get::<String,_>("sig").unwrap_or_default();
                let abbr = if sig.len() > 16 { format!("{}..{}", &sig[..8], &sig[sig.len()-4..]) } else { sig.clone() };
                let tags: Vec<String> = r.try_get("tags").unwrap_or_default();
                toon.push_str(&format!(
                    " {},{},{},{},{},{}\n",
                    abbr,
                    r.try_get::<i64,_>("slot").unwrap_or(0),
                    r.try_get::<Option<i64>,_>("block_time").ok().flatten().unwrap_or(0),
                    if r.try_get::<i16,_>("status").unwrap_or(1) == 1 { "ok" } else { "fail" },
                    r.try_get::<i64,_>("fee_lamports").unwrap_or(0),
                    tags.join("|"),
                ));
            }
            Ok(tool_result(&toon))
        }

        "get_assets_by_owner" => {
            use sqlx::Row;
            let addr  = args["address"].as_str().unwrap_or("").to_string();
            let limit = args["limit"].as_i64().unwrap_or(20).min(100);

            let rows = sqlx::query(
                r#"SELECT mint, name, symbol, asset_type, supply, decimals, is_frozen
                   FROM asset_index
                   WHERE owner = $1
                   ORDER BY updated_at DESC
                   LIMIT $2"#
            )
            .bind(&addr)
            .bind(limit)
            .fetch_all(state.pool()).await.unwrap_or_default();

            let mut toon = format!("assets[{}]{{mint,name,symbol,type}}:\n", rows.len());
            for r in &rows {
                toon.push_str(&format!(
                    " {},{},{},{}\n",
                    &r.try_get::<String,_>("mint").unwrap_or_default()[..20.min(44)],
                    r.try_get::<Option<String>,_>("name").ok().flatten().unwrap_or_else(|| "-".into()),
                    r.try_get::<Option<String>,_>("symbol").ok().flatten().unwrap_or_else(|| "-".into()),
                    r.try_get::<String,_>("asset_type").unwrap_or_default(),
                ));
            }
            Ok(tool_result(&toon))
        }

        "get_related_wallets" => {
            use sqlx::Row;
            let addr  = args["address"].as_str().unwrap_or("").to_string();
            let limit = args["limit"].as_i64().unwrap_or(10).min(50);

            let rows = sqlx::query(
                r#"SELECT CASE WHEN src = $1 THEN dst ELSE src END AS peer, reason, weight
                   FROM intelligence_wallet_edges
                   WHERE src = $1 OR dst = $1
                   ORDER BY weight DESC LIMIT $2"#
            )
            .bind(&addr)
            .bind(limit)
            .fetch_all(state.pool()).await.unwrap_or_default();

            let mut toon = format!("related_wallets[{}]{{address,reason,weight}}:\n", rows.len());
            for r in &rows {
                let peer = r.try_get::<String,_>("peer").unwrap_or_default();
                let abbr = if peer.len() > 16 { format!("{}..{}", &peer[..8], &peer[peer.len()-4..]) } else { peer.clone() };
                toon.push_str(&format!(
                    " {},{},{:.1}\n",
                    abbr,
                    r.try_get::<String,_>("reason").unwrap_or_default(),
                    r.try_get::<f64,_>("weight").unwrap_or(0.0),
                ));
            }
            if rows.is_empty() {
                toon.push_str(" (no related wallets indexed yet — run more transactions first)\n");
            }
            Ok(tool_result(&toon))
        }

        "network_pulse" => {
            use sqlx::Row;
            let pool = state.pool();
            let since_24h = chrono::Utc::now().timestamp() - 86400;
            let since_1m  = chrono::Utc::now().timestamp() - 60;

            let activity = sqlx::query(
                "SELECT COUNT(DISTINCT sig) AS tx_count, COUNT(DISTINCT address) AS wc FROM address_index WHERE block_time >= $1"
            ).bind(since_24h).fetch_one(pool).await?;

            let tps_row = sqlx::query(
                "SELECT COUNT(DISTINCT sig) AS c FROM address_index WHERE block_time >= $1"
            ).bind(since_1m).fetch_one(pool).await?;

            let tps: i64 = tps_row.try_get::<i64,_>("c").unwrap_or(0) / 60;

            let text = format!(
                "pulse:\n chain:              x1\n tps_1m:             {}\n active_wallets_24h: {}\n indexed_txs_24h:    {}",
                tps,
                activity.try_get::<i64,_>("wc").unwrap_or(0),
                activity.try_get::<i64,_>("tx_count").unwrap_or(0),
            );
            Ok(tool_result(&text))
        }

        unknown => Err(ApiError::BadRequest(format!("unknown tool: {}", unknown))),
    }
}

/// MCP tool result wrapper (text content).
fn tool_result(text: &str) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }]
    })
}

/// Build wallet context TOON (shared between MCP tool and REST endpoint).
async fn build_wallet_context_toon(state: &AppState, addr: &str) -> Result<String, ApiError> {
    use sqlx::Row;
    let pool = state.pool();

    let label: Option<String> = sqlx::query("SELECT name FROM entity_labels WHERE address = $1")
        .bind(addr).fetch_optional(pool).await?
        .and_then(|r| r.try_get("name").ok());

    let xnt_lamports: i64 = {
        let rpc  = &state.cfg().validator_rpc_url;
        let body = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"getBalance","params":[addr,{"commitment":"confirmed"}]});
        match state.http().post(rpc).json(&body).send().await {
            Ok(resp) => resp.json::<Value>().await.ok()
                .and_then(|v| v["result"]["value"].as_i64())
                .unwrap_or(0),
            Err(_) => 0,
        }
    };

    let profile_row = sqlx::query(
        r#"SELECT wallet_type, confidence, automation_score, sniper_score, whale_score, risk_score
           FROM intelligence_wallet_profiles WHERE address = $1 AND "window" = '7d'"#
    ).bind(addr).fetch_optional(pool).await?;

    let tx_rows = sqlx::query(
        r#"SELECT ai.sig, ai.slot, t.block_time, t.fee_lamports, t.tags
           FROM address_index ai JOIN tx_store t ON t.sig = ai.sig
           WHERE ai.address = $1 ORDER BY ai.slot DESC, ai.pos DESC LIMIT 10"#
    ).bind(addr).fetch_all(pool).await.unwrap_or_default();

    let related_rows = sqlx::query(
        r#"SELECT CASE WHEN src = $1 THEN dst ELSE src END AS peer, reason, weight
           FROM intelligence_wallet_edges WHERE src = $1 OR dst = $1 ORDER BY weight DESC LIMIT 5"#
    ).bind(addr).fetch_all(pool).await.unwrap_or_default();

    let mut out = String::new();
    out.push_str(&format!("wallet: {}\n", addr));
    if let Some(l) = &label { out.push_str(&format!(" label:   {}\n", l)); }
    out.push_str(&format!(
        " balance: {:.6} XNT ({} lamports)\n\n",
        xnt_lamports as f64 / 1e9, xnt_lamports
    ));

    if let Some(p) = &profile_row {
        out.push_str("intel:\n");
        out.push_str(&format!(" type:       {}\n", p.try_get::<String,_>("wallet_type").unwrap_or_default()));
        out.push_str(&format!(" confidence: {:.2}\n", p.try_get::<f64,_>("confidence").unwrap_or(0.0)));
        out.push_str(&format!(" automation: {}  sniper: {}  whale: {}  risk: {}\n\n",
            p.try_get::<i32,_>("automation_score").unwrap_or(0),
            p.try_get::<i32,_>("sniper_score").unwrap_or(0),
            p.try_get::<i32,_>("whale_score").unwrap_or(0),
            p.try_get::<i32,_>("risk_score").unwrap_or(0),
        ));
    }

    out.push_str(&format!("recent_txs[{}]{{sig,slot,fee,tags}}:\n", tx_rows.len()));
    for r in &tx_rows {
        let sig  = r.try_get::<String,_>("sig").unwrap_or_default();
        let abbr = if sig.len() > 16 { format!("{}..{}", &sig[..8], &sig[sig.len()-4..]) } else { sig.clone() };
        let tags: Vec<String> = r.try_get("tags").unwrap_or_default();
        out.push_str(&format!(
            " {},{},{},{}\n",
            abbr,
            r.try_get::<i64,_>("slot").unwrap_or(0),
            r.try_get::<i64,_>("fee_lamports").unwrap_or(0),
            tags.join("|"),
        ));
    }

    if !related_rows.is_empty() {
        out.push_str(&format!("\nrelated[{}]{{address,weight}}:\n", related_rows.len()));
        for r in &related_rows {
            let peer = r.try_get::<String,_>("peer").unwrap_or_default();
            let abbr = if peer.len() > 16 { format!("{}..{}", &peer[..8], &peer[peer.len()-4..]) } else { peer.clone() };
            out.push_str(&format!(" {},{:.1}\n", abbr, r.try_get::<f64,_>("weight").unwrap_or(0.0)));
        }
    }

    Ok(out)
}
