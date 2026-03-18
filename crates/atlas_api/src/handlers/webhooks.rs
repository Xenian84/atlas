use axum::{
    extract::{State, Path, Json as AxumJson, Extension},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use crate::{state::{AppState, ApiKeyHash}, error::ApiError};

#[derive(Deserialize)]
pub struct CreateSubscriptionReq {
    pub event_type: String,
    pub address:    Option<String>,
    pub owner:      Option<String>,
    pub program_id: Option<String>,
    pub url:        String,
    pub secret:     String,
    pub min_conf:   Option<String>,
    pub format:     Option<String>,
}

/// POST /v1/webhooks/subscribe
pub async fn create_subscription(
    State(state): State<AppState>,
    Extension(ApiKeyHash(key_hash)): Extension<ApiKeyHash>,
    AxumJson(req): AxumJson<CreateSubscriptionReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !["address_activity", "token_balance_changed", "program_activity"]
        .contains(&req.event_type.as_str())
    {
        return Err(ApiError::BadRequest(
            "event_type must be address_activity|token_balance_changed|program_activity".into()
        ));
    }

    if !req.url.starts_with("https://") {
        return Err(ApiError::BadRequest("url must start with https://".into()));
    }

    let row = sqlx::query(
        r#"INSERT INTO webhook_subscriptions
               (event_type, address, owner, program_id, url, secret, min_conf, format, api_key_hash)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING id"#
    )
    .bind(&req.event_type)
    .bind(&req.address)
    .bind(&req.owner)
    .bind(&req.program_id)
    .bind(&req.url)
    .bind(&req.secret)
    .bind(req.min_conf.as_deref().unwrap_or("confirmed"))
    .bind(req.format.as_deref().unwrap_or("json"))
    .bind(&key_hash)
    .fetch_one(state.pool())
    .await?;

    let id: uuid::Uuid = row.try_get("id")?;
    Ok(Json(json!({ "id": id, "status": "created" })))
}

/// GET /v1/webhooks/subscriptions
pub async fn list_subscriptions(
    State(state): State<AppState>,
    Extension(ApiKeyHash(key_hash)): Extension<ApiKeyHash>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT id, event_type, address, owner, program_id, url, min_conf, format, is_active, created_at
           FROM webhook_subscriptions
           WHERE api_key_hash = $1
           ORDER BY created_at DESC LIMIT 100"#
    )
    .bind(&key_hash)
    .fetch_all(state.pool())
    .await?;

    let subs: Vec<serde_json::Value> = rows.iter().map(|r| json!({
        "id":         r.try_get::<uuid::Uuid, _>("id").ok(),
        "event_type": r.try_get::<String, _>("event_type").unwrap_or_default(),
        "address":    r.try_get::<Option<String>, _>("address").unwrap_or_default(),
        "owner":      r.try_get::<Option<String>, _>("owner").unwrap_or_default(),
        "program_id": r.try_get::<Option<String>, _>("program_id").unwrap_or_default(),
        "url":        r.try_get::<String, _>("url").unwrap_or_default(),
        "is_active":  r.try_get::<bool, _>("is_active").unwrap_or(false),
        "created_at": r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok()
            .map(|t| t.to_rfc3339()),
    })).collect();

    let count = subs.len();
    Ok(Json(json!({ "subscriptions": subs, "count": count })))
}

/// DELETE /v1/webhooks/subscriptions/:id
pub async fn delete_subscription(
    State(state): State<AppState>,
    Extension(ApiKeyHash(key_hash)): Extension<ApiKeyHash>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE webhook_subscriptions SET is_active = false WHERE id = $1 AND api_key_hash = $2"
    )
    .bind(id)
    .bind(&key_hash)
    .execute(state.pool())
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("subscription not found".into()));
    }

    Ok(Json(json!({ "id": id, "status": "deactivated" })))
}
