use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn};
use sqlx::PgPool;
use redis::aio::ConnectionManager;
use atlas_common::AppConfig;
use atlas_parser::Parser;

use crate::checkpoint;

pub async fn run_backfill(
    cfg:       AppConfig,
    pool:      PgPool,
    redis: ConnectionManager,
    parser:    Arc<Parser>,
    from_slot: u64,
    to_slot:   u64,
    batch:     usize,
) -> Result<()> {
    info!("Backfill: slots {} to {} (batch={})", from_slot, to_slot, batch);

    // Share a single reqwest::Client across the entire backfill for connection pooling
    let rpc_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // Use a semaphore to bound concurrent RPC calls and avoid rate-limiting
    let sem = Arc::new(tokio::sync::Semaphore::new(batch.min(8)));
    let mut current = from_slot;

    while current <= to_slot {
        // Use exclusive end so batch is exactly `batch` slots, not batch+1
        let end = (current + batch as u64 - 1).min(to_slot);

        let mut handles = vec![];
        for slot in current..=end {
            let rpc_url  = cfg.validator_rpc_url.clone();
            let client   = rpc_client.clone();
            let parser   = parser.clone();
            let pool     = pool.clone();
            let mut red  = redis.clone();
            let sem      = sem.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await;
                backfill_slot(&client, &rpc_url, slot, &parser, &pool, &mut red).await
            }));
        }

        for handle in handles {
            if let Err(e) = handle.await? {
                warn!("Backfill slot error: {:#}", e);
            }
        }

        if let Err(e) = checkpoint::update_backfill_progress(&pool, from_slot, to_slot, end).await {
            warn!("Backfill checkpoint error: {:#}", e);
        }
        current = end + 1;
        info!("Backfill progress: {}/{}", current.min(to_slot + 1), to_slot);
    }

    info!("Backfill complete: {} slots processed", to_slot - from_slot + 1);
    Ok(())
}

async fn backfill_slot(
    client:  &reqwest::Client,
    rpc_url: &str,
    slot:    u64,
    parser:  &Parser,
    pool:    &PgPool,
    redis:   &mut ConnectionManager,
) -> Result<()> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id":      1,
        "method":  "getBlock",
        "params":  [slot, {
            "encoding":                         "json",
            "maxSupportedTransactionVersion":   0,
            "transactionDetails":               "full",
            "rewards":                          false,
        }]
    });

    let resp: serde_json::Value = client.post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    let block = match resp.get("result") {
        Some(r) if !r.is_null() => r,
        _ => return Ok(()),
    };

    let block_time = block["blockTime"].as_i64();
    let txs = block["transactions"].as_array().cloned().unwrap_or_default();

    for (pos, tx_val) in txs.iter().enumerate() {
        if let Ok(Some(raw)) = parse_rpc_tx(tx_val, slot, pos as u32, block_time) {
            let facts = parser.parse(&raw);
            if let Err(e) = crate::stream::persist_tx_static(pool, redis, &facts).await {
                warn!("Backfill persist error slot={} sig={}: {:#}", slot, facts.sig, e);
            }
        }
    }

    Ok(())
}

/// Parse a getBlock transaction JSON into a RawTx.
/// Fully parses inner instructions, token balances, and loaded addresses (ALTs).
pub fn parse_rpc_tx(
    tx_val:     &serde_json::Value,
    slot:       u64,
    pos:        u32,
    block_time: Option<i64>,
) -> Result<Option<atlas_types::raw::RawTx>> {
    use atlas_types::raw::*;

    let meta = &tx_val["meta"];
    let tx   = &tx_val["transaction"];

    let sigs = tx["signatures"].as_array().cloned().unwrap_or_default();
    let sig  = sigs.first()
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    if sig.is_empty() { return Ok(None); }

    // Skip vote transactions
    if tx_val["transaction"]["message"]["instructions"]
        .as_array()
        .map(|ixs| ixs.iter().any(|ix| ix["programId"].as_str() == Some("Vote111111111111111111111111111111111111111111")))
        .unwrap_or(false)
    {
        return Ok(None);
    }

    let msg = &tx["message"];

    // Static account keys
    let static_keys: Vec<String> = msg["accountKeys"].as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    let header = &msg["header"];
    let n_req_sig = header["numRequiredSignatures"].as_u64().unwrap_or(0) as usize;
    let n_ro_sig  = header["numReadonlySignedAccounts"].as_u64().unwrap_or(0) as usize;
    let n_ro_uns  = header["numReadonlyUnsignedAccounts"].as_u64().unwrap_or(0) as usize;
    let static_count = static_keys.len();

    let mut account_keys: Vec<RawAccountKey> = static_keys.iter().enumerate().map(|(i, k)| {
        let is_signer   = i < n_req_sig;
        let is_writable = if is_signer {
            i < n_req_sig - n_ro_sig
        } else {
            i < static_count - n_ro_uns
        };
        RawAccountKey { pubkey: k.clone(), is_signer, is_writable }
    }).collect();

    // Parse and merge loaded ALT addresses
    let mut loaded_writable: Vec<String> = vec![];
    let mut loaded_readonly: Vec<String> = vec![];
    if let Some(la) = meta["loadedAddresses"].as_object() {
        if let Some(ws) = la.get("writable").and_then(|v| v.as_array()) {
            for addr in ws {
                if let Some(s) = addr.as_str() {
                    loaded_writable.push(s.to_string());
                    account_keys.push(RawAccountKey { pubkey: s.to_string(), is_signer: false, is_writable: true });
                }
            }
        }
        if let Some(rs) = la.get("readonly").and_then(|v| v.as_array()) {
            for addr in rs {
                if let Some(s) = addr.as_str() {
                    loaded_readonly.push(s.to_string());
                    account_keys.push(RawAccountKey { pubkey: s.to_string(), is_signer: false, is_writable: false });
                }
            }
        }
    }

    let parse_ix = |ix: &serde_json::Value| RawInstruction {
        program_id_index: ix["programIdIndex"].as_u64().unwrap_or(0) as u8,
        accounts: ix["accounts"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| v.as_u64().unwrap_or(0) as u8)
            .collect(),
        data: bs58::decode(ix["data"].as_str().unwrap_or(""))
            .into_vec()
            .unwrap_or_default(),
    };

    let instructions: Vec<RawInstruction> = msg["instructions"].as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|ix| parse_ix(ix))
        .collect();

    // Inner instructions (CPI calls)
    let inner_instructions: Vec<RawInnerInstruction> = meta["innerInstructions"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|inner| RawInnerInstruction {
            index: inner["index"].as_u64().unwrap_or(0) as u8,
            instructions: inner["instructions"].as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|ix| parse_ix(ix))
                .collect(),
        })
        .collect();

    let pre_balances:  Vec<u64> = meta["preBalances"].as_array().unwrap_or(&vec![])
        .iter().map(|v| v.as_u64().unwrap_or(0)).collect();
    let post_balances: Vec<u64> = meta["postBalances"].as_array().unwrap_or(&vec![])
        .iter().map(|v| v.as_u64().unwrap_or(0)).collect();

    let parse_token_bal = |arr: &serde_json::Value| -> Vec<RawTokenBalance> {
        arr.as_array().unwrap_or(&vec![]).iter().filter_map(|b| {
            let ui = &b["uiTokenAmount"];
            Some(RawTokenBalance {
                account_index: b["accountIndex"].as_u64()? as u8,
                mint:    b["mint"].as_str()?.to_string(),
                owner:   b["owner"].as_str().unwrap_or("").to_string(),
                ui_amount: ui["uiAmount"].as_f64(),
                amount:  ui["amount"].as_str().unwrap_or("0").to_string(),
                decimals: ui["decimals"].as_u64().unwrap_or(0) as u8,
            })
        }).collect()
    };

    let pre_token_balances  = parse_token_bal(&meta["preTokenBalances"]);
    let post_token_balances = parse_token_bal(&meta["postTokenBalances"]);

    let log_messages: Vec<String> = meta["logMessages"].as_array().unwrap_or(&vec![])
        .iter().filter_map(|v| v.as_str().map(String::from)).collect();

    let err = if meta["err"].is_null() {
        None
    } else {
        Some(serde_json::to_string(&meta["err"]).unwrap_or_else(|_| "failed".to_string()))
    };

    Ok(Some(RawTx {
        sig,
        slot,
        pos,
        block_time,
        is_vote: false,
        err,
        fee: meta["fee"].as_u64().unwrap_or(0),
        compute_units_consumed: meta["computeUnitsConsumed"].as_u64(),
        compute_units_limit:         None,  // filled by ComputeBudgetModule
        priority_fee_micro_lamports: None,  // filled by ComputeBudgetModule
        account_keys,
        instructions,
        inner_instructions,
        pre_balances,
        post_balances,
        pre_token_balances,
        post_token_balances,
        log_messages,
        loaded_addresses: RawLoadedAddresses {
            writable: loaded_writable,
            readonly: loaded_readonly,
        },
    }))
}
