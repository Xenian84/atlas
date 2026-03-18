use atlas_types::raw::RawTx;
use atlas_types::facts::{TxFactsV1, Action};

/// Trait for a parser module. Each module is responsible for detecting
/// one class of protocol actions from a raw transaction.
/// Modules run in order; each appends to facts.actions.
pub trait ParserModule: Send + Sync {
    fn name(&self) -> &'static str;

    /// Detect actions in this tx and push them into facts.actions.
    fn detect(&self, raw: &RawTx, facts: &mut TxFactsV1);

    /// Generate tags based on already-parsed facts.
    /// Called after all detect() passes.
    fn tags(&self, facts: &TxFactsV1) -> Vec<String> {
        let _ = facts;
        vec![]
    }
}

// ── Action type constants ─────────────────────────────────────────────────────
pub mod action_type {
    pub const TRANSFER:  &str = "TRANSFER";
    pub const SWAP:      &str = "SWAP";
    pub const MINT:      &str = "MINT";
    pub const BURN:      &str = "BURN";
    pub const STAKE:     &str = "STAKE";
    pub const UNSTAKE:   &str = "UNSTAKE";
    pub const NFT_SALE:  &str = "NFT_SALE";
    pub const DEPLOY:    &str = "DEPLOY";
    pub const UNKNOWN:   &str = "UNKNOWN";
}

// ── Protocol family constants ─────────────────────────────────────────────────
pub mod protocol {
    pub const SYSTEM:      &str = "SYSTEM";
    pub const TOKEN:       &str = "TOKEN";
    pub const X1DEX:       &str = "X1DEX";
    pub const JUPITERLIKE: &str = "JUPITERLIKE";
    pub const NFTPROG:     &str = "NFTPROG";
    pub const STAKE:       &str = "STAKE";
    pub const BRIDGE:      &str = "BRIDGE";
    pub const OTC:         &str = "OTC";
    pub const UNKNOWN:     &str = "UNKNOWN";
}
