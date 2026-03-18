use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use crate::error::ApiError;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Map a DB row from asset_index into the canonical DAS asset object.
fn asset_row_to_value(row: &sqlx::postgres::PgRow) -> Value {
    use sqlx::Row;
    let mint:       String  = row.try_get("mint").unwrap_or_default();
    let owner:      Option<String> = row.try_get("owner").ok().flatten();
    let asset_type: String  = row.try_get("asset_type").unwrap_or_default();
    let name:       String  = row.try_get("name").unwrap_or_default();
    let symbol:     String  = row.try_get("symbol").unwrap_or_default();
    let uri:        Option<String> = row.try_get("uri").ok().flatten();
    let image_uri:  Option<String> = row.try_get("image_uri").ok().flatten();
    let decimals:   i16     = row.try_get("decimals").unwrap_or(0);
    let supply: Decimal = row.try_get("supply").unwrap_or_default();
    let is_burned:  bool    = row.try_get("is_burned").unwrap_or(false);
    let is_compressed: bool = row.try_get("is_compressed").unwrap_or(false);
    let collection_mint:     Option<String> = row.try_get("collection_mint").ok().flatten();
    let collection_verified: bool           = row.try_get("collection_verified").unwrap_or(false);
    let creator:             Option<String> = row.try_get("creator").ok().flatten();
    let creator_verified:    bool           = row.try_get("creator_verified").unwrap_or(false);
    let update_authority:    Option<String> = row.try_get("update_authority").ok().flatten();
    let royalty_basis_pts:   i32            = row.try_get("royalty_basis_pts").unwrap_or(0);
    let attributes_json:     Value          = row.try_get("attributes_json").unwrap_or(json!([]));
    let metadata_json:       Value          = row.try_get("metadata_json").unwrap_or(json!({}));
    let slot_updated:        i64            = row.try_get("slot_updated").unwrap_or(0);
    let tree_address:        Option<String> = row.try_get("tree_address").ok().flatten();

    json!({
        "interface": interface_for_type(&asset_type, is_compressed),
        "id": mint,
        "content": {
            "metadata": {
                "name": name,
                "symbol": symbol,
                "attributes": attributes_json,
                "extra": metadata_json,
            },
            "links": {
                "image": image_uri,
                "external_url": uri,
            }
        },
        "ownership": {
            "owner": owner,
            "frozen": false,
            "delegated": false,
        },
        "compression": {
            "compressed": is_compressed,
            "tree": tree_address,
        },
        "grouping": collection_mint.as_ref().map(|c| vec![json!({
            "group_key": "collection",
            "group_value": c,
            "verified": collection_verified,
        })]).unwrap_or_default(),
        "royalty": {
            "basis_points": royalty_basis_pts,
            "percent": royalty_basis_pts as f64 / 100.0,
        },
        "creators": creator.as_ref().map(|c| vec![json!({
            "address": c,
            "verified": creator_verified,
            "share": 100,
        })]).unwrap_or_default(),
        "supply": supply.to_string(),
        "decimals": decimals,
        "burnt": is_burned,
        "update_authority": update_authority,
        "last_indexed_slot": slot_updated,
        "token_info": if asset_type == "fungible" {
            json!({
                "decimals": decimals,
                "supply": supply.to_string(),
            })
        } else {
            Value::Null
        },
    })
}

fn interface_for_type(asset_type: &str, compressed: bool) -> &'static str {
    match asset_type {
        "fungible"       => "FungibleToken",
        "compressed_nft" => "V1_NFT",
        "nft"            => "ProgrammableNFT",
        "inscription"    => "Custom",
        _                => if compressed { "V1_NFT" } else { "FungibleToken" },
    }
}

fn page_params(params: &Value) -> (i64, i64) {
    let page  = params["page"].as_u64().unwrap_or(1).max(1) as i64;
    let limit = params["limit"].as_u64().unwrap_or(100).min(1000) as i64;
    let offset = (page - 1) * limit;
    (limit, offset)
}

fn sort_clause(params: &Value) -> &'static str {
    let sort_by  = params["sortBy"]["sortBy"].as_str().unwrap_or("recent_action");
    let sort_dir = params["sortBy"]["sortDirection"].as_str().unwrap_or("desc");
    match (sort_by, sort_dir) {
        ("created",  "asc")  => "slot_created ASC",
        ("created",  _)      => "slot_created DESC",
        ("updated",  "asc")  => "slot_updated ASC",
        (_, "asc")           => "slot_updated ASC",
        _                    => "slot_updated DESC",
    }
}

// ── getAsset ─────────────────────────────────────────────────────────────────

pub async fn get_asset(pool: &PgPool, params: &Value) -> Result<Value, ApiError> {
    let id = params["id"].as_str()
        .ok_or_else(|| ApiError::BadRequest("id required".into()))?;

    let row = sqlx::query(
        "SELECT * FROM asset_index WHERE mint = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("asset {} not found", id)))?;

    Ok(asset_row_to_value(&row))
}

// ── getAssetBatch ─────────────────────────────────────────────────────────────

pub async fn get_asset_batch(pool: &PgPool, params: &Value) -> Result<Value, ApiError> {
    let ids: Vec<&str> = params["ids"].as_array()
        .ok_or_else(|| ApiError::BadRequest("ids array required".into()))?
        .iter()
        .filter_map(|v| v.as_str())
        .take(1000)
        .collect();

    if ids.is_empty() {
        return Ok(json!([]));
    }

    let rows = sqlx::query(
        "SELECT * FROM asset_index WHERE mint = ANY($1) ORDER BY slot_updated DESC"
    )
    .bind(&ids as &[&str])
    .fetch_all(pool)
    .await?;

    Ok(json!(rows.iter().map(asset_row_to_value).collect::<Vec<_>>()))
}

// ── getAssetsByOwner ──────────────────────────────────────────────────────────

pub async fn get_assets_by_owner(pool: &PgPool, params: &Value) -> Result<Value, ApiError> {
    let owner = params["ownerAddress"].as_str()
        .ok_or_else(|| ApiError::BadRequest("ownerAddress required".into()))?;

    let show_fungible = params["displayOptions"]["showFungible"].as_bool().unwrap_or(false);
    let show_nfts     = params["displayOptions"]["showNativeBalance"].as_bool().unwrap_or(true);
    let (limit, offset) = page_params(params);
    let sort = sort_clause(params);

    let type_filter = if show_fungible { "" } else { "AND asset_type != 'fungible'" };

    let query = format!(
        "SELECT * FROM asset_index WHERE owner = $1 {} ORDER BY {} LIMIT $2 OFFSET $3",
        type_filter, sort
    );

    let rows = sqlx::query(&query)
        .bind(owner)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let items: Vec<Value> = rows.iter().map(asset_row_to_value).collect();
    let total = items.len() as u64 + offset as u64;

    Ok(json!({
        "total": total,
        "limit": limit,
        "page": params["page"].as_u64().unwrap_or(1),
        "items": items,
    }))
}

// ── getAssetsByGroup ──────────────────────────────────────────────────────────

pub async fn get_assets_by_group(pool: &PgPool, params: &Value) -> Result<Value, ApiError> {
    let group_key   = params["groupKey"].as_str().unwrap_or("collection");
    let group_value = params["groupValue"].as_str()
        .ok_or_else(|| ApiError::BadRequest("groupValue required".into()))?;

    if group_key != "collection" {
        return Err(ApiError::BadRequest("only groupKey=collection supported".into()));
    }

    let (limit, offset) = page_params(params);
    let sort = sort_clause(params);

    let query = format!(
        "SELECT * FROM asset_index WHERE collection_mint = $1 ORDER BY {} LIMIT $2 OFFSET $3",
        sort
    );

    let rows = sqlx::query(&query)
        .bind(group_value)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let items: Vec<Value> = rows.iter().map(asset_row_to_value).collect();
    Ok(json!({
        "total": items.len(),
        "limit": limit,
        "page": params["page"].as_u64().unwrap_or(1),
        "items": items,
    }))
}

// ── getAssetsByCreator ────────────────────────────────────────────────────────

pub async fn get_assets_by_creator(pool: &PgPool, params: &Value) -> Result<Value, ApiError> {
    let creator       = params["creatorAddress"].as_str()
        .ok_or_else(|| ApiError::BadRequest("creatorAddress required".into()))?;
    let only_verified = params["onlyVerified"].as_bool().unwrap_or(false);
    let (limit, offset) = page_params(params);
    let sort = sort_clause(params);

    let verified_clause = if only_verified { "AND creator_verified = true" } else { "" };
    let query = format!(
        "SELECT * FROM asset_index WHERE creator = $1 {} ORDER BY {} LIMIT $2 OFFSET $3",
        verified_clause, sort
    );

    let rows = sqlx::query(&query)
        .bind(creator)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let items: Vec<Value> = rows.iter().map(asset_row_to_value).collect();
    Ok(json!({
        "total": items.len(),
        "limit": limit,
        "page": params["page"].as_u64().unwrap_or(1),
        "items": items,
    }))
}

// ── getAssetsByAuthority ──────────────────────────────────────────────────────

pub async fn get_assets_by_authority(pool: &PgPool, params: &Value) -> Result<Value, ApiError> {
    let authority = params["authorityAddress"].as_str()
        .ok_or_else(|| ApiError::BadRequest("authorityAddress required".into()))?;
    let (limit, offset) = page_params(params);
    let sort = sort_clause(params);

    let query = format!(
        "SELECT * FROM asset_index WHERE update_authority = $1 ORDER BY {} LIMIT $2 OFFSET $3",
        sort
    );

    let rows = sqlx::query(&query)
        .bind(authority)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let items: Vec<Value> = rows.iter().map(asset_row_to_value).collect();
    Ok(json!({
        "total": items.len(),
        "limit": limit,
        "page": params["page"].as_u64().unwrap_or(1),
        "items": items,
    }))
}

// ── searchAssets ──────────────────────────────────────────────────────────────

pub async fn search_assets(pool: &PgPool, params: &Value) -> Result<Value, ApiError> {
    let (limit, offset) = page_params(params);
    let sort = sort_clause(params);

    let mut conditions = vec!["1=1".to_string()];
    let mut binds: Vec<String> = vec![];
    let mut bind_idx = 1usize;

    if let Some(owner) = params["ownerAddress"].as_str() {
        conditions.push(format!("owner = ${}", bind_idx));
        binds.push(owner.to_string());
        bind_idx += 1;
    }
    if let Some(creator) = params["creatorAddress"].as_str() {
        conditions.push(format!("creator = ${}", bind_idx));
        binds.push(creator.to_string());
        bind_idx += 1;
        if params["creatorVerified"].as_bool().unwrap_or(false) {
            conditions.push("creator_verified = true".to_string());
        }
    }
    if let Some(group) = params["grouping"].as_array() {
        if group.len() >= 2 {
            if group[0].as_str() == Some("collection") {
                if let Some(col) = group[1].as_str() {
                    conditions.push(format!("collection_mint = ${}", bind_idx));
                    binds.push(col.to_string());
                    bind_idx += 1;
                }
            }
        }
    }
    if let Some(token_type) = params["tokenType"].as_str() {
        let type_cond = match token_type {
            "fungible"       => "asset_type = 'fungible'",
            "nonFungible"    => "asset_type = 'nft'",
            "regularNft"     => "asset_type = 'nft'",
            "compressedNft"  => "asset_type = 'compressed_nft'",
            _                => "1=1",
        };
        conditions.push(type_cond.to_string());
    }
    if let Some(burnt) = params["burnt"].as_bool() {
        conditions.push(format!("is_burned = {}", burnt));
    }
    if let Some(compressed) = params["compressed"].as_bool() {
        conditions.push(format!("is_compressed = {}", compressed));
    }

    let where_clause = conditions.join(" AND ");
    let query = format!(
        "SELECT * FROM asset_index WHERE {} ORDER BY {} LIMIT ${} OFFSET ${}",
        where_clause, sort, bind_idx, bind_idx + 1
    );

    let mut q = sqlx::query(&query);
    for b in &binds {
        q = q.bind(b);
    }
    let rows = q.bind(limit).bind(offset).fetch_all(pool).await?;

    let items: Vec<Value> = rows.iter().map(asset_row_to_value).collect();
    Ok(json!({
        "total": items.len(),
        "limit": limit,
        "page": params["page"].as_u64().unwrap_or(1),
        "items": items,
    }))
}

// ── getTokenAccounts ──────────────────────────────────────────────────────────

pub async fn get_token_accounts(pool: &PgPool, params: &Value) -> Result<Value, ApiError> {
    let (limit, offset) = page_params(params);

    let rows = if let Some(mint) = params["mint"].as_str() {
        sqlx::query(
            "SELECT * FROM token_account_index WHERE mint = $1 ORDER BY slot_updated DESC LIMIT $2 OFFSET $3"
        )
        .bind(mint)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else if let Some(owner) = params["owner"].as_str() {
        sqlx::query(
            "SELECT * FROM token_account_index WHERE owner = $1 ORDER BY slot_updated DESC LIMIT $2 OFFSET $3"
        )
        .bind(owner)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        return Err(ApiError::BadRequest("mint or owner required".into()));
    };

    use sqlx::Row;
    let items: Vec<Value> = rows.iter().map(|r| json!({
        "token_account": r.try_get::<String, _>("token_account").unwrap_or_default(),
        "owner":         r.try_get::<String, _>("owner").unwrap_or_default(),
        "mint":          r.try_get::<String, _>("mint").unwrap_or_default(),
        "amount":        r.try_get::<Decimal, _>("amount")
                          .map(|v| v.to_string()).unwrap_or_default(),
        "decimals":      r.try_get::<i16, _>("decimals").unwrap_or(0),
    })).collect();

    Ok(json!({
        "total": items.len(),
        "limit": limit,
        "page": params["page"].as_u64().unwrap_or(1),
        "token_accounts": items,
    }))
}

// ── getSignaturesForAsset ────────────────────────────────────────────────────

pub async fn get_signatures_for_asset(pool: &PgPool, params: &Value) -> Result<Value, ApiError> {
    let id = params["id"].as_str()
        .ok_or_else(|| ApiError::BadRequest("id (mint) required".into()))?;
    let (limit, offset) = page_params(params);

    // Check tx_store actions_json for any reference to this mint
    let rows = sqlx::query(
        r#"SELECT sig, slot, block_time, status
           FROM tx_store
           WHERE token_deltas_json @> $1
           ORDER BY slot DESC, pos DESC
           LIMIT $2 OFFSET $3"#
    )
    .bind(json!([{"mint": id}]))
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    use sqlx::Row;
    let items: Vec<Value> = rows.iter().map(|r| json!({
        "signature":  r.try_get::<String, _>("sig").unwrap_or_default(),
        "slot":       r.try_get::<i64, _>("slot").unwrap_or(0),
        "block_time": r.try_get::<Option<i64>, _>("block_time").ok().flatten(),
        "status":     r.try_get::<i16, _>("status").map(|s| if s == 1 { "confirmed" } else { "failed" }).unwrap_or("unknown"),
    })).collect();

    Ok(json!({
        "total": items.len(),
        "limit": limit,
        "page": params["page"].as_u64().unwrap_or(1),
        "items": items,
    }))
}

// ── getNftEditions ────────────────────────────────────────────────────────────

pub async fn get_nft_editions(pool: &PgPool, params: &Value) -> Result<Value, ApiError> {
    let master_mint = params["mint"].as_str()
        .ok_or_else(|| ApiError::BadRequest("mint required".into()))?;
    let (limit, offset) = page_params(params);

    // Editions share the same collection_mint as the master
    let rows = sqlx::query(
        r#"SELECT * FROM asset_index
           WHERE collection_mint = $1 AND asset_type = 'nft'
           ORDER BY slot_created ASC
           LIMIT $2 OFFSET $3"#
    )
    .bind(master_mint)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let items: Vec<Value> = rows.iter().map(asset_row_to_value).collect();
    Ok(json!({
        "master_edition_mint": master_mint,
        "supply": items.len(),
        "limit": limit,
        "page": params["page"].as_u64().unwrap_or(1),
        "editions": items,
    }))
}
