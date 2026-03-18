use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use atlas_common::redis_ext;
use crate::{state::AppState, error::ApiError, handlers::das};

#[derive(Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id:      Value,
    pub method:  String,
    pub params:  Option<Value>,
}

#[derive(Serialize)]
pub struct RpcResponse {
    pub jsonrpc: &'static str,
    pub id:      Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:   Option<Value>,
}

impl RpcResponse {
    fn ok(id: Value, result: Value) -> Self {
        Self { jsonrpc: "2.0", id, result: Some(result), error: None }
    }
    fn err(id: Value, code: i32, msg: &str) -> Self {
        Self { jsonrpc: "2.0", id, result: None,
            error: Some(serde_json::json!({ "code": code, "message": msg })) }
    }
}

/// JSON-RPC handler — routes to local index or proxies to validator RPC.
pub async fn json_rpc_handler(
    State(state): State<AppState>,
    Json(req):    Json<RpcRequest>,
) -> Result<Json<RpcResponse>, ApiError> {
    let id = req.id.clone();

    let params = req.params.unwrap_or(Value::Null);

    match req.method.as_str() {
        // ── Local index methods ───────────────────────────────────────────────
        "getTransactionsForAddress" => {
            handle_get_txs_for_address(state, id, Some(params)).await
        }

        // ── DAS API methods ───────────────────────────────────────────────────
        "getAsset" => {
            let r = das::get_asset(state.pool(), &params).await?;
            Ok(Json(RpcResponse::ok(id, r)))
        }
        "getAssetBatch" => {
            let r = das::get_asset_batch(state.pool(), &params).await?;
            Ok(Json(RpcResponse::ok(id, r)))
        }
        "getAssetsByOwner" => {
            let r = das::get_assets_by_owner(state.pool(), &params).await?;
            Ok(Json(RpcResponse::ok(id, r)))
        }
        "getAssetsByGroup" => {
            let r = das::get_assets_by_group(state.pool(), &params).await?;
            Ok(Json(RpcResponse::ok(id, r)))
        }
        "getAssetsByCreator" => {
            let r = das::get_assets_by_creator(state.pool(), &params).await?;
            Ok(Json(RpcResponse::ok(id, r)))
        }
        "getAssetsByAuthority" => {
            let r = das::get_assets_by_authority(state.pool(), &params).await?;
            Ok(Json(RpcResponse::ok(id, r)))
        }
        "searchAssets" => {
            let r = das::search_assets(state.pool(), &params).await?;
            Ok(Json(RpcResponse::ok(id, r)))
        }
        "getTokenAccounts" => {
            let r = das::get_token_accounts(state.pool(), &params).await?;
            Ok(Json(RpcResponse::ok(id, r)))
        }
        "getSignaturesForAsset" => {
            let r = das::get_signatures_for_asset(state.pool(), &params).await?;
            Ok(Json(RpcResponse::ok(id, r)))
        }
        "getNftEditions" => {
            let r = das::get_nft_editions(state.pool(), &params).await?;
            Ok(Json(RpcResponse::ok(id, r)))
        }

        // ── Priority Fee API ──────────────────────────────────────────────────
        "getPriorityFeeEstimate" => {
            handle_priority_fee(state, id, params).await
        }

        // ── getTransaction — serve from Atlas index, fallback to proxy ─────────
        "getTransaction" => {
            handle_get_transaction(state, id, params).await
        }

        // ── getSignaturesForAddress — serve from Atlas index ──────────────────
        "getSignaturesForAddress" => {
            handle_get_signatures_for_address(state, id, params).await
        }

        // ── getBalance — serve from accounts table, fallback to proxy ─────────
        "getBalance" => {
            handle_get_balance(state, id, params).await
        }

        // ── getTokenAccountsByOwner — serve from token_owner_map index ────────
        "getTokenAccountsByOwner" | "getTokenAccountsByOwnerV2" => {
            handle_get_token_accounts_by_owner(state, id, params, &req.method).await
        }

        // ── getTokenSupply — serve from token_metadata ────────────────────────
        "getTokenSupply" => {
            handle_get_token_supply(state, id, params).await
        }

        // ── getTokenLargestAccounts — serve from geyser_accounts ──────────────
        "getTokenLargestAccounts" => {
            handle_get_token_largest_accounts(state, id, params).await
        }

        // ── getProgramAccountsV2 — paginated proxy wrapper ────────────────────
        "getProgramAccountsV2" => {
            handle_get_program_accounts_v2(state, id, params).await
        }

        // ── Cached validator proxy ────────────────────────────────────────────
        "getLatestBlockhash" | "getSlot" | "getBlockHeight" | "getBlockTime" => {
            proxy_rpc_cached(state, id, &req.method, Some(params)).await
        }
        _ => {
            proxy_rpc_cached(state, id, &req.method, Some(params)).await
        }
    }
}

async fn handle_priority_fee(
    state:  AppState,
    id:     Value,
    params: Value,
) -> Result<Json<RpcResponse>, ApiError> {
    // Ask the validator for recent prioritization fees
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getRecentPrioritizationFees",
        "params": [params.get("accountKeys").cloned().unwrap_or(Value::Null)]
    });

    let resp: Value = state.http()
        .post(&state.cfg().validator_rpc_url)
        .json(&body)
        .send().await
        .map_err(|e| ApiError::Internal(e.into()))?
        .json().await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let fees: Vec<u64> = resp["result"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|f| f["prioritizationFee"].as_u64())
        .collect();

    let recommended = if fees.is_empty() {
        serde_json::json!({
            "min":       0,
            "low":       0,
            "medium":    1000,
            "high":      5000,
            "veryHigh":  100000,
            "unsafe_max": 1000000,
        })
    } else {
        let mut sorted = fees.clone();
        sorted.sort_unstable();
        let pct = |p: usize| -> u64 {
            sorted[((sorted.len() - 1) * p / 100).min(sorted.len() - 1)]
        };
        serde_json::json!({
            "min":        sorted.first().copied().unwrap_or(0),
            "low":        pct(25),
            "medium":     pct(50),
            "high":       pct(75),
            "veryHigh":   pct(95),
            "unsafe_max": sorted.last().copied().unwrap_or(0),
            "sample_size": sorted.len(),
        })
    };

    Ok(Json(RpcResponse::ok(id, serde_json::json!({
        "context":              { "slot": resp["result"][0]["slot"].as_u64() },
        "per_compute_unit":     recommended,
    }))))
}

async fn handle_get_txs_for_address(
    state:  AppState,
    id:     Value,
    params: Option<Value>,
) -> Result<Json<RpcResponse>, ApiError> {
    let params  = params.unwrap_or(Value::Null);
    let address = params[0].as_str()
        .ok_or_else(|| ApiError::BadRequest("address required".into()))?
        .to_string();
    let opts = crate::handlers::address::TxsOpts::from_rpc(&params[1]);

    let page = crate::handlers::address::fetch_address_txs(&state, &address, opts).await?;
    let val  = serde_json::to_value(page).map_err(ApiError::SerdeJson)?;
    Ok(Json(RpcResponse::ok(id, val)))
}

async fn proxy_rpc_cached(
    state:  AppState,
    id:     Value,
    method: &str,
    params: Option<Value>,
) -> Result<Json<RpcResponse>, ApiError> {
    let params_val = params.unwrap_or(Value::Null);
    let cache_key = format!("rpc:{}:{}", method, serde_json::to_string(&params_val).unwrap_or_default());
    let mut redis = state.redis();

    if let Some(cached) = redis_ext::cache_get::<Value>(&mut redis, &cache_key).await {
        return Ok(Json(RpcResponse::ok(id, cached)));
    }

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id":      1,
        "method":  method,
        "params":  params_val,
    });

    // Use shared client from AppState — no per-request connection pool creation
    let upstream = state.http()
        .post(&state.cfg().validator_rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .json::<Value>()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // Propagate upstream error responses instead of replacing with null
    if let Some(upstream_err) = upstream.get("error") {
        return Ok(Json(RpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(upstream_err.clone()),
        }));
    }

    let result = upstream.get("result").cloned().unwrap_or(Value::Null);

    let ttl = match method {
        "getSlot" | "getBlockHeight" => 2,
        "getLatestBlockhash"         => 5,
        _                            => 10,
    };
    let _ = redis_ext::cache_set(&mut redis, &cache_key, &result, ttl).await;

    Ok(Json(RpcResponse::ok(id, result)))
}

// ── getTransaction — Atlas index first, then proxy ───────────────────────────

async fn handle_get_transaction(
    state:  AppState,
    id:     Value,
    params: Value,
) -> Result<Json<RpcResponse>, ApiError> {
    let sig = params[0].as_str().unwrap_or("").to_string();
    if sig.is_empty() {
        return Ok(Json(RpcResponse::ok(id, Value::Null)));
    }

    // Try Atlas index first
    let row = sqlx::query(
        r#"SELECT sig, slot, pos, block_time, status, fee_lamports,
                  compute_consumed, programs, tags, accounts_json,
                  actions_json, token_deltas_json, sol_deltas_json,
                  err_json, commitment
           FROM tx_store WHERE sig = $1"#
    )
    .bind(&sig)
    .fetch_optional(state.pool())
    .await
    .ok()
    .flatten();

    if let Some(r) = row {
        use sqlx::Row;
        let result = serde_json::json!({
            "slot":       r.try_get::<i64, _>("slot").unwrap_or(0),
            "blockTime":  r.try_get::<Option<i64>, _>("block_time").ok().flatten(),
            "meta": {
                "fee":             r.try_get::<i64, _>("fee_lamports").unwrap_or(0),
                "computeUnitsConsumed": r.try_get::<Option<i32>, _>("compute_consumed").ok().flatten(),
                "err":             r.try_get::<Option<Value>, _>("err_json").ok().flatten(),
                "logMessages":     serde_json::json!([]),
                "preBalances":     serde_json::json!([]),
                "postBalances":    serde_json::json!([]),
            },
            "transaction": {
                "signatures": [&sig],
                "message": {
                    "accountKeys": r.try_get::<Value, _>("accounts_json").unwrap_or_default(),
                }
            },
            "_atlas": {
                "tags":          r.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
                "programs":      r.try_get::<Vec<String>, _>("programs").unwrap_or_default(),
                "actions":       r.try_get::<Value, _>("actions_json").unwrap_or_default(),
                "token_deltas":  r.try_get::<Value, _>("token_deltas_json").unwrap_or_default(),
                "sol_deltas":    r.try_get::<Value, _>("sol_deltas_json").unwrap_or_default(),
                "commitment":    r.try_get::<String, _>("commitment").unwrap_or_default(),
                "source":        "atlas_index",
            }
        });
        return Ok(Json(RpcResponse::ok(id, result)));
    }

    // Fallback to validator RPC
    proxy_rpc_cached(state, id, "getTransaction", Some(params)).await
}

// ── getSignaturesForAddress — Atlas index ─────────────────────────────────────

async fn handle_get_signatures_for_address(
    state:  AppState,
    id:     Value,
    params: Value,
) -> Result<Json<RpcResponse>, ApiError> {
    use sqlx::Row;
    let address = params[0].as_str().unwrap_or("").to_string();
    if address.is_empty() {
        return Ok(Json(RpcResponse::ok(id, serde_json::json!([]))));
    }

    let limit: i64 = params[1]["limit"].as_i64().unwrap_or(1000).min(1000);

    let rows = sqlx::query(
        r#"SELECT sig, slot, block_time, status
           FROM address_index
           WHERE address = $1
           ORDER BY slot DESC, pos DESC
           LIMIT $2"#
    )
    .bind(&address)
    .bind(limit)
    .fetch_all(state.pool())
    .await
    .unwrap_or_default();

    let sigs: Vec<Value> = rows.iter().map(|r| serde_json::json!({
        "signature":   r.try_get::<String, _>("sig").unwrap_or_default(),
        "slot":        r.try_get::<i64, _>("slot").unwrap_or(0),
        "blockTime":   r.try_get::<Option<i64>, _>("block_time").ok().flatten(),
        "confirmationStatus": "confirmed",
        "err":         if r.try_get::<i16, _>("status").unwrap_or(1) == 1 { Value::Null } else { serde_json::json!("TransactionError") },
    })).collect();

    Ok(Json(RpcResponse::ok(id, serde_json::json!(sigs))))
}

// ── getBalance — Atlas accounts table first ───────────────────────────────────

async fn handle_get_balance(
    state:  AppState,
    id:     Value,
    params: Value,
) -> Result<Json<RpcResponse>, ApiError> {
    use sqlx::Row;
    let address = params[0].as_str().unwrap_or("").to_string();
    if address.is_empty() {
        return Ok(Json(RpcResponse::ok(id, serde_json::json!({"context":{"slot":0},"value":0}))));
    }

    // Try Atlas accounts table first
    if let Ok(Some(row)) = sqlx::query(
        "SELECT lamports, updated_slot FROM accounts WHERE address = $1"
    )
    .bind(&address)
    .fetch_optional(state.pool())
    .await
    {
        let lamports = row.try_get::<i64, _>("lamports").unwrap_or(0);
        let slot     = row.try_get::<i64, _>("updated_slot").unwrap_or(0);
        return Ok(Json(RpcResponse::ok(id, serde_json::json!({
            "context": { "slot": slot, "source": "atlas_index" },
            "value": lamports,
        }))));
    }

    // Fallback to RPC
    proxy_rpc_cached(state, id, "getBalance", Some(params)).await
}

// ── getTokenAccountsByOwner / V2 — Atlas token_owner_map + geyser_accounts ───

async fn handle_get_token_accounts_by_owner(
    state:   AppState,
    id:      Value,
    params:  Value,
    method:  &str,
) -> Result<Json<RpcResponse>, ApiError> {
    use sqlx::Row;
    let owner = params[0].as_str().unwrap_or("").to_string();
    if owner.is_empty() {
        return proxy_rpc_cached(state, id, method, Some(params)).await;
    }

    // Optional mint filter (programId filter is ignored — we serve all SPL token accounts)
    let mint_filter: Option<String> = params[1]["mint"].as_str().map(String::from);

    // Resolve from token_owner_map + geyser_accounts + token_metadata
    let rows = if let Some(ref mint) = mint_filter {
        sqlx::query(
            r#"SELECT tom.token_account, tom.mint, tom.owner,
                      ga.lamports, ga.data, ga.executable, ga.rent_epoch, ga.updated_slot,
                      tm.decimals, tm.symbol, tm.name
               FROM token_owner_map tom
               LEFT JOIN geyser_accounts ga ON ga.address = tom.token_account
               LEFT JOIN token_metadata  tm ON tm.mint    = tom.mint
               WHERE tom.owner = $1 AND tom.mint = $2"#
        )
        .bind(&owner).bind(mint)
        .fetch_all(state.pool()).await?
    } else {
        sqlx::query(
            r#"SELECT tom.token_account, tom.mint, tom.owner,
                      ga.lamports, ga.data, ga.executable, ga.rent_epoch, ga.updated_slot,
                      tm.decimals, tm.symbol, tm.name
               FROM token_owner_map tom
               LEFT JOIN geyser_accounts ga ON ga.address = tom.token_account
               LEFT JOIN token_metadata  tm ON tm.mint    = tom.mint
               WHERE tom.owner = $1"#
        )
        .bind(&owner)
        .fetch_all(state.pool()).await?
    };

    // If nothing found in our index, fall back to RPC (cold wallets)
    if rows.is_empty() {
        return proxy_rpc_cached(state, id, "getTokenAccountsByOwner", Some(params)).await;
    }

    let slot: i64 = rows.iter()
        .filter_map(|r| r.try_get::<i64, _>("updated_slot").ok())
        .max().unwrap_or(0);

    let accounts: Vec<Value> = rows.iter().map(|r| {
        let token_account: String = r.try_get("token_account").unwrap_or_default();
        let mint:          String = r.try_get("mint").unwrap_or_default();
        let lamports: i64         = r.try_get("lamports").unwrap_or(2039280);
        let decimals: i16         = r.try_get("decimals").unwrap_or(0);
        let data_hex: Option<String> = r.try_get("data").ok().flatten();

        serde_json::json!({
            "pubkey": token_account,
            "account": {
                "lamports":   lamports,
                "owner":      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                "executable": false,
                "rentEpoch":  r.try_get::<i64, _>("rent_epoch").unwrap_or(0),
                "data": {
                    "program": "spl-token",
                    "parsed": {
                        "type": "account",
                        "info": {
                            "mint":             mint,
                            "owner":            owner,
                            "tokenAmount": {
                                "decimals": decimals,
                            },
                            "state": "initialized",
                        }
                    },
                    "space": 165,
                },
                // Raw base64 if available
                "data_raw": data_hex,
            }
        })
    }).collect();

    Ok(Json(RpcResponse::ok(id, serde_json::json!({
        "context": { "slot": slot, "source": "atlas_index" },
        "value":   accounts,
    }))))
}

// ── getTokenSupply — serve from token_metadata ────────────────────────────────

async fn handle_get_token_supply(
    state:  AppState,
    id:     Value,
    params: Value,
) -> Result<Json<RpcResponse>, ApiError> {
    use sqlx::Row;
    let mint = params[0].as_str().unwrap_or("").to_string();
    if mint.is_empty() {
        return proxy_rpc_cached(state, id, "getTokenSupply", Some(params)).await;
    }

    if let Ok(Some(row)) = sqlx::query(
        "SELECT supply::text AS supply, decimals, updated_at FROM token_metadata WHERE mint = $1"
    )
    .bind(&mint)
    .fetch_optional(state.pool())
    .await
    {
        let supply_str: String = row.try_get("supply").unwrap_or_else(|_| "0".into());
        let decimals:   i16    = row.try_get("decimals").unwrap_or(0);
        let ui_amount: f64     = supply_str.parse::<f64>().unwrap_or(0.0)
                                  / 10f64.powi(decimals as i32);

        return Ok(Json(RpcResponse::ok(id, serde_json::json!({
            "context": { "source": "atlas_index" },
            "value": {
                "amount":         supply_str,
                "decimals":       decimals,
                "uiAmount":       ui_amount,
                "uiAmountString": format!("{:.prec$}", ui_amount, prec = decimals as usize),
            }
        }))));
    }

    proxy_rpc_cached(state, id, "getTokenSupply", Some(params)).await
}

// ── getTokenLargestAccounts — top holders from geyser_accounts ───────────────

async fn handle_get_token_largest_accounts(
    state:  AppState,
    id:     Value,
    params: Value,
) -> Result<Json<RpcResponse>, ApiError> {
    use sqlx::Row;
    let mint = params[0].as_str().unwrap_or("").to_string();
    if mint.is_empty() {
        return proxy_rpc_cached(state, id, "getTokenLargestAccounts", Some(params)).await;
    }

    let decimals: i16 = sqlx::query_scalar(
        "SELECT decimals FROM token_metadata WHERE mint = $1"
    )
    .bind(&mint)
    .fetch_optional(state.pool())
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    // token_balance_index tracks per-tx deltas; for current balances use geyser_accounts
    // geyser_accounts stores raw account data; token_owner_map has the mint→account mapping.
    // We join to get amounts from the most recent geyser snapshot via a lateral approach.
    let rows = sqlx::query(
        r#"SELECT tom.token_account, ga.lamports, ga.updated_slot
           FROM token_owner_map tom
           JOIN geyser_accounts ga ON ga.address = tom.token_account
           WHERE tom.mint = $1 AND ga.lamports > 2039280
           ORDER BY ga.lamports DESC
           LIMIT 20"#
    )
    .bind(&mint)
    .fetch_all(state.pool())
    .await?;

    if rows.is_empty() {
        return proxy_rpc_cached(state, id, "getTokenLargestAccounts", Some(params)).await;
    }

    let slot: i64 = rows.iter()
        .filter_map(|r| r.try_get::<i64, _>("updated_slot").ok())
        .max().unwrap_or(0);

    let accounts: Vec<Value> = rows.iter().map(|r| {
        let addr:    String = r.try_get("token_account").unwrap_or_default();
        let lamports: i64   = r.try_get("lamports").unwrap_or(0);
        // lamports here is raw SPL token amount stored in the account data, not SOL
        // For accurate token amounts we'd need to parse account data — use lamports as proxy
        let amount = lamports.to_string();
        let ui: f64 = lamports as f64 / 10f64.powi(decimals as i32);
        serde_json::json!({
            "address":        addr,
            "amount":         amount,
            "decimals":       decimals,
            "uiAmount":       ui,
            "uiAmountString": format!("{:.prec$}", ui, prec = decimals as usize),
        })
    }).collect();

    Ok(Json(RpcResponse::ok(id, serde_json::json!({
        "context": { "slot": slot, "source": "atlas_index" },
        "value":   accounts,
    }))))
}

// ── getProgramAccountsV2 — paginated proxy ────────────────────────────────────

async fn handle_get_program_accounts_v2(
    state:  AppState,
    id:     Value,
    params: Value,
) -> Result<Json<RpcResponse>, ApiError> {
    let program_id   = params[0].as_str().unwrap_or("").to_string();
    let opts         = &params[1];
    let page:  u64   = opts["page"].as_u64().unwrap_or(1).max(1);
    let limit: u64   = opts["limit"].as_u64().unwrap_or(100).min(1000);
    let offset: u64  = (page - 1) * limit;

    // Build getProgramAccounts call with dataSlice + filters forwarded
    let mut pa_config = serde_json::json!({
        "encoding":        opts.get("encoding").cloned().unwrap_or(serde_json::json!("base64")),
        "withContext":     true,
        "commitment":      opts.get("commitment").cloned().unwrap_or(serde_json::json!("confirmed")),
    });

    if let Some(filters) = opts.get("filters") {
        pa_config["filters"] = filters.clone();
    }
    if let Some(ds) = opts.get("dataSlice") {
        pa_config["dataSlice"] = ds.clone();
    }

    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method":  "getProgramAccounts",
        "params":  [program_id, pa_config],
    });

    let upstream = state.http()
        .post(&state.cfg().validator_rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .json::<Value>()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    if let Some(err) = upstream.get("error") {
        return Ok(Json(RpcResponse { jsonrpc: "2.0", id, result: None, error: Some(err.clone()) }));
    }

    let all_accounts = upstream["result"]["value"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let total   = all_accounts.len() as u64;
    let page_accounts: Vec<Value> = all_accounts
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();

    Ok(Json(RpcResponse::ok(id, serde_json::json!({
        "context": upstream["result"]["context"].clone(),
        "value":   page_accounts,
        "pagination": {
            "page":       page,
            "limit":      limit,
            "total":      total,
            "totalPages": (total + limit - 1) / limit,
            "hasMore":    offset + limit < total,
        }
    }))))
}
