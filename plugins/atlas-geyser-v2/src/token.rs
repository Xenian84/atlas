/// Inline SPL Token / Token-2022 account parser.
///
/// Extracted from x1-geyser-postgres (which copied it from solana-runtime).
/// We only need mint + owner — the two fields Atlas writes to token_owner_map.
use solana_sdk::pubkey::{Pubkey, PUBKEY_BYTES};

// ── Program IDs ─────────────────────────────────────────────────────────────

/// SPL Token v1 program
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// SPL Token-2022 program
pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

// ── Account layout offsets ───────────────────────────────────────────────────

const MINT_OFFSET: usize  = 0;
const OWNER_OFFSET: usize = 32;
const TOKEN_ACCOUNT_LEN: usize = 165; // minimum size of a valid SPL token account

// ── Parser ───────────────────────────────────────────────────────────────────

/// Try to extract (mint, owner) from raw account data.
///
/// Returns `Some((mint_b58, owner_b58))` if the data looks like an SPL token
/// account (either v1 or Token-2022), `None` otherwise.
pub fn parse_token_account(data: &[u8]) -> Option<(String, String)> {
    if data.len() < TOKEN_ACCOUNT_LEN {
        return None;
    }

    let mint  = read_pubkey(data, MINT_OFFSET);
    let owner = read_pubkey(data, OWNER_OFFSET);

    // A zero pubkey is never a valid mint or owner.
    if mint.to_bytes() == [0u8; 32] || owner.to_bytes() == [0u8; 32] {
        return None;
    }

    Some((mint.to_string(), owner.to_string()))
}

#[inline]
fn read_pubkey(data: &[u8], offset: usize) -> Pubkey {
    let bytes: &[u8; PUBKEY_BYTES] = data[offset..offset + PUBKEY_BYTES]
        .try_into()
        .expect("slice length checked above");
    Pubkey::from(*bytes)
}

/// Returns true if `owner` is the SPL Token v1 or Token-2022 program ID.
#[inline]
pub fn is_token_program(owner: &[u8]) -> bool {
    let s = std::str::from_utf8(owner).unwrap_or("");
    s == TOKEN_PROGRAM_ID || s == TOKEN_2022_PROGRAM_ID
}
