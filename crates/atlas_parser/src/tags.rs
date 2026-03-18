use atlas_types::facts::TxFactsV1;

/// Apply deterministic tag rules based on facts state.
/// Called after all module detect() and deltas() passes.
pub fn apply_tags(facts: &mut TxFactsV1) {
    let mut tags = std::mem::take(&mut facts.tags);

    // Status
    if !facts.is_success() {
        tags.push("failed".into());
    }

    // Action type tags
    for action in &facts.actions {
        match action.t.as_str() {
            "TRANSFER"  => add_unique(&mut tags, "transfer"),
            "SWAP"      => add_unique(&mut tags, "swap"),
            "MINT"      => add_unique(&mut tags, "mint"),
            "BURN"      => add_unique(&mut tags, "burn"),
            "STAKE"     => add_unique(&mut tags, "stake"),
            "UNSTAKE"   => add_unique(&mut tags, "unstake"),
            "DEPLOY"    => add_unique(&mut tags, "deploy"),
            "NFT_SALE"  => add_unique(&mut tags, "nft"),
            _           => {}
        }
    }

    // No parsed actions
    if facts.actions.is_empty() {
        add_unique(&mut tags, "fee_only");
    }

    // Compute budget
    if let Some(consumed) = facts.compute_units.consumed {
        if consumed >= 800_000 {
            add_unique(&mut tags, "high_compute");
        }
    }
    if facts.compute_units.price_micro_lamports.unwrap_or(0) > 0 {
        add_unique(&mut tags, "priority_fee");
    }

    // Vote tx detection: if only Vote program used
    if facts.programs.len() == 1
        && facts.programs[0] == "Vote111111111111111111111111111111111111111111"
    {
        add_unique(&mut tags, "vote");
    }

    tags.sort();
    tags.dedup();
    facts.tags = tags;
}

fn add_unique(tags: &mut Vec<String>, tag: &str) {
    if !tags.iter().any(|t| t == tag) {
        tags.push(tag.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_types::facts::{TxFactsV1, TxStatus, Action};
    use crate::module::action_type;

    fn tx_with_action(t: &str) -> TxFactsV1 {
        let mut f = TxFactsV1::new("s".to_string(), 1, 0);
        f.status = TxStatus::Success;
        f.actions.push(Action::new(t, "proto", "feePayer"));
        f
    }

    #[test]
    fn success_tx_gets_no_failed_tag() {
        let mut f = TxFactsV1::new("s".to_string(), 1, 0);
        f.status = TxStatus::Success;
        apply_tags(&mut f);
        assert!(!f.tags.contains(&"failed".to_string()));
    }

    #[test]
    fn failed_tx_gets_failed_tag() {
        let mut f = TxFactsV1::new("s".to_string(), 1, 0);
        f.status = TxStatus::Failed;
        apply_tags(&mut f);
        assert!(f.tags.contains(&"failed".to_string()));
    }

    #[test]
    fn swap_action_gets_swap_tag() {
        let mut f = tx_with_action(action_type::SWAP);
        apply_tags(&mut f);
        assert!(f.tags.contains(&"swap".to_string()));
    }

    #[test]
    fn transfer_action_gets_transfer_tag() {
        let mut f = tx_with_action(action_type::TRANSFER);
        apply_tags(&mut f);
        assert!(f.tags.contains(&"transfer".to_string()));
    }

    #[test]
    fn no_actions_gets_fee_only_tag() {
        let mut f = TxFactsV1::new("s".to_string(), 1, 0);
        f.status = TxStatus::Success;
        apply_tags(&mut f);
        assert!(f.tags.contains(&"fee_only".to_string()));
    }

    #[test]
    fn tags_are_sorted_after_apply() {
        let mut f = TxFactsV1::new("s".to_string(), 1, 0);
        f.status = TxStatus::Failed;
        f.actions.push(Action::new(action_type::SWAP, "proto", "addr"));
        apply_tags(&mut f);
        let sorted = { let mut t = f.tags.clone(); t.sort(); t };
        assert_eq!(f.tags, sorted);
    }
}
