use anyhow::Result;
use sqlx::PgPool;
use atlas_types::facts::TxFactsV1;

/// UPSERT account balances from sol_deltas into the accounts table.
/// Uses post_lamports from each NativeDelta — only updates if slot is newer.
/// This gives us real-time XNT balances for every address that ever transacted.
pub async fn upsert_account_balances(pool: &PgPool, facts: &TxFactsV1) -> Result<()> {
    if facts.sol_deltas.is_empty() { return Ok(()); }

    let addresses:     Vec<String> = facts.sol_deltas.iter().map(|d| d.owner.clone()).collect();
    let post_lamports: Vec<i64>    = facts.sol_deltas.iter().map(|d| d.post_lamports as i64).collect();
    let slot = facts.slot as i64;

    sqlx::query(
        r#"INSERT INTO accounts (address, lamports, updated_slot, updated_at)
           SELECT addr, lam, $3, now()
           FROM UNNEST($1::text[], $2::bigint[]) AS t(addr, lam)
           ON CONFLICT (address) DO UPDATE SET
               lamports     = EXCLUDED.lamports,
               updated_slot = EXCLUDED.updated_slot,
               updated_at   = EXCLUDED.updated_at
           WHERE accounts.updated_slot < EXCLUDED.updated_slot"#
    )
    .bind(&addresses as &[String])
    .bind(&post_lamports as &[i64])
    .bind(slot)
    .execute(pool)
    .await?;

    Ok(())
}

/// UPSERT token account ownership from token_deltas into token_account_index.
/// Tracks which mint each owner holds — used for token balance queries.
pub async fn upsert_token_account_index(pool: &PgPool, facts: &TxFactsV1) -> Result<()> {
    for delta in &facts.token_deltas {
        sqlx::query(
            r#"INSERT INTO token_account_index (token_account, owner, mint, amount, slot_updated, updated_at)
               VALUES ($1, $2, $3, 0, $4, now())
               ON CONFLICT (token_account) DO UPDATE SET
                   owner        = EXCLUDED.owner,
                   mint         = EXCLUDED.mint,
                   slot_updated = EXCLUDED.slot_updated,
                   updated_at   = EXCLUDED.updated_at
               WHERE token_account_index.slot_updated < EXCLUDED.slot_updated"#
        )
        .bind(&delta.account)
        .bind(&delta.owner)
        .bind(&delta.mint)
        .bind(facts.slot as i64)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// UPSERT tx_store. On conflict, update only if new commitment is stronger.
pub async fn upsert_tx(pool: &PgPool, facts: &TxFactsV1) -> Result<()> {
    let accounts_json  = serde_json::to_value(&facts.accounts)?;
    let actions_json   = serde_json::to_value(&facts.actions)?;
    let tok_deltas     = serde_json::to_value(&facts.token_deltas)?;
    let sol_deltas_v   = serde_json::to_value(&facts.sol_deltas)?;
    let status         = facts.status.as_smallint();
    let commitment_rank = facts.commitment.rank() as i32;

    sqlx::query(
        r#"INSERT INTO tx_store (
            sig, slot, pos, block_time, status, fee_lamports,
            compute_consumed, compute_limit, priority_fee_micro_lamports,
            programs, tags, accounts_json, actions_json,
            token_deltas_json, sol_deltas_json, err_json, raw_ref, commitment
           ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
           ON CONFLICT (sig) DO UPDATE SET
               block_time                  = EXCLUDED.block_time,
               status                      = EXCLUDED.status,
               fee_lamports                = EXCLUDED.fee_lamports,
               compute_consumed            = EXCLUDED.compute_consumed,
               compute_limit               = EXCLUDED.compute_limit,
               priority_fee_micro_lamports = EXCLUDED.priority_fee_micro_lamports,
               programs                    = EXCLUDED.programs,
               tags                        = EXCLUDED.tags,
               accounts_json               = EXCLUDED.accounts_json,
               actions_json                = EXCLUDED.actions_json,
               token_deltas_json           = EXCLUDED.token_deltas_json,
               sol_deltas_json             = EXCLUDED.sol_deltas_json,
               err_json                    = EXCLUDED.err_json,
               commitment                  = EXCLUDED.commitment
           WHERE (CASE tx_store.commitment
               WHEN 'shred' THEN -1 WHEN 'processed' THEN 0 WHEN 'confirmed' THEN 1 WHEN 'finalized' THEN 2 ELSE -1 END) < $19"#
    )
    .bind(&facts.sig)
    .bind(facts.slot as i64)
    .bind(facts.pos as i32)
    .bind(facts.block_time)
    .bind(status)
    .bind(facts.fee_lamports as i64)
    .bind(facts.compute_units.consumed.map(|v| v as i32))
    .bind(facts.compute_units.limit.map(|v| v as i32))
    .bind(facts.compute_units.price_micro_lamports.map(|v| v as i64))
    .bind(&facts.programs as &[String])
    .bind(&facts.tags     as &[String])
    .bind(&accounts_json)
    .bind(&actions_json)
    .bind(&tok_deltas)
    .bind(&sol_deltas_v)
    .bind(facts.err.as_ref())
    .bind(facts.raw_ref.as_deref())
    .bind(facts.commitment.as_str())
    .bind(commitment_rank)
    .execute(pool)
    .await?;

    Ok(())
}

/// Batch UPSERT address_index using a single UNNEST query for O(1) round-trips.
pub async fn upsert_address_index_batch(pool: &PgPool, facts: &TxFactsV1) -> Result<()> {
    let addresses = facts.all_addresses();
    if addresses.is_empty() { return Ok(()); }

    let action_types = facts.action_types();
    let slot      = facts.slot as i64;
    let pos       = facts.pos  as i32;
    let sig       = &facts.sig;
    let block_time = facts.block_time;
    let status    = facts.status.as_smallint();
    let tags      = &facts.tags as &[String];
    let atypes    = &action_types as &[String];

    // One INSERT per tx — UNNEST expands the address list; all other columns are constant.
    sqlx::query(
        r#"INSERT INTO address_index (address, slot, pos, sig, block_time, status, tags, action_types)
           SELECT addr, $2, $3, $4, $5, $6, $7, $8
           FROM UNNEST($1::text[]) AS addr
           ON CONFLICT DO NOTHING"#
    )
    .bind(&addresses as &[String])
    .bind(slot)
    .bind(pos)
    .bind(sig)
    .bind(block_time)
    .bind(status)
    .bind(tags)
    .bind(atypes)
    .execute(pool)
    .await?;

    Ok(())
}

/// UPSERT token_balance_index rows.
pub async fn upsert_token_balance_index(pool: &PgPool, facts: &TxFactsV1) -> Result<()> {
    for delta in &facts.token_deltas {
        let delta_val: sqlx::types::Decimal = delta.delta.parse().unwrap_or_default();
        sqlx::query(
            r#"INSERT INTO token_balance_index (owner, slot, pos, sig, mint, delta, direction)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT DO NOTHING"#
        )
        .bind(&delta.owner)
        .bind(facts.slot as i64)
        .bind(facts.pos as i32)
        .bind(&facts.sig)
        .bind(&delta.mint)
        .bind(delta_val)
        .bind(delta.direction.as_smallint())
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// UPSERT program_activity_index rows.
pub async fn upsert_program_activity(pool: &PgPool, facts: &TxFactsV1) -> Result<()> {
    for program_id in &facts.programs {
        sqlx::query(
            r#"INSERT INTO program_activity_index (program_id, slot, pos, sig, block_time, tags)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT DO NOTHING"#
        )
        .bind(program_id)
        .bind(facts.slot as i64)
        .bind(facts.pos as i32)
        .bind(&facts.sig)
        .bind(facts.block_time)
        .bind(&facts.tags as &[String])
        .execute(pool)
        .await?;
    }
    Ok(())
}
