use atlas_types::raw::RawTx;
use atlas_types::facts::{TxFactsV1, Action, TokenDelta, DeltaDirection};
use crate::module::{ParserModule, action_type, protocol};
use crate::config::ProgramsConfig;

/// Detect SPL token transfers (Transfer=3, TransferChecked=12).
pub struct TokenTransferModule {
    pub cfg: ProgramsConfig,
}

impl ParserModule for TokenTransferModule {
    fn name(&self) -> &'static str { "token_transfer" }

    fn detect(&self, raw: &RawTx, facts: &mut TxFactsV1) {
        let keys = &raw.account_keys;
        for ix in &raw.instructions {
            let prog_idx = ix.program_id_index as usize;
            let Some(prog) = keys.get(prog_idx) else { continue };
            if !self.cfg.is_token_program(&prog.pubkey) { continue }
            if ix.data.is_empty() { continue }

            // discriminant: 3=Transfer, 12=TransferChecked
            let disc = ix.data[0];
            if disc != 3 && disc != 12 { continue }

            let from = ix.accounts.first()
                .and_then(|i| keys.get(*i as usize))
                .map(|k| k.pubkey.clone())
                .unwrap_or_default();

            let to = if disc == 3 {
                ix.accounts.get(1)
            } else {
                ix.accounts.get(2)  // TransferChecked: [src, mint, dst, auth]
            }
            .and_then(|i| keys.get(*i as usize))
            .map(|k| k.pubkey.clone())
            .unwrap_or_default();

            // Extract raw amount: bytes 1..9 as little-endian u64
            let raw_amount: Option<u64> = if ix.data.len() >= 9 {
                Some(u64::from_le_bytes([
                    ix.data[1], ix.data[2], ix.data[3], ix.data[4],
                    ix.data[5], ix.data[6], ix.data[7], ix.data[8],
                ]))
            } else {
                None
            };

            let mut action = Action::new(action_type::TRANSFER, protocol::TOKEN, &from);
            action.x = Some(to.clone());

            // Enrich with mint info + actual raw amount from instruction data
            if let Some(pre) = raw.pre_token_balances.iter()
                .find(|b| keys.get(b.account_index as usize)
                    .map(|k| k.pubkey == from).unwrap_or(false))
            {
                action.amt = Some(serde_json::json!({
                    "mint":       pre.mint,
                    "decimals":   pre.decimals,
                    "raw_amount": raw_amount,
                }));
            } else if let Some(amount) = raw_amount {
                action.amt = Some(serde_json::json!({ "raw_amount": amount }));
            }

            facts.actions.push(action);
        }

        // Compute token deltas from pre/post balances
        self.compute_token_deltas(raw, facts);
    }
}

impl TokenTransferModule {
    fn compute_token_deltas(&self, raw: &RawTx, facts: &mut TxFactsV1) {
        use std::collections::HashMap;
        let keys = &raw.account_keys;

        let pre_map: HashMap<u8, &atlas_types::raw::RawTokenBalance> =
            raw.pre_token_balances.iter().map(|b| (b.account_index, b)).collect();

        for post in &raw.post_token_balances {
            let pre_amount: u64 = pre_map.get(&post.account_index)
                .and_then(|p| p.amount.parse().ok())
                .unwrap_or(0);
            let post_amount: u64 = post.amount.parse().unwrap_or(0);
            if pre_amount == post_amount { continue }

            let delta_i64 = post_amount as i64 - pre_amount as i64;
            let direction = if delta_i64 > 0 { DeltaDirection::In } else { DeltaDirection::Out };

            let account = keys.get(post.account_index as usize)
                .map(|k| k.pubkey.clone())
                .unwrap_or_default();

            facts.token_deltas.push(TokenDelta {
                mint:      post.mint.clone(),
                owner:     post.owner.clone(),
                account,
                pre:       pre_amount.to_string(),
                post:      post_amount.to_string(),
                delta:     delta_i64.to_string(),
                decimals:  post.decimals,
                symbol:    None,
                direction,
            });
        }
    }
}
