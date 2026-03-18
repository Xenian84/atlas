use atlas_types::raw::RawTx;
use atlas_types::facts::{TxFactsV1, Action};
use crate::module::{ParserModule, action_type, protocol};
use crate::config::ProgramsConfig;

/// Detect Stake Program operations: Delegate(2), Deactivate(5), Withdraw(4).
pub struct StakeOpsModule {
    pub cfg: ProgramsConfig,
}

impl ParserModule for StakeOpsModule {
    fn name(&self) -> &'static str { "stake_ops" }

    fn detect(&self, raw: &RawTx, facts: &mut TxFactsV1) {
        let keys = &raw.account_keys;
        for ix in &raw.instructions {
            let prog_idx = ix.program_id_index as usize;
            let Some(prog) = keys.get(prog_idx) else { continue };
            if !self.cfg.is_stake_program(&prog.pubkey) { continue }
            if ix.data.len() < 4 { continue }

            let disc = u32::from_le_bytes([ix.data[0], ix.data[1], ix.data[2], ix.data[3]]);
            let stake_account = ix.accounts.first()
                .and_then(|i| keys.get(*i as usize))
                .map(|k| k.pubkey.clone())
                .unwrap_or_default();

            match disc {
                2 => {
                    // Delegate
                    let validator = ix.accounts.get(1)
                        .and_then(|i| keys.get(*i as usize))
                        .map(|k| k.pubkey.clone());
                    let mut action = Action::new(action_type::STAKE, protocol::STAKE, &stake_account);
                    action.x    = validator;
                    action.meta = Some(serde_json::json!({ "op": "delegate" }));
                    facts.actions.push(action);
                }
                4 => {
                    // Withdraw
                    let mut action = Action::new(action_type::UNSTAKE, protocol::STAKE, &stake_account);
                    action.meta = Some(serde_json::json!({ "op": "withdraw" }));
                    facts.actions.push(action);
                }
                5 => {
                    // Deactivate
                    let mut action = Action::new(action_type::UNSTAKE, protocol::STAKE, &stake_account);
                    action.meta = Some(serde_json::json!({ "op": "deactivate" }));
                    facts.actions.push(action);
                }
                _ => {}
            }
        }
    }
}
