use atlas_types::raw::RawTx;
use atlas_types::facts::{TxFactsV1, Action};
use crate::module::{ParserModule, action_type, protocol};
use crate::config::ProgramsConfig;

/// Detect swaps on known DEX programs.
/// Strategy: if a known DEX program appears in instructions AND there are >= 2
/// token deltas, classify as SWAP.
pub struct SwapDetectModule {
    pub cfg: ProgramsConfig,
}

impl ParserModule for SwapDetectModule {
    fn name(&self) -> &'static str { "swap_detect" }

    fn detect(&self, raw: &RawTx, facts: &mut TxFactsV1) {
        let keys = &raw.account_keys;
        let dex_program = keys.iter().find(|k| self.cfg.is_dex(&k.pubkey));
        let Some(dex) = dex_program else { return };

        // Require at least 2 distinct token mints in the tx (in/out)
        let mints: std::collections::HashSet<&str> = raw.pre_token_balances.iter()
            .chain(raw.post_token_balances.iter())
            .map(|b| b.mint.as_str())
            .collect();

        if mints.len() < 2 { return }

        let initiator = raw.account_keys.first()
            .map(|k| k.pubkey.clone())
            .unwrap_or_default();

        let proto = classify_dex_protocol(&dex.pubkey);

        let mut action = Action::new(action_type::SWAP, proto, &initiator);
        action.x = Some(dex.pubkey.clone());
        action.meta = Some(serde_json::json!({
            "dex_program": dex.pubkey,
            "mint_count": mints.len(),
        }));
        facts.actions.push(action);
    }

    fn tags(&self, facts: &TxFactsV1) -> Vec<String> {
        if facts.actions.iter().any(|a| a.t == action_type::SWAP) {
            vec!["swap".into()]
        } else {
            vec![]
        }
    }
}

fn classify_dex_protocol(program_id: &str) -> &'static str {
    // Jupiter v6
    if program_id == "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" {
        return protocol::JUPITERLIKE;
    }
    protocol::X1DEX
}
