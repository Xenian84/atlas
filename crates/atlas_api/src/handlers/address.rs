use axum::{extract::{Path, State, Query}, http::HeaderMap, response::Response};
use serde::Deserialize;
use sqlx::Row;
use atlas_types::facts::{TxHistoryPage, TxSummary, TxStatus};
use atlas_types::cursor::SlotPosCursor;
use atlas_toon::render_tx_history;
use atlas_common::redis_ext;
use crate::{state::AppState, error::ApiError, negotiate::{negotiate, respond}};

/// Range filter for blockTime / slot (mirrors Helius filter syntax).
#[derive(Deserialize, Default, Clone)]
pub struct RangeFilter {
    pub gte: Option<i64>,
    pub gt:  Option<i64>,
    pub lte: Option<i64>,
    pub lt:  Option<i64>,
}

impl RangeFilter {
    pub fn is_set(&self) -> bool {
        self.gte.is_some() || self.gt.is_some() || self.lte.is_some() || self.lt.is_some()
    }
    /// Lower bound (exclusive slot / ts): value must be > lower
    pub fn lower_exclusive(&self) -> Option<i64> {
        self.gt
    }
    /// Lower bound (inclusive): value must be >= lower
    pub fn lower_inclusive(&self) -> Option<i64> {
        self.gte
    }
    /// Upper bound (exclusive): value must be < upper
    pub fn upper_exclusive(&self) -> Option<i64> {
        self.lt
    }
    /// Upper bound (inclusive): value must be <= upper
    pub fn upper_inclusive(&self) -> Option<i64> {
        self.lte
    }
}

#[derive(Deserialize, Default)]
pub struct TxsQuery {
    /// Max results. Up to 1000 for signature-only, up to 100 for full tx detail.
    pub limit:      Option<usize>,
    /// Keyset cursor from a previous response (`next_cursor` field).
    pub before:     Option<String>,
    /// Type filter: `swap`, `transfer`, `balanceChanged`, or `all`.
    #[serde(rename = "type")]
    pub tx_type:    Option<String>,
    /// Sort direction: `desc` (newest first, default) or `asc` (oldest first).
    #[serde(rename = "sortOrder")]
    pub sort_order: Option<String>,
    /// Filter by transaction status: `succeeded`, `failed`, or `any` (default).
    pub status:     Option<String>,
    /// Filter by Unix block time range.
    #[serde(rename = "blockTime")]
    pub block_time: Option<RangeFilter>,
    /// Filter by slot range.
    pub slot:       Option<RangeFilter>,
    /// Response format: `json` or `toon`.
    pub format:     Option<String>,
}

/// Parsed, validated query options passed through to fetch helpers.
#[derive(Clone, Default)]
pub struct TxsOpts {
    pub limit:      usize,
    pub cursor:     Option<SlotPosCursor>,
    pub tx_type:    String,
    pub sort_asc:   bool,
    pub status:     Option<i16>,   // None=any, Some(1)=success, Some(2)=fail
    pub block_time: Option<RangeFilter>,
    pub slot:       Option<RangeFilter>,
}

impl TxsOpts {
    pub fn from_query(q: &TxsQuery) -> Self {
        let sort_asc  = q.sort_order.as_deref() == Some("asc");
        let limit     = q.limit.unwrap_or(100).min(1000);
        let cursor    = q.before.as_deref().and_then(|s| s.parse::<SlotPosCursor>().ok());
        let tx_type   = q.tx_type.clone().unwrap_or_else(|| "all".into());
        let status    = match q.status.as_deref() {
            Some("succeeded") => Some(1i16),
            Some("failed")    => Some(2i16),
            _                 => None,
        };
        Self { limit, cursor, tx_type, sort_asc, status, block_time: q.block_time.clone(), slot: q.slot.clone() }
    }

    pub fn from_rpc(opts: &serde_json::Value) -> Self {
        let sort_asc  = opts["sortOrder"].as_str() == Some("asc");
        let limit     = opts["limit"].as_u64().unwrap_or(100).min(1000) as usize;
        let cursor    = opts["before"].as_str().and_then(|s| s.parse::<SlotPosCursor>().ok())
            .or_else(|| opts["paginationToken"].as_str().and_then(|s| s.parse::<SlotPosCursor>().ok()));
        let tx_type   = opts["type"].as_str().unwrap_or("all").to_string();
        let status    = match opts["filters"]["status"].as_str() {
            Some("succeeded") => Some(1i16),
            Some("failed")    => Some(2i16),
            _                 => None,
        };
        let block_time = serde_json::from_value::<RangeFilter>(opts["filters"]["blockTime"].clone()).ok()
            .filter(|r| r.is_set());
        let slot = serde_json::from_value::<RangeFilter>(opts["filters"]["slot"].clone()).ok()
            .filter(|r| r.is_set());
        Self { limit, cursor, tx_type, sort_asc, status, block_time, slot }
    }
}

/// GET /v1/address/:addr/txs
pub async fn get_address_txs(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
    Query(q):     Query<TxsQuery>,
    headers:      HeaderMap,
) -> Result<Response, ApiError> {
    let opts   = TxsOpts::from_query(&q);
    let format = negotiate(&headers, q.format.as_deref());
    let page   = fetch_address_txs(&state, &addr, opts).await?;
    let toon   = render_tx_history(&page);
    Ok(respond(format, &page, toon))
}

/// Core query — used by REST and JSON-RPC handlers.
pub async fn fetch_address_txs(
    state:   &AppState,
    address: &str,
    opts:    TxsOpts,
) -> Result<TxHistoryPage, ApiError> {
    let cache_key = format!(
        "addr:{}:before:{}:limit:{}:type:{}:asc:{}:status:{:?}",
        address,
        opts.cursor.as_ref().map(|c| format!("{}:{}", c.slot, c.pos)).unwrap_or_else(|| "none".into()),
        opts.limit,
        opts.tx_type,
        opts.sort_asc,
        opts.status,
    );
    let mut redis = state.redis();

    // Skip cache when range filters are active (too many variations)
    let use_cache = opts.block_time.is_none() && opts.slot.is_none();

    if use_cache {
        if let Some(cached) = redis_ext::cache_get::<TxHistoryPage>(&mut redis, &cache_key).await {
            metrics::counter!(atlas_common::metrics::CACHE_HIT_TOTAL).increment(1);
            return Ok(cached);
        }
    }
    metrics::counter!(atlas_common::metrics::CACHE_MISS_TOTAL).increment(1);

    // Step 1: keyset pagination on address_index
    let index_rows = match opts.tx_type.as_str() {
        "swap"           => fetch_index_rows(state, address, &opts, Some("SWAP")).await?,
        "transfer"       => fetch_index_rows(state, address, &opts, Some("TRANSFER")).await?,
        "balanceChanged" => fetch_index_rows_with_balance(state, address, &opts).await?,
        _                => fetch_index_rows(state, address, &opts, None).await?,
    };

    if index_rows.is_empty() {
        return Ok(TxHistoryPage { address: address.into(), limit: opts.limit, next_cursor: None, transactions: vec![] });
    }

    let sigs: Vec<String> = index_rows.iter()
        .map(|r| r.try_get::<String, _>("sig").unwrap_or_default())
        .collect();

    // Only set next_cursor when we got a full page — signals more data exists
    let next_cursor = if index_rows.len() == opts.limit {
        let last = index_rows.last().unwrap();
        let ls: i64 = last.try_get("slot").unwrap_or(0);
        let lp: i32 = last.try_get("pos").unwrap_or(0);
        Some(format!("{}:{}", ls, lp))
    } else {
        None
    };

    // Step 2: batch-fetch full tx facts for the page
    let tx_rows = sqlx::query(
        r#"SELECT sig, slot, pos, block_time, status, fee_lamports, tags,
                  actions_json, token_deltas_json
           FROM tx_store WHERE sig = ANY($1)"#
    )
    .bind(&sigs as &[String])
    .fetch_all(state.pool())
    .await?;

    use std::collections::HashMap;
    let mut tx_map: HashMap<String, _> = HashMap::new();
    for row in &tx_rows {
        let sig: String = row.try_get("sig").unwrap_or_default();
        tx_map.insert(sig, row);
    }

    let mut transactions = vec![];
    for idx_row in &index_rows {
        let sig: String = idx_row.try_get("sig").unwrap_or_default();
        let Some(tx) = tx_map.get(&sig) else { continue };

        let actions: Vec<atlas_types::facts::Action> =
            serde_json::from_value(tx.try_get("actions_json").unwrap_or(serde_json::Value::Array(vec![]))).unwrap_or_default();
        let token_deltas: Vec<atlas_types::facts::TokenDelta> =
            serde_json::from_value(tx.try_get("token_deltas_json").unwrap_or(serde_json::Value::Array(vec![]))).unwrap_or_default();

        if opts.tx_type == "balanceChanged" && token_deltas.is_empty() { continue; }

        // Apply status filter post-fetch (status stored as i16: 1=success, 2=fail)
        let tx_status = tx.try_get::<i16, _>("status").unwrap_or(1);
        if let Some(want) = opts.status {
            if tx_status != want { continue; }
        }

        let action_types: Vec<String> = idx_row
            .try_get::<Vec<String>, _>("action_types")
            .unwrap_or_else(|_| {
                let mut t: Vec<String> = actions.iter().map(|a| a.t.clone()).collect();
                t.sort(); t.dedup(); t
            });

        transactions.push(TxSummary {
            signature:    sig,
            slot:         tx.try_get::<i64, _>("slot").unwrap_or(0) as u64,
            pos:          tx.try_get::<i32, _>("pos").unwrap_or(0) as u32,
            block_time:   tx.try_get("block_time").ok(),
            status:       TxStatus::from_smallint(tx_status),
            fee_lamports: tx.try_get::<i64, _>("fee_lamports").unwrap_or(0) as u64,
            tags:         tx.try_get("tags").unwrap_or_default(),
            action_types,
            actions,
            token_deltas,
        });
    }

    let page = TxHistoryPage { address: address.into(), limit: opts.limit, next_cursor, transactions };
    if use_cache {
        let _ = redis_ext::cache_set(&mut redis, &cache_key, &page, 10).await;
    }
    Ok(page)
}

/// Build the ORDER BY clause based on sort direction.
fn order_clause(asc: bool) -> &'static str {
    if asc { "ORDER BY slot ASC,  pos ASC  LIMIT" }
    else   { "ORDER BY slot DESC, pos DESC LIMIT" }
}

/// Cursor comparison operator — flips for ASC pagination.
fn cursor_op(asc: bool) -> &'static str {
    if asc { ">" } else { "<" }
}

/// Append blockTime / slot range conditions to a WHERE clause string and bind list.
/// Returns the extra SQL fragment (may be empty) and advances `$n` counters.
fn range_conditions(
    block_time: Option<&RangeFilter>,
    slot:       Option<&RangeFilter>,
    next_n:     &mut i32,
) -> String {
    let mut clauses = String::new();
    let append = |clauses: &mut String, col: &str, op: &str, n: i32| {
        clauses.push_str(&format!(" AND {col} {op} ${n}"));
    };
    if let Some(f) = block_time {
        if f.gte.is_some() { append(&mut clauses, "block_time", ">=", *next_n); *next_n += 1; }
        if f.gt.is_some()  { append(&mut clauses, "block_time", ">",  *next_n); *next_n += 1; }
        if f.lte.is_some() { append(&mut clauses, "block_time", "<=", *next_n); *next_n += 1; }
        if f.lt.is_some()  { append(&mut clauses, "block_time", "<",  *next_n); *next_n += 1; }
    }
    if let Some(f) = slot {
        if f.gte.is_some() { append(&mut clauses, "slot", ">=", *next_n); *next_n += 1; }
        if f.gt.is_some()  { append(&mut clauses, "slot", ">",  *next_n); *next_n += 1; }
        if f.lte.is_some() { append(&mut clauses, "slot", "<=", *next_n); *next_n += 1; }
        if f.lt.is_some()  { append(&mut clauses, "slot", "<",  *next_n); *next_n += 1; }
    }
    clauses
}

/// Bind all range filter values onto a query builder.
fn bind_ranges<'q>(
    mut q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    block_time: Option<&'q RangeFilter>,
    slot:       Option<&'q RangeFilter>,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    if let Some(f) = block_time {
        if let Some(v) = f.gte { q = q.bind(v); }
        if let Some(v) = f.gt  { q = q.bind(v); }
        if let Some(v) = f.lte { q = q.bind(v); }
        if let Some(v) = f.lt  { q = q.bind(v); }
    }
    if let Some(f) = slot {
        if let Some(v) = f.gte { q = q.bind(v); }
        if let Some(v) = f.gt  { q = q.bind(v); }
        if let Some(v) = f.lte { q = q.bind(v); }
        if let Some(v) = f.lt  { q = q.bind(v); }
    }
    q
}

/// Fetch address_index rows with optional action_type + range filters.
async fn fetch_index_rows(
    state:       &AppState,
    address:     &str,
    opts:        &TxsOpts,
    type_filter: Option<&str>,
) -> Result<Vec<sqlx::postgres::PgRow>, ApiError> {
    let asc = opts.sort_asc;
    let op  = cursor_op(asc);
    let ord = order_clause(asc);

    // Build dynamic WHERE suffix for range filters
    // Param positions after fixed binds:
    //   cursor variant:    $1=address, $2=slot, $3=pos, [$4=type], then ranges, then $N=limit
    //   no-cursor variant: $1=address, [$2=type], then ranges, then $N=limit
    let (cursor_clause, base_n_type, base_n_no_type) = match &opts.cursor {
        Some(_) => (format!("AND (slot, pos) {op} ($2, $3)"), 4i32, 3i32),
        None    => ("".to_string(), 3i32, 2i32),
    };
    let type_clause = type_filter.map(|_| {
        let n = if opts.cursor.is_some() { base_n_type - 1 } else { base_n_no_type - 1 };
        format!("AND ${n} = ANY(action_types)")
    }).unwrap_or_default();

    // Recalculate starting param index for ranges
    let mut next_n_with_type    = base_n_type;
    let mut next_n_without_type = base_n_no_type;
    let range_sql_with    = range_conditions(opts.block_time.as_ref(), opts.slot.as_ref(), &mut next_n_with_type);
    let range_sql_without = range_conditions(opts.block_time.as_ref(), opts.slot.as_ref(), &mut next_n_without_type);

    let sql = match (opts.cursor.is_some(), type_filter.is_some()) {
        (true, true) => format!(
            "SELECT sig, slot, pos, block_time, status, tags, action_types \
             FROM address_index \
             WHERE address = $1 {cursor_clause} {type_clause}{range_sql_with} \
             {ord} ${next_n_with_type}"
        ),
        (true, false) => format!(
            "SELECT sig, slot, pos, block_time, status, tags, action_types \
             FROM address_index \
             WHERE address = $1 {cursor_clause}{range_sql_with} \
             {ord} ${next_n_with_type}"
        ),
        (false, true) => format!(
            "SELECT sig, slot, pos, block_time, status, tags, action_types \
             FROM address_index \
             WHERE address = $1 {type_clause}{range_sql_without} \
             {ord} ${next_n_without_type}"
        ),
        (false, false) => format!(
            "SELECT sig, slot, pos, block_time, status, tags, action_types \
             FROM address_index \
             WHERE address = $1{range_sql_without} \
             {ord} ${next_n_without_type}"
        ),
    };

    let mut q = sqlx::query(&sql).bind(address);
    if let Some(c) = &opts.cursor {
        q = q.bind(c.slot as i64).bind(c.pos as i32);
    }
    if let Some(t) = type_filter {
        q = q.bind(t);
    }
    q = bind_ranges(q, opts.block_time.as_ref(), opts.slot.as_ref());
    q = q.bind(opts.limit as i64);

    Ok(q.fetch_all(state.pool()).await?)
}

/// Fetch address_index rows with at least one token balance change + range filters.
async fn fetch_index_rows_with_balance(
    state:   &AppState,
    address: &str,
    opts:    &TxsOpts,
) -> Result<Vec<sqlx::postgres::PgRow>, ApiError> {
    let asc = opts.sort_asc;
    let op  = cursor_op(asc);
    let ord = order_clause(asc);

    let (cursor_clause, mut next_n) = match &opts.cursor {
        Some(_) => (format!("AND (ai.slot, ai.pos) {op} ($2, $3)"), 4i32),
        None    => ("".to_string(), 2i32),
    };
    let range_sql = range_conditions(opts.block_time.as_ref(), opts.slot.as_ref(), &mut next_n);

    let sql = format!(
        "SELECT ai.sig, ai.slot, ai.pos, ai.block_time, ai.status, ai.tags, ai.action_types \
         FROM address_index ai \
         WHERE ai.address = $1 {cursor_clause} \
           AND EXISTS (SELECT 1 FROM token_balance_index t WHERE t.sig = ai.sig) \
           {range_sql} \
         {ord} ${next_n}"
    );

    let mut q = sqlx::query(&sql).bind(address);
    if let Some(c) = &opts.cursor {
        q = q.bind(c.slot as i64).bind(c.pos as i32);
    }
    q = bind_ranges(q, opts.block_time.as_ref(), opts.slot.as_ref());
    q = q.bind(opts.limit as i64);

    Ok(q.fetch_all(state.pool()).await?)
}
