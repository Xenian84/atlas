//! Unix socket receiver — reads length-prefixed bincode AtlasEntryFrames from
//! the tachyon validator bridge and publishes events to Redis atlas:shreds.

use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tracing::{info, warn, error};

use crate::frame::{AtlasEntryFrame, WIRE_VERSION};

pub async fn run_receiver(
    socket_path:  String,
    redis:        ConnectionManager,
    redis_stream: String,
) -> Result<()> {
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    info!("Listening on Unix socket {}", socket_path);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let r  = redis.clone();
                let rs = redis_stream.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, r, rs).await {
                        error!("Bridge connection error: {:#}", e);
                    }
                });
            }
            Err(e) => warn!("Accept error: {}", e),
        }
    }
}

async fn handle_connection(
    mut stream:   tokio::net::UnixStream,
    mut redis:    ConnectionManager,
    redis_stream: String,
) -> Result<()> {
    info!("Validator bridge connected");
    let recv_start = std::time::Instant::now();
    let mut frames_received: u64 = 0;
    let mut txs_published:   u64 = 0;

    loop {
        // ── Read 4-byte little-endian length prefix ──────────────────────────
        let mut len_buf = [0u8; 4];
        match stream.read_exact(&mut len_buf).await {
            Ok(_)  => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                info!("Validator bridge disconnected after {} frames ({} txs) in {:?}",
                    frames_received, txs_published, recv_start.elapsed());
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }

        let frame_len = u32::from_le_bytes(len_buf) as usize;

        // Safety cap: reject frames > 8MB
        if frame_len > 8 * 1024 * 1024 {
            return Err(anyhow::anyhow!(
                "Frame too large ({} bytes) — possible desync, closing connection", frame_len
            ));
        }

        // ── Read payload ──────────────────────────────────────────────────────
        let mut payload = vec![0u8; frame_len];
        stream.read_exact(&mut payload).await?;

        // ── Deserialize ───────────────────────────────────────────────────────
        let frame: AtlasEntryFrame = match bincode::deserialize(&payload) {
            Ok(f)  => f,
            Err(e) => {
                warn!("Bincode deserialize failed ({} bytes): {}", frame_len, e);
                continue;
            }
        };

        // Version check
        if frame.version != WIRE_VERSION {
            warn!("Wire version mismatch: got {} expected {} — skipping frame (upgrade atlas-shredstream)",
                frame.version, WIRE_VERSION);
            continue;
        }

        frames_received += 1;
        let latency_us = micros_now() - frame.ts_us;

        // ── Publish each transaction to Redis ─────────────────────────────────
        for tx in &frame.txs {
            let sig      = tx.sig_b58();
            let accounts = tx.accounts_b58();
            let programs = tx.programs_b58();

            let event = serde_json::json!({
                "sig":        sig,
                "slot":       frame.slot,
                "entry_idx":  frame.entry_idx,
                "num_hashes": frame.num_hashes,
                "commitment": "shred",
                "accounts":   accounts,
                "programs":   programs,
                "raw_tx_b64": base64_encode(&tx.raw_tx),
                "ts_us":      frame.ts_us,
                "latency_us": latency_us,
            });

            let data = event.to_string();
            let result: redis::RedisResult<String> = redis.xadd(
                &redis_stream,
                "*",
                &[("data", data.as_str())],
            ).await;

            match result {
                Ok(_)  => txs_published += 1,
                Err(e) => warn!("XADD {} failed for {}: {}", redis_stream, sig, e),
            }
        }
    }
}

fn micros_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

fn base64_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    const ALPHABET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    let mut i = 0;
    while i + 2 < bytes.len() {
        let (b0, b1, b2) = (bytes[i] as usize, bytes[i+1] as usize, bytes[i+2] as usize);
        let _ = write!(out, "{}{}{}{}",
            ALPHABET[b0>>2] as char,
            ALPHABET[((b0&3)<<4)|(b1>>4)] as char,
            ALPHABET[((b1&15)<<2)|(b2>>6)] as char,
            ALPHABET[b2&63] as char);
        i += 3;
    }
    match bytes.len() - i {
        1 => { let b0 = bytes[i] as usize;
               let _ = write!(out, "{}{}==", ALPHABET[b0>>2] as char, ALPHABET[(b0&3)<<4] as char); }
        2 => { let (b0, b1) = (bytes[i] as usize, bytes[i+1] as usize);
               let _ = write!(out, "{}{}{}=",
                   ALPHABET[b0>>2] as char,
                   ALPHABET[((b0&3)<<4)|(b1>>4)] as char,
                   ALPHABET[(b1&15)<<2] as char); }
        _ => {}
    }
    out
}
