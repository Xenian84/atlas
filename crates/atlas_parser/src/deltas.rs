use atlas_types::raw::RawTx;
use atlas_types::facts::{TxFactsV1, NativeDelta};

/// Compute native XNT balance deltas from pre/post balances.
/// Called once per tx after all module detect() passes.
pub fn compute_xnt_deltas(raw: &RawTx, facts: &mut TxFactsV1) {
    let keys = &raw.account_keys;
    for (i, (pre, post)) in raw.pre_balances.iter().zip(raw.post_balances.iter()).enumerate() {
        if pre == post { continue }
        let Some(key) = keys.get(i) else { continue };
        let delta = *post as i64 - *pre as i64;
        facts.sol_deltas.push(NativeDelta {
            owner:         key.pubkey.clone(),
            pre_lamports:  *pre,
            post_lamports: *post,
            delta_lamports: delta,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_types::raw::{RawTx, RawAccountKey, RawLoadedAddresses};
    use atlas_types::facts::TxFactsV1;

    fn make_raw(pre: Vec<u64>, post: Vec<u64>, keys: Vec<&str>) -> RawTx {
        RawTx {
            sig:          "testsig".to_string(),
            slot:         1,
            pos:          0,
            block_time:   None,
            is_vote:      false,
            err:          None,
            fee:          5000,
            compute_units_consumed:        None,
            compute_units_limit:           None,
            priority_fee_micro_lamports:   None,
            account_keys: keys.iter().map(|k| RawAccountKey {
                pubkey: k.to_string(), is_signer: false, is_writable: true,
            }).collect(),
            instructions:       vec![],
            inner_instructions: vec![],
            pre_balances:  pre,
            post_balances: post,
            pre_token_balances:  vec![],
            post_token_balances: vec![],
            log_messages:        vec![],
            loaded_addresses: RawLoadedAddresses::default(),
        }
    }

    #[test]
    fn xnt_delta_computed_for_changed_accounts() {
        let raw = make_raw(
            vec![1_000_000_000, 500_000_000],
            vec![  995_000_000, 505_000_000],
            vec!["alice", "bob"],
        );
        let mut facts = TxFactsV1::new("s".to_string(), 1, 0);
        compute_xnt_deltas(&raw, &mut facts);
        assert_eq!(facts.sol_deltas.len(), 2);
        let alice = facts.sol_deltas.iter().find(|d| d.owner == "alice").unwrap();
        assert_eq!(alice.delta_lamports, -5_000_000);
        let bob = facts.sol_deltas.iter().find(|d| d.owner == "bob").unwrap();
        assert_eq!(bob.delta_lamports, 5_000_000);
    }

    #[test]
    fn unchanged_balances_produce_no_delta() {
        let raw = make_raw(vec![100, 200], vec![100, 200], vec!["a", "b"]);
        let mut facts = TxFactsV1::new("s".to_string(), 1, 0);
        compute_xnt_deltas(&raw, &mut facts);
        assert!(facts.sol_deltas.is_empty());
    }
}
