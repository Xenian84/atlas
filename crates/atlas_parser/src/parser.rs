use atlas_types::raw::RawTx;
use atlas_types::facts::{TxFactsV1, AccountRef, Commitment, TxStatus};
use crate::module::ParserModule;
use crate::modules::*;
use crate::config::ProgramsConfig;
use crate::deltas::compute_xnt_deltas;
use crate::tags::apply_tags;
use crate::spam::{SpamConfig, apply_spam_tags};

pub struct Parser {
    modules:    Vec<Box<dyn ParserModule>>,
    spam:       SpamConfig,
    commitment: Commitment,
}

impl Parser {
    pub fn new(cfg: ProgramsConfig, spam: SpamConfig, commitment: Commitment) -> Self {
        let modules: Vec<Box<dyn ParserModule>> = vec![
            // ComputeBudget runs first so other modules can rely on the values
            Box::new(ComputeBudgetModule),
            Box::new(SystemTransferModule  { cfg: cfg.clone() }),
            Box::new(TokenTransferModule   { cfg: cfg.clone() }),
            Box::new(SwapDetectModule      { cfg: cfg.clone() }),
            Box::new(MintBurnModule        { cfg: cfg.clone() }),
            Box::new(StakeOpsModule        { cfg: cfg.clone() }),
            Box::new(DeployDetectModule    { cfg: cfg.clone() }),
            Box::new(NftOpsModule          { cfg: cfg.clone() }),
        ];
        Self { modules, spam, commitment }
    }

    /// Parse a raw transaction into a canonical TxFactsV1.
    /// Enforces deterministic ordering via normalize() at the end.
    pub fn parse(&self, raw: &RawTx) -> TxFactsV1 {
        let mut facts = TxFactsV1::new(raw.sig.clone(), raw.slot, raw.pos);
        facts.block_time   = raw.block_time;
        facts.commitment   = self.commitment.clone();
        facts.status       = if raw.err.is_none() { TxStatus::Success } else { TxStatus::Failed };
        facts.err          = raw.err.as_deref().map(|e| serde_json::json!(e));
        facts.fee_lamports = raw.fee;
        facts.compute_units.consumed = raw.compute_units_consumed.map(|v| v as u32);
        // limit and price_micro_lamports are populated by ComputeBudgetModule below

        // Build accounts from account_keys (which now includes ALT-resolved addresses
        // already merged in grpc_conv.rs)
        facts.accounts = raw.account_keys.iter().map(|k| {
            let mut roles = vec![];
            if k.is_signer   { roles.push("signer"); }
            if k.is_writable { roles.push("writable"); } else { roles.push("readonly"); }
            AccountRef::new(&k.pubkey, roles)
        }).collect();

        if let Some(acc) = facts.accounts.first_mut() {
            acc.r.push("feePayer".to_string());
        }

        // Collect programs from all instructions
        for ix in &raw.instructions {
            if let Some(key) = raw.account_keys.get(ix.program_id_index as usize) {
                facts.programs.push(key.pubkey.clone());
            }
        }

        // Run all parser modules
        for module in &self.modules {
            module.detect(raw, &mut facts);
        }

        // XNT native deltas (runs after modules so compute_units are set)
        compute_xnt_deltas(raw, &mut facts);

        // Tags
        apply_tags(&mut facts);
        apply_spam_tags(&mut facts, &self.spam);

        facts.normalize();

        facts
    }
}
