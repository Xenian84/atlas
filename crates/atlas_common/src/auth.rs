use sha2::{Sha256, Digest};
use hex;

/// Hash an API key for storage/comparison.
/// We store the SHA-256 hex digest in the DB, never the raw key.
pub fn hash_api_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Validate that a raw API key matches a stored hash.
pub fn verify_api_key(raw_key: &str, stored_hash: &str) -> bool {
    hash_api_key(raw_key) == stored_hash
}

/// Redis token bucket rate limiter key for a given API key hash.
pub fn rate_limit_key(key_hash: &str) -> String {
    format!("rl:{}", key_hash)
}

/// Check and decrement rate limit bucket.
/// Returns (allowed, current_count).
pub async fn check_rate_limit_with_count(
    conn: &mut redis::aio::ConnectionManager,
    key_hash: &str,
    limit_rpm: i64,
) -> (bool, i64) {
    let redis_key = rate_limit_key(key_hash);
    let pipe_result: redis::RedisResult<(i64, i64)> = redis::pipe()
        .atomic()
        .incr(&redis_key, 1_i64)
        .expire(&redis_key, 60_i64)
        .query_async(conn)
        .await;

    match pipe_result {
        Ok((count, _)) => (count <= limit_rpm, count),
        Err(_) => (true, 0), // fail open on Redis errors
    }
}

/// Check and decrement rate limit bucket (compat wrapper).
pub async fn check_rate_limit(
    conn: &mut redis::aio::ConnectionManager,
    key_hash: &str,
    limit_rpm: i64,
) -> bool {
    check_rate_limit_with_count(conn, key_hash, limit_rpm).await.0
}
