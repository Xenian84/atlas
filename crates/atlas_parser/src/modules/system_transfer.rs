use atlas_types::raw::RawTx;
use atlas_types::facts::{TxFactsV1, Action};
use crate::module::{ParserModule, action_type, protocol};
use crate::config::ProgramsConfig;

/// Detect native XNT transfers via System Program instruction 2 (Transfer).
pub struct SystemTransferModule {
    pub cfg: ProgramsConfig,
}

impl ParserModule for SystemTransferModule {
    fn name(&self) -> &'static str { "system_transfer" }

    fn detect(&self, raw: &RawTx, facts: &mut TxFactsV1) {
        let keys = &raw.account_keys;
        for ix in &raw.instructions {
            let prog_idx = ix.program_id_index as usize;
            let Some(prog) = keys.get(prog_idx) else { continue };
            if !self.cfg.is_system_program(&prog.pubkey) { continue }
            // System program Transfer instruction has discriminant [2,0,0,0] (little-endian u32=2)
            if ix.data.len() < 12 { continue }
            let discriminant = u32::from_le_bytes([ix.data[0], ix.data[1], ix.data[2], ix.data[3]]);
            if discriminant != 2 { continue }
            let lamports = u64::from_le_bytes(
                ix.data[4..12].try_into().unwrap_or([0u8; 8])
            );
            let from = ix.accounts.first()
                .and_then(|i| keys.get(*i as usize))
                .map(|k| k.pubkey.clone())
                .unwrap_or_default();
            let to = ix.accounts.get(1)
                .and_then(|i| keys.get(*i as usize))
                .map(|k| k.pubkey.clone())
                .unwrap_or_default();

            let mut action = Action::new(action_type::TRANSFER, protocol::SYSTEM, &from);
            action.x   = Some(to);
            action.amt = Some(serde_json::json!({ "lamports": lamports }));
            facts.actions.push(action);
        }
    }
}
