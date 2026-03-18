//! Background task that tails atlas:newtx Redis stream and fans events out to
//! an in-process tokio broadcast channel. WebSocket handlers subscribe to this
//! channel and filter by address / program / event type.

use std::sync::Arc;
use anyhow::Result;
use redis::aio::ConnectionManager;
use serde_json::Value;
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

pub type TxEvent = Arc<Value>;

/// Capacity of the in-process broadcast ring-buffer.
/// Slow WebSocket consumers will lag but won't block indexing.
const BROADCAST_CAPACITY: usize = 1024;

pub fn create_broadcast() -> broadcast::Sender<TxEvent> {
    broadcast::channel(BROADCAST_CAPACITY).0
}

/// Spawns a background task that performs `XREAD COUNT 100 BLOCK 1000 STREAMS atlas:newtx $`
/// indefinitely and puts each event into the broadcast channel.
/// Call once at startup — the task runs until the process exits.
pub fn start_reader(redis: ConnectionManager, tx: broadcast::Sender<TxEvent>) {
    tokio::spawn(async move {
        if let Err(e) = read_loop(redis, tx).await {
            error!(err = %e, "broadcast reader exited with error");
        }
    });
}

async fn read_loop(mut conn: ConnectionManager, tx: broadcast::Sender<TxEvent>) -> Result<()> {
    // Start from the latest entry — we don't replay history for live stream consumers.
    let mut last_id = "$".to_string();

    loop {
        let cmd = redis::cmd("XREAD")
            .arg("COUNT").arg(100u64)
            .arg("BLOCK").arg(1000u64)   // 1-second blocking poll
            .arg("STREAMS").arg("atlas:newtx")
            .arg(&last_id)
            .query_async::<_, redis::Value>(&mut conn)
            .await;

        let reply = match cmd {
            Ok(r)  => r,
            Err(e) => { warn!(err = %e, "XREAD error, retrying"); continue; }
        };

        // XREAD returns: [ ["atlas:newtx", [ [id, [field, val, ...]], ... ]] ]
        let entries = match parse_xread_reply(reply) {
            Some(e) => e,
            None    => continue,  // nil reply (timeout with no new messages)
        };

        for (id, payload) in entries {
            last_id = id;
            // Broadcast to all subscribers; ignore "no receivers" error.
            let _ = tx.send(Arc::new(payload));
            debug!(id = %last_id, "broadcast sent");
        }
    }
}

/// Parse the nested XREAD reply into (entry_id, payload_value) pairs.
fn parse_xread_reply(reply: redis::Value) -> Option<Vec<(String, Value)>> {
    use redis::Value::*;

    // Top-level: Bulk([stream_name_entry])
    let streams = match reply {
        Bulk(v) if !v.is_empty() => v,
        _ => return None,
    };

    // Each stream entry: Bulk([stream_name, Bulk([entries])])
    let stream = match &streams[0] {
        Bulk(v) if v.len() >= 2 => v,
        _ => return None,
    };

    let entries = match &stream[1] {
        Bulk(v) => v,
        _ => return None,
    };

    let mut out = Vec::new();
    for entry in entries {
        let parts = match entry {
            Bulk(v) if v.len() >= 2 => v,
            _ => continue,
        };

        let id = match &parts[0] {
            Data(b) => String::from_utf8_lossy(b).to_string(),
            _ => continue,
        };

        let fields = match &parts[1] {
            Bulk(v) => v,
            _ => continue,
        };

        // Fields are [key, value, key, value, …]. Look for the "data" key.
        let mut i = 0;
        while i + 1 < fields.len() {
            if let (Data(k), Data(v)) = (&fields[i], &fields[i + 1]) {
                if k == b"data" {
                    if let Ok(json) = serde_json::from_slice::<Value>(v) {
                        out.push((id.clone(), json));
                    }
                }
            }
            i += 2;
        }
    }

    Some(out)
}
