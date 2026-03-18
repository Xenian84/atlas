use atlas_types::raw::{RawTx, RawAccountKey, RawInstruction, RawInnerInstruction, RawTokenBalance, RawLoadedAddresses};
use anyhow::{Result, bail};

/// Convert a Yellowstone gRPC SubscribeUpdateTransaction into a RawTx.
/// Returns None for vote transactions (filtered upstream or here).
pub fn convert_grpc_tx(
    slot: u64,
    pos: u32,
    update: &yellowstone_grpc_proto::geyser::SubscribeUpdateTransactionInfo,
    block_time: Option<i64>,
) -> Result<Option<RawTx>> {
    let tx = match &update.transaction {
        Some(t) => t,
        None => bail!("missing transaction in update"),
    };
    let meta = match &update.meta {
        Some(m) => m,
        None => bail!("missing meta in update"),
    };

    let sig = bs58_encode(&update.signature);

    if update.is_vote { return Ok(None); }

    let msg = tx.message.as_ref().ok_or_else(|| anyhow::anyhow!("no message"))?;
    let header = msg.header.as_ref().ok_or_else(|| anyhow::anyhow!("no header"))?;

    let num_required_signatures = header.num_required_signatures as usize;
    let num_readonly_signed     = header.num_readonly_signed_accounts as usize;
    let num_readonly_unsigned   = header.num_readonly_unsigned_accounts as usize;
    let static_count            = msg.account_keys.len();

    // Static account keys
    let mut account_keys: Vec<RawAccountKey> = msg.account_keys.iter().enumerate().map(|(i, k)| {
        let is_signer   = i < num_required_signatures;
        let is_writable = if is_signer {
            i < num_required_signatures - num_readonly_signed
        } else {
            i < static_count - num_readonly_unsigned
        };
        RawAccountKey { pubkey: bs58_encode(k), is_signer, is_writable }
    }).collect();

    // Merge ALT-resolved addresses into account_keys so all modules can resolve
    // any account index correctly (including those >= static_count).
    for addr in &meta.loaded_writable_addresses {
        account_keys.push(RawAccountKey {
            pubkey:     bs58_encode(addr),
            is_signer:  false,
            is_writable: true,
        });
    }
    for addr in &meta.loaded_readonly_addresses {
        account_keys.push(RawAccountKey {
            pubkey:     bs58_encode(addr),
            is_signer:  false,
            is_writable: false,
        });
    }

    let loaded_addresses = RawLoadedAddresses {
        writable: meta.loaded_writable_addresses.iter().map(|b| bs58_encode(b)).collect(),
        readonly: meta.loaded_readonly_addresses.iter().map(|b| bs58_encode(b)).collect(),
    };

    let instructions: Vec<RawInstruction> = msg.instructions.iter().map(|ix| RawInstruction {
        program_id_index: ix.program_id_index as u8,
        accounts:         ix.accounts.iter().map(|b| *b as u8).collect(),
        data:             ix.data.clone(),
    }).collect();

    let inner_instructions: Vec<RawInnerInstruction> = meta.inner_instructions.iter()
        .map(|inner| RawInnerInstruction {
            index: inner.index as u8,
            instructions: inner.instructions.iter().map(|ix| RawInstruction {
                program_id_index: ix.program_id_index as u8,
                accounts:         ix.accounts.iter().map(|b| *b as u8).collect(),
                data:             ix.data.clone(),
            }).collect(),
        })
        .collect();

    let pre_token_balances  = convert_token_balances(&meta.pre_token_balances);
    let post_token_balances = convert_token_balances(&meta.post_token_balances);

    let compute_units_consumed = meta.compute_units_consumed;

    // Preserve the actual error message from the transaction error
    let err = meta.err.as_ref().map(|e| {
        serde_json::to_string(&e.err).unwrap_or_else(|_| "transaction failed".to_string())
    });

    Ok(Some(RawTx {
        sig,
        slot,
        pos,
        block_time,
        is_vote: update.is_vote,
        err,
        fee: meta.fee,
        compute_units_consumed,
        compute_units_limit: None,          // filled by compute_budget parser module
        priority_fee_micro_lamports: None,  // filled by compute_budget parser module
        account_keys,
        instructions,
        inner_instructions,
        pre_balances:   meta.pre_balances.clone(),
        post_balances:  meta.post_balances.clone(),
        pre_token_balances,
        post_token_balances,
        log_messages:   meta.log_messages.clone(),
        loaded_addresses,
    }))
}

fn convert_token_balances(
    balances: &[yellowstone_grpc_proto::solana::storage::confirmed_block::TokenBalance],
) -> Vec<RawTokenBalance> {
    balances.iter().filter_map(|b| {
        let ui = b.ui_token_amount.as_ref()?;
        Some(RawTokenBalance {
            account_index: b.account_index as u8,
            mint:          b.mint.clone(),
            owner:         b.owner.clone(),
            ui_amount:     Some(ui.ui_amount),
            amount:        ui.amount.clone(),
            decimals:      ui.decimals as u8,
        })
    }).collect()
}

fn bs58_encode(bytes: &[u8]) -> String {
    bs58::encode(bytes).into_string()
}
