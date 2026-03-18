//! AtlasEntryFrame — the wire format between the tachyon validator bridge
//! (atlas_bridge.rs) and the atlas-shredstream binary.
//!
//! IMPORTANT: This struct definition is intentionally duplicated in
//! `/root/tachyon/core/src/atlas_bridge.rs`. Both sides must stay in sync.
//!
//! Design rules that guarantee bincode compatibility:
//!   1. Only primitive types: u8, u16, u32, u64, i64, bool, Vec, fixed arrays
//!   2. No Solana SDK types — no Pubkey, Signature, Hash, etc.
//!   3. Field ORDER is significant for bincode — never reorder fields
//!   4. Add new fields ONLY at the end of each struct
//!   5. Bump WIRE_VERSION when making breaking changes
//!
//! Wire format on the Unix socket:
//!   [4 bytes LE u32: frame_len][frame_len bytes: bincode(AtlasEntryFrame)]

use serde::{Deserialize, Serialize};

/// Bump this constant when the frame layout changes.
/// The atlas_bridge.rs in tachyon embeds this in each frame for version checking.
pub const WIRE_VERSION: u8 = 1;

/// One entry from the tachyon banking stage, forwarded before confirmed commitment.
/// Arrives within ~15ms of the leader broadcasting the slot shreds.
#[derive(Debug, Serialize, Deserialize)]
pub struct AtlasEntryFrame {
    /// Wire format version — must equal WIRE_VERSION on both sides.
    pub version: u8,

    /// Slot number this entry belongs to.
    pub slot: u64,

    /// Entry index within the slot (0-based).
    pub entry_idx: u32,

    /// PoH hashes since the previous entry (latency indicator per entry).
    pub num_hashes: u64,

    /// Microsecond timestamp recorded inside the validator when this was forwarded.
    pub ts_us: i64,

    /// Transactions in this entry (votes filtered out by the bridge).
    pub txs: Vec<AtlasTxFrame>,
}

/// One non-vote transaction extracted from a tachyon Entry.
///
/// Byte array fields use Vec<u8> (not fixed-size arrays) because serde only
/// derives Serialize/Deserialize for arrays up to [u8; 32]. bincode encodes
/// Vec<u8> as (len: u64, bytes...) — the validator bridge writes the same.
#[derive(Debug, Serialize, Deserialize)]
pub struct AtlasTxFrame {
    /// First (fee-payer) Ed25519 signature — 64 raw bytes.
    pub sig: Vec<u8>,

    /// All account keys referenced by this transaction — flat bytes, 32 per key.
    /// Length is always a multiple of 32.
    pub account_keys: Vec<u8>,

    /// Number of accounts in account_keys (account_keys.len() / 32).
    pub num_accounts: u16,

    /// Indices into the logical account key array for top-level instruction programs.
    pub program_indices: Vec<u8>,

    /// Full bincode-serialized VersionedTransaction for complete instruction parsing.
    pub raw_tx: Vec<u8>,
}

impl AtlasTxFrame {
    /// First signature as a base58 string.
    pub fn sig_b58(&self) -> String {
        bs58::encode(&self.sig).into_string()
    }

    /// Extract program pubkeys as base58 strings using program_indices.
    pub fn programs_b58(&self) -> Vec<String> {
        self.program_indices
            .iter()
            .filter_map(|&idx| {
                let start = idx as usize * 32;
                self.account_keys.get(start..start + 32)
            })
            .map(|pk| bs58::encode(pk).into_string())
            .collect()
    }

    /// All account keys as base58 strings (32 bytes each).
    pub fn accounts_b58(&self) -> Vec<String> {
        self.account_keys
            .chunks_exact(32)
            .map(|pk| bs58::encode(pk).into_string())
            .collect()
    }
}
