use atlas_types::raw::RawTx;
use atlas_types::facts::{TxFactsV1, Action};
use crate::module::{ParserModule, action_type, protocol};
use crate::config::ProgramsConfig;

/// Detect BPF Upgradeable Loader deploys (DeployWithMaxDataLen=1, Upgrade=3).
pub struct DeployDetectModule {
    pub cfg: ProgramsConfig,
}

impl ParserModule for DeployDetectModule {
    fn name(&self) -> &'static str { "deploy_detect" }

    fn detect(&self, raw: &RawTx, facts: &mut TxFactsV1) {
        let keys = &raw.account_keys;
        for ix in &raw.instructions {
            let prog_idx = ix.program_id_index as usize;
            let Some(prog) = keys.get(prog_idx) else { continue };
            if !self.cfg.is_bpf_upgradeable(&prog.pubkey) { continue }
            if ix.data.len() < 4 { continue }

            let disc = u32::from_le_bytes([ix.data[0], ix.data[1], ix.data[2], ix.data[3]]);
            let deployer = raw.account_keys.first()
                .map(|k| k.pubkey.clone())
                .unwrap_or_default();

            match disc {
                1 | 3 => {
                    let deployed_program = ix.accounts.first()
                        .and_then(|i| keys.get(*i as usize))
                        .map(|k| k.pubkey.clone());

                    let mut action = Action::new(action_type::DEPLOY, protocol::UNKNOWN, &deployer);
                    action.x    = deployed_program;
                    action.meta = Some(serde_json::json!({
                        "op": if disc == 1 { "deploy" } else { "upgrade" }
                    }));
                    facts.actions.push(action);
                }
                _ => {}
            }
        }
    }

    fn tags(&self, facts: &TxFactsV1) -> Vec<String> {
        if facts.actions.iter().any(|a| a.t == action_type::DEPLOY) {
            vec!["deploy".into()]
        } else {
            vec![]
        }
    }
}
