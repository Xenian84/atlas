use atlas_types::raw::RawTx;
use atlas_types::facts::{TxFactsV1, Action};
use crate::module::{ParserModule, action_type, protocol};
use crate::config::ProgramsConfig;

/// Detect NFT operations — mints and sales on X1 NFT marketplaces.
/// TODO: update program IDs once X1 NFT standards are finalized.
pub struct NftOpsModule {
    pub cfg: ProgramsConfig,
}

impl ParserModule for NftOpsModule {
    fn name(&self) -> &'static str { "nft_ops" }

    fn detect(&self, raw: &RawTx, facts: &mut TxFactsV1) {
        let keys = &raw.account_keys;

        let nft_marketplace = self.cfg.x1.get("nft_marketplace")
            .filter(|id| !id.starts_with("TODO"))
            .cloned();

        let Some(marketplace_id) = nft_marketplace else { return };

        let has_marketplace = keys.iter().any(|k| k.pubkey == marketplace_id);
        if !has_marketplace { return }

        let initiator = raw.account_keys.first()
            .map(|k| k.pubkey.clone())
            .unwrap_or_default();

        // Heuristic: if tx has XNT transfer + token transfer -> NFT sale
        let has_sol_movement = !raw.pre_balances.is_empty()
            && raw.pre_balances.iter().zip(raw.post_balances.iter())
                .any(|(pre, post)| pre != post);

        let has_token_movement = !raw.pre_token_balances.is_empty();

        if has_sol_movement && has_token_movement {
            let mut action = Action::new(action_type::NFT_SALE, protocol::NFTPROG, &initiator);
            action.x    = Some(marketplace_id);
            action.meta = Some(serde_json::json!({ "op": "sale" }));
            facts.actions.push(action);
        }
    }
}
