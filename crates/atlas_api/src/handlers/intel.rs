use axum::{extract::{Path, State, Query}, Json};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use atlas_common::redis_ext;
use crate::{state::AppState, error::ApiError};

#[derive(Deserialize, Default)]
pub struct WindowQuery {
    pub window: Option<String>,
}

const VALID_WINDOWS: &[&str] = &["24h", "7d", "30d", "all"];

/// GET /v1/address/:addr/profile
pub async fn get_wallet_profile(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
    Query(q):     Query<WindowQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let window = q.window.as_deref().unwrap_or("7d");

    if !VALID_WINDOWS.contains(&window) {
        return Err(ApiError::BadRequest(
            format!("window must be one of: {}", VALID_WINDOWS.join(", "))
        ));
    }

    let cache_key = format!("intel:profile:{}:{}", addr, window);
    let mut redis = state.redis();

    if let Some(cached) = redis_ext::cache_get::<serde_json::Value>(&mut redis, &cache_key).await {
        return Ok(Json(cached));
    }

    let row = sqlx::query(
        r#"SELECT address, "window", updated_at, wallet_type, confidence,
                  automation_score, sniper_score, whale_score, risk_score,
                  features_json, top_programs_json, top_tokens_json, top_counterparties_json
           FROM intelligence_wallet_profiles
           WHERE address = $1 AND "window" = $2"#
    )
    .bind(&addr)
    .bind(window)
    .fetch_optional(state.pool())
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("no profile for {} window={}", addr, window)))?;

    let profile = json!({
        "address":              row.try_get::<String, _>("address").unwrap_or_default(),
        "window":               row.try_get::<String, _>("window").unwrap_or_default(),
        "updated_at":           row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").ok()
                                    .map(|t| t.to_rfc3339()),
        "wallet_type":          row.try_get::<String, _>("wallet_type").unwrap_or_default(),
        "confidence":           row.try_get::<f64, _>("confidence").unwrap_or(0.0),
        "scores": {
            "automation": row.try_get::<i32, _>("automation_score").unwrap_or(0),
            "sniper":     row.try_get::<i32, _>("sniper_score").unwrap_or(0),
            "whale":      row.try_get::<i32, _>("whale_score").unwrap_or(0),
            "risk":       row.try_get::<i32, _>("risk_score").unwrap_or(0),
        },
        "features":             row.try_get::<serde_json::Value, _>("features_json").unwrap_or_default(),
        "top_programs":         row.try_get::<serde_json::Value, _>("top_programs_json").unwrap_or_default(),
        "top_tokens":           row.try_get::<serde_json::Value, _>("top_tokens_json").unwrap_or_default(),
        "top_counterparties":   row.try_get::<serde_json::Value, _>("top_counterparties_json").unwrap_or_default(),
    });

    let _ = redis_ext::cache_set(&mut redis, &cache_key, &profile, 60).await;
    Ok(Json(profile))
}

/// GET /v1/address/:addr/scores
pub async fn get_wallet_scores(
    State(state): State<AppState>,
    Path(addr):   Path<String>,
    Query(q):     Query<WindowQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let window = q.window.as_deref().unwrap_or("7d");

    if !VALID_WINDOWS.contains(&window) {
        return Err(ApiError::BadRequest(
            format!("window must be one of: {}", VALID_WINDOWS.join(", "))
        ));
    }

    let cache_key = format!("intel:scores:{}:{}", addr, window);
    let mut redis = state.redis();

    if let Some(cached) = redis_ext::cache_get::<serde_json::Value>(&mut redis, &cache_key).await {
        return Ok(Json(cached));
    }

    let row = sqlx::query(
        r#"SELECT wallet_type, confidence, updated_at, automation_score, sniper_score, whale_score, risk_score
           FROM intelligence_wallet_profiles
           WHERE address = $1 AND "window" = $2"#
    )
    .bind(&addr)
    .bind(window)
    .fetch_optional(state.pool())
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("no profile for {}", addr)))?;

    let scores = json!({
        "address":     addr,
        "window":      window,
        "updated_at":  row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").ok()
                           .map(|t| t.to_rfc3339()),
        "wallet_type": row.try_get::<String, _>("wallet_type").unwrap_or_default(),
        "confidence":  row.try_get::<f64, _>("confidence").unwrap_or(0.0),
        "automation":  row.try_get::<i32, _>("automation_score").unwrap_or(0),
        "sniper":      row.try_get::<i32, _>("sniper_score").unwrap_or(0),
        "whale":       row.try_get::<i32, _>("whale_score").unwrap_or(0),
        "risk":        row.try_get::<i32, _>("risk_score").unwrap_or(0),
    });

    let _ = redis_ext::cache_set(&mut redis, &cache_key, &scores, 60).await;
    Ok(Json(scores))
}
