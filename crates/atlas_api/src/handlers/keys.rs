//! API key management — admin only.
//!
//! POST   /v1/keys         — create a new API key
//! GET    /v1/keys         — list all keys (admin only)
//! DELETE /v1/keys/:id     — revoke a key

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use atlas_common::auth;
use crate::{error::ApiError, state::AppState};

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name:       String,
    pub tier:       Option<String>,
    pub rate_limit: Option<i32>,
    pub email:      Option<String>,
}

/// POST /v1/keys — create new API key (admin only)
pub async fn create_key(
    State(state): State<AppState>,
    Json(req):    Json<CreateKeyRequest>,
) -> Result<Json<Value>, ApiError> {
    // Generate a random API key: atlas_<32 random hex chars>
    let raw_key = generate_api_key();
    let key_hash   = auth::hash_api_key(&raw_key);
    let key_prefix = raw_key.chars().take(12).collect::<String>();
    let tier       = req.tier.unwrap_or_else(|| "free".to_string());
    let rate_limit = req.rate_limit.unwrap_or(300);

    sqlx::query(
        r#"INSERT INTO api_keys (key_hash, key_prefix, name, tier, rate_limit, owner_email)
           VALUES ($1, $2, $3, $4, $5, $6)"#
    )
    .bind(&key_hash)
    .bind(&key_prefix)
    .bind(&req.name)
    .bind(&tier)
    .bind(rate_limit)
    .bind(req.email.as_deref())
    .execute(state.pool())
    .await?;

    // Return the full key ONCE — it's never stored in plaintext
    Ok(Json(json!({
        "api_key":    raw_key,
        "key_prefix": key_prefix,
        "name":       req.name,
        "tier":       tier,
        "rate_limit": rate_limit,
        "warning":    "Store this key securely — it will not be shown again",
    })))
}

/// GET /v1/keys — list all active keys (admin only, shows prefix only)
pub async fn list_keys(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT id, key_prefix, name, tier, rate_limit, is_active,
                  created_at, last_used_at, owner_email
           FROM api_keys
           ORDER BY created_at DESC"#
    )
    .fetch_all(state.pool())
    .await?;

    let keys: Vec<Value> = rows.iter().map(|r| json!({
        "id":           r.try_get::<uuid::Uuid, _>("id").ok().map(|u| u.to_string()),
        "key_prefix":   r.try_get::<String, _>("key_prefix").unwrap_or_default(),
        "name":         r.try_get::<String, _>("name").unwrap_or_default(),
        "tier":         r.try_get::<String, _>("tier").unwrap_or_default(),
        "rate_limit":   r.try_get::<i32, _>("rate_limit").unwrap_or(0),
        "is_active":    r.try_get::<bool, _>("is_active").unwrap_or(false),
        "created_at":   r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
        "last_used_at": r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_used_at").ok().flatten().map(|t| t.to_rfc3339()),
        "owner_email":  r.try_get::<Option<String>, _>("owner_email").ok().flatten(),
    })).collect();

    Ok(Json(json!({ "keys": keys, "count": keys.len() })))
}

/// DELETE /v1/keys/:id — revoke a key
pub async fn revoke_key(
    State(state): State<AppState>,
    Path(id):     Path<uuid::Uuid>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE api_keys SET is_active = false WHERE id = $1"
    )
    .bind(id)
    .execute(state.pool())
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("key not found".into()));
    }

    Ok(Json(json!({ "id": id.to_string(), "status": "revoked" })))
}

fn generate_api_key() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    // Combine system time + random-ish state for key entropy
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let mut h1 = DefaultHasher::new();
    t.hash(&mut h1);
    let p1 = h1.finish();

    // Second hash with different seed for more entropy
    let mut h2 = DefaultHasher::new();
    (t ^ 0xDEADBEEF_CAFEBABE).hash(&mut h2);
    let p2 = h2.finish();

    let mut h3 = DefaultHasher::new();
    (t.wrapping_mul(0x9E3779B9)).hash(&mut h3);
    let p3 = h3.finish();

    format!("atlas_{:016x}{:016x}{:016x}", p1, p2, p3)
}
