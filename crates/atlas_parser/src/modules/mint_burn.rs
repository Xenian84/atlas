use atlas_types::raw::RawTx;
use atlas_types::facts::{TxFactsV1, Action};
use crate::module::{ParserModule, action_type, protocol};
use crate::config::ProgramsConfig;

/// Detect SPL Token MintTo (7) and Burn (8) instructions.
pub struct MintBurnModule {
    pub cfg: ProgramsConfig,
}

impl ParserModule for MintBurnModule {
    fn name(&self) -> &'static str { "mint_burn" }

    fn detect(&self, raw: &RawTx, facts: &mut TxFactsV1) {
        let keys = &raw.account_keys;
        for ix in &raw.instructions {
            let prog_idx = ix.program_id_index as usize;
            let Some(prog) = keys.get(prog_idx) else { continue };
            if !self.cfg.is_token_program(&prog.pubkey) { continue }
            if ix.data.is_empty() { continue }

            match ix.data[0] {
                7 => {
                    // MintTo: [mint, dest, authority]
                    let authority = ix.accounts.get(2)
                        .and_then(|i| keys.get(*i as usize))
                        .map(|k| k.pubkey.clone())
                        .unwrap_or_default();
                    let mint = ix.accounts.first()
                        .and_then(|i| keys.get(*i as usize))
                        .map(|k| k.pubkey.clone())
                        .unwrap_or_default();
                    let amount = if ix.data.len() >= 9 {
                        u64::from_le_bytes(ix.data[1..9].try_into().unwrap_or([0u8;8]))
                    } else { 0 };

                    let mut action = Action::new(action_type::MINT, protocol::TOKEN, &authority);
                    action.amt = Some(serde_json::json!({ "mint": mint, "amount": amount }));
                    facts.actions.push(action);
                }
                8 => {
                    // Burn: [account, mint, owner]
                    let owner = ix.accounts.get(2)
                        .and_then(|i| keys.get(*i as usize))
                        .map(|k| k.pubkey.clone())
                        .unwrap_or_default();
                    let mint = ix.accounts.get(1)
                        .and_then(|i| keys.get(*i as usize))
                        .map(|k| k.pubkey.clone())
                        .unwrap_or_default();
                    let amount = if ix.data.len() >= 9 {
                        u64::from_le_bytes(ix.data[1..9].try_into().unwrap_or([0u8;8]))
                    } else { 0 };

                    let mut action = Action::new(action_type::BURN, protocol::TOKEN, &owner);
                    action.amt = Some(serde_json::json!({ "mint": mint, "amount": amount }));
                    facts.actions.push(action);
                }
                _ => {}
            }
        }
    }
}
