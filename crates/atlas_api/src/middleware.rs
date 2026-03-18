use axum::{
    extract::{Request, State},
    http::header::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use sqlx::Row;
use atlas_common::auth;
use crate::{state::{AppState, ApiKeyHash}, error::ApiError};

/// Validate an API key without going through the full middleware stack.
/// Used by the WebSocket handler which can't use axum middleware.
pub async fn check_api_key(state: &AppState, raw_key: &str) -> bool {
    if raw_key.is_empty() { return false; }
    let key_hash   = auth::hash_api_key(raw_key);
    let admin_hash = auth::hash_api_key(&state.cfg().admin_api_key);
    if key_hash == admin_hash { return true; }

    let rec = sqlx::query(
        "SELECT is_active FROM api_keys WHERE key_hash = $1"
    )
    .bind(&key_hash)
    .fetch_optional(state.pool())
    .await;

    matches!(rec, Ok(Some(r)) if r.try_get::<bool, _>("is_active").unwrap_or(false))
}

/// Auth middleware — validates X-API-Key, injects ApiKeyHash extension.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let raw_key = req.headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if raw_key.is_empty() {
        return Err(ApiError::Unauthorized);
    }

    let key_hash   = auth::hash_api_key(&raw_key);
    let admin_hash = auth::hash_api_key(&state.cfg().admin_api_key);

    let rate_limit: i64 = if key_hash == admin_hash {
        state.cfg().rate_limit_rpm as i64
    } else {
        let rec = sqlx::query(
            "SELECT rate_limit, is_active FROM api_keys WHERE key_hash = $1"
        )
        .bind(&key_hash)
        .fetch_optional(state.pool())
        .await?;

        match rec {
            Some(r) if r.try_get::<bool, _>("is_active").unwrap_or(false) => {
                r.try_get::<i32, _>("rate_limit").unwrap_or(300) as i64
            }
            _ => return Err(ApiError::Unauthorized),
        }
    };

    let (allowed, current_count) =
        auth::check_rate_limit_with_count(&mut state.redis(), &key_hash, rate_limit).await;

    // Update last_used_at in the background — fire and forget, non-blocking
    {
        let pool     = state.pool().clone();
        let kh_clone = key_hash.clone();
        tokio::spawn(async move {
            let _ = sqlx::query(
                "UPDATE api_keys SET last_used_at = now() WHERE key_hash = $1"
            )
            .bind(&kh_clone)
            .execute(&pool)
            .await;
        });
    }

    if !allowed {
        return Err(ApiError::RateLimited);
    }

    // Inject the key hash so handlers can scope data per API key
    req.extensions_mut().insert(ApiKeyHash(key_hash));

    let mut response = next.run(req).await;

    // Add standard rate-limit headers to every response
    let headers = response.headers_mut();
    let _ = headers.insert(
        HeaderName::from_static("x-ratelimit-limit"),
        HeaderValue::from_str(&rate_limit.to_string()).unwrap_or(HeaderValue::from_static("300")),
    );
    let remaining = (rate_limit - current_count as i64).max(0);
    let _ = headers.insert(
        HeaderName::from_static("x-ratelimit-remaining"),
        HeaderValue::from_str(&remaining.to_string()).unwrap_or(HeaderValue::from_static("0")),
    );
    let _ = headers.insert(
        HeaderName::from_static("x-ratelimit-window"),
        HeaderValue::from_static("60"),
    );

    Ok(response)
}
