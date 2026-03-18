//! GET /v1/address/:addr/related — wallets that frequently co-appear with this address.

use axum::{extract::{State, Path, Query}, Json};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use crate::{state::AppState, error::ApiError};

#[derive(Debug, Deserialize)]
pub struct RelatedQuery {
    /// Max results (default 20, max 50).
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct RelatedWallet {
    pub address: String,
    pub reason:  String,
    pub weight:  f64,
    /// Identity label if known.
    pub name:    Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RelatedResponse {
    pub address: String,
    pub related: Vec<RelatedWallet>,
}

pub async fn get_related(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
    Query(q):     Query<RelatedQuery>,
) -> Result<Json<RelatedResponse>, ApiError> {
    let limit = q.limit.unwrap_or(20).min(50);

    // Fetch edges in both directions (this address as src or dst).
    let rows = sqlx::query(
        r#"SELECT
               CASE WHEN e.src = $1 THEN e.dst ELSE e.src END AS peer,
               e.reason,
               e.weight,
               el.name
           FROM intelligence_wallet_edges e
           LEFT JOIN entity_labels el
               ON el.address = CASE WHEN e.src = $1 THEN e.dst ELSE e.src END
           WHERE e.src = $1 OR e.dst = $1
           ORDER BY e.weight DESC
           LIMIT $2"#
    )
    .bind(&addr)
    .bind(limit)
    .fetch_all(state.pool())
    .await?;

    let related = rows.iter().map(|r| RelatedWallet {
        address: r.try_get("peer").unwrap_or_default(),
        reason:  r.try_get("reason").unwrap_or_default(),
        weight:  r.try_get::<f64, _>("weight").unwrap_or(1.0),
        name:    r.try_get("name").ok(),
    }).collect();

    Ok(Json(RelatedResponse { address: addr, related }))
}
