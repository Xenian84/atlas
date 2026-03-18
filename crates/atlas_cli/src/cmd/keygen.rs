//! atlas keygen — generate an X1 / Solana-compatible ed25519 keypair.
//!
//! Writes a JSON file compatible with `solana-keygen` format:
//!   [secret_bytes...32, public_bytes...32]  (64-byte array)

use anyhow::{Context, Result};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::path::PathBuf;

pub async fn run(output: Option<String>, json: bool) -> Result<()> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let secret_bytes = signing_key.to_bytes();
    let public_bytes = verifying_key.to_bytes();

    // Solana keypair format: 64-byte array [secret(32) || public(32)]
    let mut keypair_bytes = Vec::with_capacity(64);
    keypair_bytes.extend_from_slice(&secret_bytes);
    keypair_bytes.extend_from_slice(&public_bytes);

    let address = bs58::encode(&public_bytes).into_string();

    // Determine output path
    let path: PathBuf = match output {
        Some(p) => PathBuf::from(p),
        None => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".atlas").join("keypair.json")
        }
    };

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }

    // Write as JSON array (solana-keygen compatible)
    let json_content = serde_json::to_string(&keypair_bytes)?;
    std::fs::write(&path, json_content)
        .with_context(|| format!("write keypair to {}", path.display()))?;

    if json {
        println!("{}", serde_json::json!({
            "address":      address,
            "keypair_path": path.display().to_string(),
            "network":      "X1 Mainnet",
            "note":         "Fund this address with XNT before submitting transactions",
        }));
    } else {
        println!("✓  Keypair generated");
        println!("   Path:    {}", path.display());
        println!("   Address: {address}");
        println!();
        println!("   To use this wallet on X1, fund it with:");
        println!("   • XNT for transaction fees (~0.001 XNT)");
        println!();
        println!("   ⚠  Keep your keypair file secret — it controls this address.");
    }

    Ok(())
}
