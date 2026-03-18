/// Raw transaction representation extracted from Yellowstone gRPC before parsing.
/// This is an intermediate struct; parsers convert RawTx → TxFactsV1.

#[derive(Debug, Clone)]
pub struct RawTx {
    pub sig:        String,
    pub slot:       u64,
    pub pos:        u32,
    pub block_time: Option<i64>,
    pub is_vote:    bool,
    pub err:        Option<String>,

    pub fee:            u64,
    pub compute_units_consumed: Option<u64>,
    pub compute_units_limit:    Option<u32>,
    pub priority_fee_micro_lamports: Option<u64>,

    pub account_keys:    Vec<RawAccountKey>,
    pub instructions:    Vec<RawInstruction>,
    pub inner_instructions: Vec<RawInnerInstruction>,
    pub pre_balances:    Vec<u64>,
    pub post_balances:   Vec<u64>,
    pub pre_token_balances:  Vec<RawTokenBalance>,
    pub post_token_balances: Vec<RawTokenBalance>,
    pub log_messages:    Vec<String>,
    pub loaded_addresses: RawLoadedAddresses,
}

#[derive(Debug, Clone)]
pub struct RawAccountKey {
    pub pubkey:     String,
    pub is_signer:  bool,
    pub is_writable: bool,
}

#[derive(Debug, Clone)]
pub struct RawInstruction {
    pub program_id_index: u8,
    pub accounts:         Vec<u8>,
    pub data:             Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RawInnerInstruction {
    pub index:        u8,
    pub instructions: Vec<RawInstruction>,
}

#[derive(Debug, Clone)]
pub struct RawTokenBalance {
    pub account_index: u8,
    pub mint:          String,
    pub owner:         String,
    pub ui_amount:     Option<f64>,
    pub amount:        String,
    pub decimals:      u8,
}

#[derive(Debug, Clone, Default)]
pub struct RawLoadedAddresses {
    pub writable: Vec<String>,
    pub readonly: Vec<String>,
}
