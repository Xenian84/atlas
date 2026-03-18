use redis::{aio::ConnectionManager, AsyncCommands};
use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};

/// Get a JSON-deserialized value from Redis cache.
pub async fn cache_get<T: DeserializeOwned>(
    conn: &mut ConnectionManager,
    key: &str,
) -> Option<T> {
    let raw: redis::RedisResult<String> = conn.get(key).await;
    match raw {
        Ok(s) => serde_json::from_str(&s).ok(),
        Err(_) => None,
    }
}

/// Set a JSON-serialized value in Redis cache with TTL.
pub async fn cache_set<T: Serialize>(
    conn: &mut ConnectionManager,
    key: &str,
    value: &T,
    ttl_secs: usize,
) -> Result<()> {
    let s = serde_json::to_string(value)?;
    conn.set_ex::<_, _, ()>(key, s, ttl_secs as u64).await?;
    Ok(())
}

/// Publish a JSON payload to a Redis channel.
pub async fn publish_json<T: Serialize>(
    conn: &mut ConnectionManager,
    channel: &str,
    payload: &T,
) -> Result<()> {
    let s = serde_json::to_string(payload)?;
    conn.publish::<_, _, ()>(channel, s).await?;
    Ok(())
}

/// Add entry to a Redis stream (XADD).
pub async fn xadd_json<T: Serialize>(
    conn: &mut ConnectionManager,
    stream: &str,
    payload: &T,
) -> Result<()> {
    let s = serde_json::to_string(payload)?;
    let _: redis::RedisResult<String> = redis::cmd("XADD")
        .arg(stream)
        .arg("MAXLEN").arg("~").arg(10_000usize)
        .arg("*")
        .arg("data").arg(s)
        .query_async(conn)
        .await;
    Ok(())
}
