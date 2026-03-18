use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── TxFactsV1 — Canonical Facts Object (CFO) ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxFactsV1 {
    pub v:            String,   // "txfacts.v1"
    pub chain:        String,   // "x1"
    pub sig:          String,
    pub slot:         u64,
    pub pos:          u32,
    pub block_time:   Option<i64>,
    pub commitment:   Commitment,
    pub status:       TxStatus,
    pub err:          Option<Value>,
    pub fee_lamports: u64,
    pub compute_units: ComputeUnits,
    /// Sorted by pubkey asc after normalize()
    pub accounts:     Vec<AccountRef>,
    /// Sorted asc after normalize()
    pub programs:     Vec<String>,
    /// Sorted by (t, p, s) after normalize()
    pub actions:      Vec<Action>,
    /// Sorted by (mint, owner) after normalize()
    pub token_deltas: Vec<TokenDelta>,
    #[serde(rename = "xnt_deltas")]
    pub sol_deltas:   Vec<NativeDelta>,
    pub logs_digest:  Option<Value>,
    /// Sorted asc after normalize()
    pub tags:         Vec<String>,
    pub raw_ref:      Option<String>,
}

impl TxFactsV1 {
    pub fn new(sig: String, slot: u64, pos: u32) -> Self {
        Self {
            v:            "txfacts.v1".to_string(),
            chain:        "x1".to_string(),
            sig,
            slot,
            pos,
            block_time:   None,
            commitment:   Commitment::Confirmed,
            status:       TxStatus::Success,
            err:          None,
            fee_lamports: 0,
            compute_units: ComputeUnits::default(),
            accounts:     vec![],
            programs:     vec![],
            actions:      vec![],
            token_deltas: vec![],
            sol_deltas:   vec![],
            logs_digest:  None,
            tags:         vec![],
            raw_ref:      None,
        }
    }

    /// Enforce deterministic ordering on all collections.
    /// MUST be called before any serialization or storage.
    pub fn normalize(&mut self) {
        self.accounts.sort_by(|a, b| a.a.cmp(&b.a));
        self.programs.sort();
        self.programs.dedup();
        self.actions.sort_by(|a, b| {
            a.t.cmp(&b.t).then(a.p.cmp(&b.p)).then(a.s.cmp(&b.s))
        });
        self.token_deltas.sort_by(|a, b| {
            a.mint.cmp(&b.mint).then(a.owner.cmp(&b.owner))
        });
        self.tags.sort();
        self.tags.dedup();
    }

    pub fn cursor_str(&self) -> String {
        format!("{}:{}", self.slot, self.pos)
    }

    pub fn is_success(&self) -> bool {
        self.status == TxStatus::Success
    }

    /// Collect all unique addresses referenced by this tx.
    pub fn all_addresses(&self) -> Vec<String> {
        self.accounts.iter().map(|a| a.a.clone()).collect()
    }

    /// Collect all unique action types.
    pub fn action_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.actions.iter().map(|a| a.t.clone()).collect();
        types.sort();
        types.dedup();
        types
    }
}

// ── Sub-types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Commitment {
    Processed,
    #[default]
    Confirmed,
    Finalized,
}

impl Commitment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Commitment::Processed  => "processed",
            Commitment::Confirmed  => "confirmed",
            Commitment::Finalized  => "finalized",
        }
    }

    /// Numeric rank — higher = more confirmed
    pub fn rank(&self) -> u8 {
        match self {
            Commitment::Processed  => 0,
            Commitment::Confirmed  => 1,
            Commitment::Finalized  => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TxStatus {
    Success,
    Failed,
}

impl TxStatus {
    pub fn as_smallint(&self) -> i16 {
        match self {
            TxStatus::Success => 1,
            TxStatus::Failed  => 2,
        }
    }

    pub fn from_smallint(v: i16) -> Self {
        if v == 1 { TxStatus::Success } else { TxStatus::Failed }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputeUnits {
    pub consumed:             Option<u32>,
    pub limit:                Option<u32>,
    pub price_micro_lamports: Option<u64>,
}

/// One account referenced by the transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRef {
    /// Pubkey (base58)
    pub a: String,
    /// Roles: signer, feePayer, writable, readonly
    pub r: Vec<String>,
    /// Owner program pubkey if known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub o: Option<String>,
    /// Human label (future)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,
}

impl AccountRef {
    pub fn new(pubkey: impl Into<String>, roles: Vec<&str>) -> Self {
        Self {
            a: pubkey.into(),
            r: roles.into_iter().map(String::from).collect(),
            o: None,
            n: None,
        }
    }

    pub fn is_signer(&self) -> bool   { self.r.iter().any(|r| r == "signer") }
    pub fn is_writable(&self) -> bool { self.r.iter().any(|r| r == "writable") }
    pub fn is_fee_payer(&self) -> bool { self.r.iter().any(|r| r == "feePayer") }
}

/// A parsed high-level action within a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Type: TRANSFER | SWAP | MINT | BURN | STAKE | UNSTAKE | NFT_SALE | DEPLOY | UNKNOWN
    pub t: String,
    /// Protocol family: SYSTEM | TOKEN | X1DEX | JUPITERLIKE | NFTPROG | STAKE | BRIDGE | OTC
    pub p: String,
    /// Subject (initiator pubkey)
    pub s: String,
    /// Counterparty / pool / destination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    /// Amount — small numeric value (lamports, token units, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amt: Option<Value>,
    /// Action-specific metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

impl Action {
    pub fn new(t: impl Into<String>, p: impl Into<String>, s: impl Into<String>) -> Self {
        Self {
            t: t.into(), p: p.into(), s: s.into(),
            x: None, amt: None, meta: None,
        }
    }
}

/// Token balance change for one (owner, mint) pair within a tx.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDelta {
    pub mint:      String,
    pub owner:     String,
    pub account:   String,
    /// Pre-balance as string (lossless for large u64)
    pub pre:       String,
    /// Post-balance as string
    pub post:      String,
    /// Signed delta as string
    pub delta:     String,
    pub decimals:  u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol:    Option<String>,
    pub direction: DeltaDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeltaDirection {
    In,
    Out,
    None,
}

impl DeltaDirection {
    pub fn as_smallint(&self) -> i16 {
        match self {
            DeltaDirection::In   => 1,
            DeltaDirection::Out  => 2,
            DeltaDirection::None => 0,
        }
    }
}

/// Native XNT balance change for one owner within a tx.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeDelta {
    pub owner:          String,
    pub pre_lamports:   u64,
    pub post_lamports:  u64,
    pub delta_lamports: i64,
}

// ── Compact tx summary (used in history list responses) ───────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxSummary {
    pub signature:    String,
    pub slot:         u64,
    pub pos:          u32,
    pub block_time:   Option<i64>,
    pub status:       TxStatus,
    pub fee_lamports: u64,
    pub tags:         Vec<String>,
    pub action_types: Vec<String>,
    pub actions:      Vec<Action>,
    pub token_deltas: Vec<TokenDelta>,
}

impl From<&TxFactsV1> for TxSummary {
    fn from(f: &TxFactsV1) -> Self {
        Self {
            signature:    f.sig.clone(),
            slot:         f.slot,
            pos:          f.pos,
            block_time:   f.block_time,
            status:       f.status.clone(),
            fee_lamports: f.fee_lamports,
            tags:         f.tags.clone(),
            action_types: f.action_types(),
            actions:      f.actions.clone(),
            token_deltas: f.token_deltas.clone(),
        }
    }
}

// ── Address history page ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxHistoryPage {
    pub address:     String,
    pub limit:       usize,
    pub next_cursor: Option<String>,
    pub transactions: Vec<TxSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_facts() -> TxFactsV1 {
        let mut f = TxFactsV1::new("sig1".to_string(), 100, 0);
        f.tags     = vec!["swap".to_string(), "fee_only".to_string(), "transfer".to_string()];
        f.programs = vec!["BBB".to_string(), "AAA".to_string(), "CCC".to_string()];
        f
    }

    #[test]
    fn normalize_sorts_tags_and_programs() {
        let mut f = sample_facts();
        f.normalize();
        assert_eq!(f.tags,     vec!["fee_only", "swap", "transfer"]);
        assert_eq!(f.programs, vec!["AAA", "BBB", "CCC"]);
    }

    #[test]
    fn normalize_is_idempotent() {
        let mut f = sample_facts();
        f.normalize();
        let first = f.clone();
        f.normalize();
        assert_eq!(first.tags,     f.tags);
        assert_eq!(first.programs, f.programs);
    }

    #[test]
    fn normalize_deduplicates_programs() {
        let mut f = TxFactsV1::new("sig2".to_string(), 1, 0);
        f.programs = vec!["X".to_string(), "X".to_string(), "Y".to_string()];
        f.normalize();
        assert_eq!(f.programs, vec!["X", "Y"]);
    }

    #[test]
    fn is_success_reflects_status() {
        let mut f = TxFactsV1::new("sig3".to_string(), 1, 0);
        f.status = TxStatus::Success;
        assert!(f.is_success());
        f.status = TxStatus::Failed;
        assert!(!f.is_success());
    }
}
