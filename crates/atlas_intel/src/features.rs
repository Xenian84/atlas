use anyhow::Result;
use sqlx::{PgPool, Row};
use chrono::Utc;
use atlas_types::intelligence::{WalletFeatures, ProgramUsage, TokenUsage};


/// Feature extraction result, extended with top-N lists for store.rs
pub struct ExtractResult {
    pub features:         WalletFeatures,
    pub top_programs:     Vec<ProgramUsage>,
    pub top_tokens:       Vec<TokenUsage>,
    pub top_counterparties: Vec<String>,
}

pub async fn extract(pool: &PgPool, address: &str, window: &str) -> Result<ExtractResult> {
    let window_start: Option<i64> = window_to_ts(window);

    // Base stats: tx_count, active_days, failure_rate, avg_fee, avg_priority_fee
    macro_rules! stats_query {
        ($extra:literal) => { concat!(
            r#"SELECT COUNT(*) AS tx_count,
                      COUNT(DISTINCT DATE(TO_TIMESTAMP(ai.block_time))) AS active_days,
                      (SUM(CASE WHEN ai.status = 2 THEN 1 ELSE 0 END)::float8 / NULLIF(COUNT(*),0)) AS failure_rate,
                      AVG(t.fee_lamports)::bigint AS avg_fee,
                      AVG(t.priority_fee_micro_lamports)::bigint AS avg_pfee
               FROM address_index ai
               JOIN tx_store t ON t.sig = ai.sig
               WHERE ai.address = $1"#, $extra
        )}
    }

    let stats = if let Some(ts) = window_start {
        sqlx::query(stats_query!(" AND ai.block_time >= $2"))
            .bind(address).bind(ts).fetch_one(pool).await?
    } else {
        sqlx::query(stats_query!(""))
            .bind(address).fetch_one(pool).await?
    };

    // Action counts via JSONB lateral
    macro_rules! action_query {
        ($extra:literal) => { concat!(
            r#"SELECT
                   SUM(CASE WHEN action->>'t'='SWAP'     THEN 1 ELSE 0 END) AS swaps,
                   SUM(CASE WHEN action->>'t'='TRANSFER' THEN 1 ELSE 0 END) AS transfers,
                   SUM(CASE WHEN action->>'t'='MINT'     THEN 1 ELSE 0 END) AS mints,
                   SUM(CASE WHEN action->>'t'='BURN'     THEN 1 ELSE 0 END) AS burns,
                   SUM(CASE WHEN action->>'t'='DEPLOY'   THEN 1 ELSE 0 END) AS deploys
               FROM address_index ai
               JOIN tx_store t ON t.sig = ai.sig
               CROSS JOIN LATERAL jsonb_array_elements(t.actions_json) AS action
               WHERE ai.address = $1"#, $extra
        )}
    }

    let action_q = if let Some(ts) = window_start {
        sqlx::query(action_query!(" AND ai.block_time >= $2"))
            .bind(address).bind(ts).fetch_one(pool).await?
    } else {
        sqlx::query(action_query!(""))
            .bind(address).fetch_one(pool).await?
    };

    // Burstiness: max tx in a 10-minute window
    macro_rules! burst_query {
        ($extra:literal) => { concat!(
            r#"SELECT COALESCE(MAX(cnt),0) AS max_burst FROM
               (SELECT COUNT(*) AS cnt FROM address_index WHERE address=$1"#,
            $extra,
            " GROUP BY (block_time/600)) sub"
        )}
    }

    let burst_q = if let Some(ts) = window_start {
        sqlx::query(burst_query!(" AND block_time >= $2"))
            .bind(address).bind(ts).fetch_one(pool).await?
    } else {
        sqlx::query(burst_query!(""))
            .bind(address).fetch_one(pool).await?
    };

    // Two separate queries for unique_programs and top_programs
    let (unique_programs, top_programs) = fetch_unique_programs(pool, address, window_start).await?;

    // Unique tokens from token_balance_index
    let (unique_tokens, top_tokens) = fetch_unique_tokens(pool, address, window_start).await?;

    // Net XNT delta for this address
    let net_sol_delta = fetch_net_xnt_delta(pool, address, window_start).await?;

    Ok(ExtractResult {
        features: WalletFeatures {
            tx_count:              stats.try_get::<i64, _>("tx_count").unwrap_or(0) as u64,
            active_days:           stats.try_get::<i64, _>("active_days").unwrap_or(0) as u32,
            burstiness:            burst_q.try_get::<i64, _>("max_burst").unwrap_or(0) as u32,
            unique_programs:       unique_programs as u32,
            unique_tokens:         unique_tokens as u32,
            unique_counterparties: 0,
            failure_rate:          stats.try_get::<f64, _>("failure_rate").unwrap_or(0.0),
            swap_count:            action_q.try_get::<i64, _>("swaps").unwrap_or(0) as u64,
            transfer_count:        action_q.try_get::<i64, _>("transfers").unwrap_or(0) as u64,
            mint_count:            action_q.try_get::<i64, _>("mints").unwrap_or(0) as u64,
            burn_count:            action_q.try_get::<i64, _>("burns").unwrap_or(0) as u64,
            avg_fee_lamports:      stats.try_get::<Option<i64>, _>("avg_fee").unwrap_or_default().unwrap_or(0) as u64,
            avg_priority_fee:      stats.try_get::<Option<i64>, _>("avg_pfee").unwrap_or_default().map(|v| v as u64),
            net_sol_delta,
            has_deploy_actions:    action_q.try_get::<i64, _>("deploys").unwrap_or(0) > 0,
        },
        top_programs,
        top_tokens,
        top_counterparties: vec![],
    })
}

async fn fetch_unique_programs(
    pool:    &PgPool,
    address: &str,
    window:  Option<i64>,
) -> Result<(i64, Vec<ProgramUsage>)> {
    let rows = if let Some(ts) = window {
        sqlx::query(
            r#"SELECT unnested_prog AS program_id, COUNT(*) AS cnt
               FROM address_index ai
               JOIN tx_store t ON t.sig = ai.sig
               CROSS JOIN LATERAL UNNEST(t.programs) AS unnested_prog
               WHERE ai.address = $1 AND ai.block_time >= $2
               GROUP BY unnested_prog ORDER BY cnt DESC LIMIT 10"#
        ).bind(address).bind(ts).fetch_all(pool).await?
    } else {
        sqlx::query(
            r#"SELECT unnested_prog AS program_id, COUNT(*) AS cnt
               FROM address_index ai
               JOIN tx_store t ON t.sig = ai.sig
               CROSS JOIN LATERAL UNNEST(t.programs) AS unnested_prog
               WHERE ai.address = $1
               GROUP BY unnested_prog ORDER BY cnt DESC LIMIT 10"#
        ).bind(address).fetch_all(pool).await?
    };

    let unique = rows.len() as i64;
    let top = rows.iter().take(5).map(|r| ProgramUsage {
        program_id: r.try_get("program_id").unwrap_or_default(),
        call_count: r.try_get::<i64, _>("cnt").unwrap_or(0) as u64,
    }).collect();

    Ok((unique, top))
}

async fn fetch_unique_tokens(
    pool:    &PgPool,
    address: &str,
    _window: Option<i64>,
) -> Result<(i64, Vec<TokenUsage>)> {
    // token_balance_index has no block_time; use all-time count for now
    let rows = sqlx::query(
        r#"SELECT mint, COUNT(*) AS cnt
           FROM token_balance_index
           WHERE owner = $1
           GROUP BY mint ORDER BY cnt DESC LIMIT 10"#
    ).bind(address).fetch_all(pool).await?;

    let unique = rows.len() as i64;
    let top = rows.iter().take(5).map(|r| TokenUsage {
        mint:      r.try_get("mint").unwrap_or_default(),
        abs_delta: r.try_get::<i64, _>("cnt").unwrap_or(0).to_string(),
        symbol:    None,
    }).collect();

    Ok((unique, top))
}

async fn fetch_net_xnt_delta(
    pool:    &PgPool,
    address: &str,
    window:  Option<i64>,
) -> Result<i64> {
    let row = if let Some(ts) = window {
        sqlx::query(
            r#"SELECT COALESCE(SUM((entry->>'delta_lamports')::bigint), 0) AS net_delta
               FROM address_index ai
               JOIN tx_store t ON t.sig = ai.sig
               CROSS JOIN LATERAL jsonb_array_elements(t.sol_deltas_json) AS entry
               WHERE ai.address = $1 AND ai.block_time >= $2
                 AND entry->>'owner' = $1"#
        ).bind(address).bind(ts).fetch_one(pool).await?
    } else {
        sqlx::query(
            r#"SELECT COALESCE(SUM((entry->>'delta_lamports')::bigint), 0) AS net_delta
               FROM address_index ai
               JOIN tx_store t ON t.sig = ai.sig
               CROSS JOIN LATERAL jsonb_array_elements(t.sol_deltas_json) AS entry
               WHERE ai.address = $1
                 AND entry->>'owner' = $1"#
        ).bind(address).fetch_one(pool).await?
    };

    Ok(row.try_get::<i64, _>("net_delta").unwrap_or(0))
}

fn window_to_ts(window: &str) -> Option<i64> {
    let now = Utc::now().timestamp();
    match window {
        "24h" => Some(now - 86400),
        "7d"  => Some(now - 7 * 86400),
        "30d" => Some(now - 30 * 86400),
        _     => None,
    }
}
