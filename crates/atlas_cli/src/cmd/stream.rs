//! atlas stream — tail live shred events from Redis.

use anyhow::Result;
use chrono::{DateTime, Utc};
use redis::Commands;

pub async fn run(redis_url: &str, count: usize, watch: bool) -> Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_connection()
        .map_err(|e| anyhow::anyhow!("Redis connection failed: {e}\nIs Redis running? Check with: systemctl status redis"))?;

    if watch {
        println!("Watching atlas:shreds live stream (Ctrl-C to stop)");
        println!("{}", "─".repeat(72));
        watch_stream(&mut conn)?;
    } else {
        println!("Last {count} events from atlas:shreds");
        println!("{}", "─".repeat(72));
        tail_stream(&mut conn, count)?;
    }

    Ok(())
}

/// Parse raw Redis bulk reply for XREVRANGE into (id, data_json) pairs.
fn parse_xrange(val: redis::Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let entries = match val {
        redis::Value::Bulk(v) => v,
        _ => return out,
    };
    for entry in entries {
        let parts = match entry {
            redis::Value::Bulk(v) => v,
            _ => continue,
        };
        if parts.len() < 2 { continue; }
        let id = match &parts[0] {
            redis::Value::Data(b) => String::from_utf8_lossy(b).to_string(),
            _ => continue,
        };
        // fields are a flat Bulk list: [key, val, key, val, ...]
        let fields = match &parts[1] {
            redis::Value::Bulk(v) => v,
            _ => continue,
        };
        let mut data = String::new();
        let mut i = 0;
        while i + 1 < fields.len() {
            let k = match &fields[i]   { redis::Value::Data(b) => String::from_utf8_lossy(b).to_string(), _ => { i += 2; continue; } };
            let v = match &fields[i+1] { redis::Value::Data(b) => String::from_utf8_lossy(b).to_string(), _ => { i += 2; continue; } };
            if k == "data" { data = v; }
            i += 2;
        }
        if !data.is_empty() { out.push((id, data)); }
    }
    out
}

fn tail_stream(conn: &mut redis::Connection, count: usize) -> Result<()> {
    let raw: redis::Value = redis::cmd("XREVRANGE")
        .arg("atlas:shreds")
        .arg("+")
        .arg("-")
        .arg("COUNT")
        .arg(count)
        .query(conn)?;

    let entries = parse_xrange(raw);

    if entries.is_empty() {
        println!("  (no events yet — is atlas-shredstream running?)");
        return Ok(());
    }

    for (id, data_str) in entries.iter().rev() {
        print_event(id, data_str);
    }

    println!("{}", "─".repeat(72));
    let len: i64 = conn.xlen("atlas:shreds").unwrap_or(0);
    println!("  stream length: {len} total events");

    Ok(())
}

fn watch_stream(conn: &mut redis::Connection) -> Result<()> {
    // Anchor to the latest existing entry
    let raw: redis::Value = redis::cmd("XREVRANGE")
        .arg("atlas:shreds")
        .arg("+")
        .arg("-")
        .arg("COUNT")
        .arg(1)
        .query(conn)?;

    let mut last_id = parse_xrange(raw)
        .into_iter()
        .next()
        .map(|(id, _)| id)
        .unwrap_or_else(|| "0".to_string());

    loop {
        // XREAD BLOCK 2000ms — returns Nil on timeout, Bulk on data
        let raw: redis::Value = redis::cmd("XREAD")
            .arg("COUNT").arg(20)
            .arg("BLOCK").arg(2000)
            .arg("STREAMS")
            .arg("atlas:shreds")
            .arg(&last_id)
            .query(conn)
            .unwrap_or(redis::Value::Nil);

        // XREAD wraps in one more level: [[stream_name, [entries...]]]
        if let redis::Value::Bulk(streams) = raw {
            for stream in streams {
                if let redis::Value::Bulk(mut parts) = stream {
                    if parts.len() < 2 { continue; }
                    let entries_val = parts.remove(1);
                    let entries = parse_xrange(
                        redis::Value::Bulk(match entries_val {
                            redis::Value::Bulk(v) => v,
                            _ => continue,
                        })
                    );
                    for (id, data_str) in entries {
                        print_event(&id, &data_str);
                        last_id = id;
                    }
                }
            }
        }
    }
}

fn print_event(id: &str, data_str: &str) {
    let ts = id.split('-').next()
        .and_then(|ms| ms.parse::<i64>().ok())
        .map(|ms| {
            DateTime::<Utc>::from_timestamp(ms / 1000, ((ms % 1000) * 1_000_000) as u32)
                .map(|t| t.format("%H:%M:%S%.3f").to_string())
                .unwrap_or_else(|| ms.to_string())
        })
        .unwrap_or_else(|| id.to_string());

    let v: serde_json::Value = serde_json::from_str(data_str).unwrap_or_default();

    let sig      = v["sig"].as_str().unwrap_or("?");
    let slot     = v["slot"].as_u64().unwrap_or(0);
    let programs = v["programs"].as_array()
        .map(|arr| arr.iter()
            .filter_map(|p| p.as_str())
            .map(|p| if p.len() > 8 { &p[..8] } else { p })
            .collect::<Vec<_>>()
            .join(","))
        .unwrap_or_default();
    let latency  = v["latency_us"].as_i64().unwrap_or(0);
    let commitment = v["commitment"].as_str().unwrap_or("shred");

    let sig_short = if sig.len() > 16 { &sig[..16] } else { sig };

    println!("{ts}  slot:{slot:<10} [{commitment:<6}]  {sig_short}…  progs:[{programs}]  lat:{latency}µs");
}
