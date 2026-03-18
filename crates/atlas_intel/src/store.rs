use anyhow::Result;
use sqlx::PgPool;
use crate::features::ExtractResult;
use crate::scores::ComputedScores;

/// Upsert co-occurrence wallet edges for `address`.
///
/// Strategy: find all addresses that appear in the same transactions as
/// `address` within the recent 7-day window, ranked by co-occurrence count.
/// Insert only top-20 peers to keep the edges table lean.
pub async fn upsert_edges(pool: &PgPool, address: &str) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO intelligence_wallet_edges (src, dst, reason, weight, updated_at)
           SELECT $1, peer, 'co_occurrence', cnt::float8, now()
           FROM (
               SELECT ai2.address AS peer, COUNT(*) AS cnt
               FROM address_index ai1
               JOIN address_index ai2
                   ON ai2.sig = ai1.sig AND ai2.address != $1
               WHERE ai1.address = $1
                 AND ai1.block_time >= EXTRACT(EPOCH FROM now() - INTERVAL '7 days')
                 -- exclude system programs and token programs
                 AND length(ai2.address) >= 32
               GROUP BY ai2.address
               ORDER BY cnt DESC
               LIMIT 20
           ) sub
           ON CONFLICT (src, dst, reason) DO UPDATE SET
               weight     = EXCLUDED.weight,
               updated_at = now()"#
    )
    .bind(address)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn upsert_profile(
    pool:    &PgPool,
    address: &str,
    window:  &str,
    result:  &ExtractResult,
    sc:      &ComputedScores,
) -> Result<()> {
    let features_json       = serde_json::to_value(&result.features)?;
    let top_programs_json   = serde_json::to_value(&result.top_programs)?;
    let top_tokens_json     = serde_json::to_value(&result.top_tokens)?;
    let top_cp_json         = serde_json::to_value(&result.top_counterparties)?;
    let wallet_type         = sc.wallet_type.as_str();

    sqlx::query(
        r#"INSERT INTO intelligence_wallet_profiles (
               address, "window", updated_at, wallet_type, confidence,
               automation_score, sniper_score, whale_score, risk_score,
               features_json, top_programs_json, top_tokens_json, top_counterparties_json
           ) VALUES ($1, $2, now(), $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
           ON CONFLICT (address, "window") DO UPDATE SET
               updated_at              = now(),
               wallet_type             = EXCLUDED.wallet_type,
               confidence              = EXCLUDED.confidence,
               automation_score        = EXCLUDED.automation_score,
               sniper_score            = EXCLUDED.sniper_score,
               whale_score             = EXCLUDED.whale_score,
               risk_score              = EXCLUDED.risk_score,
               features_json           = EXCLUDED.features_json,
               top_programs_json       = EXCLUDED.top_programs_json,
               top_tokens_json         = EXCLUDED.top_tokens_json,
               top_counterparties_json = EXCLUDED.top_counterparties_json"#
    )
    .bind(address)
    .bind(window)
    .bind(wallet_type)
    .bind(sc.confidence)
    .bind(sc.scores.automation as i32)
    .bind(sc.scores.sniper     as i32)
    .bind(sc.scores.whale      as i32)
    .bind(sc.scores.risk       as i32)
    .bind(features_json)
    .bind(top_programs_json)
    .bind(top_tokens_json)
    .bind(top_cp_json)
    .execute(pool)
    .await?;

    Ok(())
}
