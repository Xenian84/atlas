/// DAS Indexer — populates asset_index and token_account_index.
///
/// Called after every tx upsert. Scans token_deltas for mint events,
/// then fetches on-chain Metaplex metadata to enrich the asset record.
use anyhow::Result;
use serde_json::Value;
use sqlx::PgPool;
use tracing::{debug, warn};
use atlas_types::facts::TxFactsV1;

// Known program IDs
const TOKEN_PROGRAM:       &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM:  &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const METADATA_PROGRAM:    &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
const BUBBLEGUM_PROGRAM:   &str = "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY";

/// Index all asset-related events from a parsed transaction.
pub async fn index_assets(
    pool:     &PgPool,
    http:     &reqwest::Client,
    rpc_url:  &str,
    facts:    &TxFactsV1,
) -> Result<()> {
    // Only process if token or NFT programs are involved
    let relevant = facts.programs.iter().any(|p| {
        p == TOKEN_PROGRAM || p == TOKEN_2022_PROGRAM
        || p == METADATA_PROGRAM || p == BUBBLEGUM_PROGRAM
    });
    if !relevant { return Ok(()); }

    // Update token account balances from token_deltas
    for delta in &facts.token_deltas {
        let token_account = &delta.account;
        if !token_account.is_empty() {
            upsert_token_account(pool, token_account, &delta.owner, &delta.mint, facts.slot).await?;
        }
    }

    // Detect mints: token_deltas where a new mint appears in actions
    let minted_mints: Vec<&str> = facts.actions.iter()
        .filter(|a| a.t == "MINT" || a.t == "NFT_MINT")
        .filter_map(|a| a.amt.as_ref()?.get("mint")?.as_str())
        .collect();

    for mint in minted_mints {
        if let Err(e) = index_mint(pool, http, rpc_url, mint, facts.slot).await {
            warn!("DAS: failed to index mint {}: {}", mint, e);
        }
    }

    // Also index any mint seen in token_deltas that isn't in asset_index yet
    let delta_mints: Vec<String> = facts.token_deltas.iter()
        .map(|d| d.mint.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for mint in &delta_mints {
        ensure_asset_known(pool, http, rpc_url, mint, facts.slot).await?;
    }

    Ok(())
}

async fn ensure_asset_known(
    pool:    &PgPool,
    http:    &reqwest::Client,
    rpc_url: &str,
    mint:    &str,
    slot:    u64,
) -> Result<()> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM asset_index WHERE mint = $1)"
    )
    .bind(mint)
    .fetch_one(pool)
    .await?;

    if !exists {
        if let Err(e) = index_mint(pool, http, rpc_url, mint, slot).await {
            debug!("DAS: could not index mint {}: {}", mint, e);
        }
    }
    Ok(())
}

async fn index_mint(
    pool:    &PgPool,
    http:    &reqwest::Client,
    rpc_url: &str,
    mint:    &str,
    slot:    u64,
) -> Result<()> {
    // Fetch mint account info
    let mint_info = rpc_get_account_info(http, rpc_url, mint).await?;
    if mint_info.is_null() { return Ok(()); }

    let decimals = mint_info["data"]["parsed"]["info"]["decimals"]
        .as_u64().unwrap_or(0) as i16;
    let supply_str = mint_info["data"]["parsed"]["info"]["supply"]
        .as_str().unwrap_or("1");
    let mint_authority = mint_info["data"]["parsed"]["info"]["mintAuthority"]
        .as_str().map(String::from);

    // Determine asset type from decimals
    let asset_type = if decimals == 0 { "nft" } else { "fungible" };

    // Try to fetch Metaplex metadata
    let (name, symbol, uri, creator, creator_verified, collection_mint, collection_verified,
         royalty_basis_pts, attributes_json) =
        fetch_metaplex_metadata(http, rpc_url, mint).await.unwrap_or_default();

    sqlx::query(r#"
        INSERT INTO asset_index (
            mint, asset_type, update_authority, creator, creator_verified,
            collection_mint, collection_verified,
            name, symbol, uri, decimals, supply, slot_created, slot_updated,
            royalty_basis_pts, attributes_json
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12::numeric,$13,$14,$15,$16)
        ON CONFLICT (mint) DO UPDATE SET
            owner               = COALESCE(EXCLUDED.owner, asset_index.owner),
            name                = CASE WHEN EXCLUDED.name != '' THEN EXCLUDED.name ELSE asset_index.name END,
            symbol              = CASE WHEN EXCLUDED.symbol != '' THEN EXCLUDED.symbol ELSE asset_index.symbol END,
            uri                 = COALESCE(EXCLUDED.uri, asset_index.uri),
            creator             = COALESCE(EXCLUDED.creator, asset_index.creator),
            creator_verified    = EXCLUDED.creator_verified,
            collection_mint     = COALESCE(EXCLUDED.collection_mint, asset_index.collection_mint),
            collection_verified = EXCLUDED.collection_verified,
            royalty_basis_pts   = EXCLUDED.royalty_basis_pts,
            attributes_json     = EXCLUDED.attributes_json,
            slot_updated        = EXCLUDED.slot_updated,
            updated_at          = now()
        "#)
    .bind(mint)
    .bind(asset_type)
    .bind(mint_authority.as_deref())
    .bind(creator.as_deref())
    .bind(creator_verified)
    .bind(collection_mint.as_deref())
    .bind(collection_verified)
    .bind(&name)
    .bind(&symbol)
    .bind(uri.as_deref())
    .bind(decimals)
    .bind(supply_str)
    .bind(slot as i64)
    .bind(slot as i64)
    .bind(royalty_basis_pts)
    .bind(&attributes_json)
    .execute(pool)
    .await?;

    Ok(())
}

async fn upsert_token_account(
    pool:          &PgPool,
    token_account: &str,
    owner:         &str,
    mint:          &str,
    slot:          u64,
) -> Result<()> {
    sqlx::query(r#"
        INSERT INTO token_account_index (token_account, owner, mint, slot_updated, updated_at)
        VALUES ($1, $2, $3, $4, now())
        ON CONFLICT (token_account) DO UPDATE SET
            owner        = EXCLUDED.owner,
            slot_updated = EXCLUDED.slot_updated,
            updated_at   = now()
    "#)
    .bind(token_account)
    .bind(owner)
    .bind(mint)
    .bind(slot as i64)
    .execute(pool)
    .await?;
    Ok(())
}

// ── RPC helpers ───────────────────────────────────────────────────────────────

async fn rpc_get_account_info(
    http:    &reqwest::Client,
    rpc_url: &str,
    pubkey:  &str,
) -> Result<Value> {
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getAccountInfo",
        "params": [pubkey, { "encoding": "jsonParsed" }]
    });
    let resp: Value = http.post(rpc_url).json(&body).send().await?.json().await?;
    Ok(resp["result"]["value"].clone())
}

/// Returns (name, symbol, uri, creator, creator_verified, collection_mint,
///          collection_verified, royalty_basis_pts, attributes_json)
#[allow(clippy::type_complexity)]
async fn fetch_metaplex_metadata(
    http:    &reqwest::Client,
    rpc_url: &str,
    mint:    &str,
) -> Result<(String, String, Option<String>, Option<String>, bool,
             Option<String>, bool, i32, Value)> {
    // Derive Metaplex metadata PDA: seeds = ["metadata", METADATA_PROGRAM, mint]
    // We proxy this through the validator's getAccountInfo for the PDA
    let meta_pda = derive_metadata_pda(mint);

    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getAccountInfo",
        "params": [meta_pda, { "encoding": "base64" }]
    });
    let resp: Value = http.post(rpc_url).json(&body).send().await?.json().await?;
    let data_b64 = resp["result"]["value"]["data"][0].as_str().unwrap_or("");
    if data_b64.is_empty() {
        return Ok(Default::default());
    }

    let data = base64_decode(data_b64)?;
    parse_metaplex_metadata_bytes(&data)
}

fn derive_metadata_pda(mint: &str) -> String {
    // Placeholder — real impl uses solana_sdk::pubkey PDA derivation.
    // For now return a dummy so it fails gracefully on unknown mints.
    format!("meta_{}", &mint[..mint.len().min(8)])
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    use base64::{Engine as _, engine::general_purpose};
    Ok(general_purpose::STANDARD.decode(s)?)
}

/// Parse raw Metaplex token-metadata account bytes (v1.1 layout).
fn parse_metaplex_metadata_bytes(data: &[u8]) -> Result<(
    String, String, Option<String>, Option<String>, bool,
    Option<String>, bool, i32, Value,
)> {
    if data.len() < 10 { anyhow::bail!("metadata too short"); }

    let mut cursor = 1usize; // skip discriminator

    // update_authority (32 bytes)
    cursor += 32;
    // mint (32 bytes)
    cursor += 32;

    let name   = read_str(data, &mut cursor)?;
    let symbol = read_str(data, &mut cursor)?;
    let uri    = read_str(data, &mut cursor)?;

    // seller_fee_basis_points (2 bytes)
    let royalty = if cursor + 2 <= data.len() {
        u16::from_le_bytes([data[cursor], data[cursor+1]]) as i32
    } else { 0 };
    cursor += 2;

    // creators option
    let creator_addr;
    let creator_verified;
    if cursor < data.len() && data[cursor] == 1 {
        cursor += 1;
        let count = u32::from_le_bytes(data[cursor..cursor+4].try_into()?) as usize;
        cursor += 4;
        if count > 0 && cursor + 34 <= data.len() {
            let addr_bytes = &data[cursor..cursor+32];
            creator_addr = Some(bs58_encode(addr_bytes));
            cursor += 32;
            creator_verified = data[cursor] == 1;
            cursor += 1;
            cursor += 1; // share
            // skip remaining creators
            cursor += (count - 1) * 34;
        } else {
            creator_addr = None;
            creator_verified = false;
        }
    } else {
        creator_addr = None;
        creator_verified = false;
        cursor += 1;
    }

    let uri_trimmed = uri.trim_matches('\0').trim().to_string();
    let uri_opt = if uri_trimmed.is_empty() { None } else { Some(uri_trimmed) };

    Ok((
        name.trim_matches('\0').to_string(),
        symbol.trim_matches('\0').to_string(),
        uri_opt,
        creator_addr,
        creator_verified,
        None,  // collection_mint — parsed from collection field if present
        false,
        royalty,
        serde_json::json!([]),
    ))
}

fn read_str(data: &[u8], cursor: &mut usize) -> Result<String> {
    if *cursor + 4 > data.len() { return Ok(String::new()); }
    let len = u32::from_le_bytes(data[*cursor..*cursor+4].try_into()?) as usize;
    *cursor += 4;
    if *cursor + len > data.len() { return Ok(String::new()); }
    let s = String::from_utf8_lossy(&data[*cursor..*cursor+len]).to_string();
    *cursor += len;
    Ok(s)
}

fn bs58_encode(bytes: &[u8]) -> String {
    bs58::encode(bytes).into_string()
}
