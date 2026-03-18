use atlas_types::raw::RawTx;
use atlas_types::facts::TxFactsV1;
use crate::module::ParserModule;

const COMPUTE_BUDGET_PROGRAM: &str = "ComputeBudget111111111111111111111111111111";

/// Extracts ComputeUnitLimit and SetComputeUnitPrice from ComputeBudget instructions.
pub struct ComputeBudgetModule;

impl ParserModule for ComputeBudgetModule {
    fn name(&self) -> &'static str { "compute_budget" }

    fn detect(&self, raw: &RawTx, facts: &mut TxFactsV1) {
        let keys = &raw.account_keys;

        let all_ixs = raw.instructions.iter()
            .chain(raw.inner_instructions.iter().flat_map(|ii| &ii.instructions));

        for ix in all_ixs {
            let prog = match keys.get(ix.program_id_index as usize) {
                Some(k) => &k.pubkey,
                None    => continue,
            };
            if prog != COMPUTE_BUDGET_PROGRAM { continue; }
            if ix.data.is_empty() { continue; }

            match ix.data[0] {
                // SetComputeUnitLimit: disc=2, data[1..5] = u32 le
                2 if ix.data.len() >= 5 => {
                    let limit = u32::from_le_bytes([ix.data[1], ix.data[2], ix.data[3], ix.data[4]]);
                    facts.compute_units.limit = Some(limit);
                }
                // SetComputeUnitPrice: disc=3, data[1..9] = u64 le (micro-lamports per CU)
                3 if ix.data.len() >= 9 => {
                    let price = u64::from_le_bytes([
                        ix.data[1], ix.data[2], ix.data[3], ix.data[4],
                        ix.data[5], ix.data[6], ix.data[7], ix.data[8],
                    ]);
                    facts.compute_units.price_micro_lamports = Some(price);
                }
                _ => {}
            }
        }
    }
}
