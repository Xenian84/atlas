use anyhow::Result;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde_json::Value;

type HmacSha256 = Hmac<Sha256>;

pub const SIGNATURE_HEADER: &str = "X-Atlas-Signature";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_body_produces_sha256_prefix() {
        let sig = sign_body("mysecret", "hello");
        assert!(sig.starts_with("sha256="), "expected sha256= prefix, got: {}", sig);
        assert_eq!(sig.len(), 7 + 64, "HMAC-SHA256 hex is 64 chars");
    }

    #[test]
    fn sign_body_is_deterministic() {
        let a = sign_body("key", "payload");
        let b = sign_body("key", "payload");
        assert_eq!(a, b);
    }

    #[test]
    fn sign_body_differs_with_different_keys() {
        let a = sign_body("key1", "payload");
        let b = sign_body("key2", "payload");
        assert_ne!(a, b);
    }

    #[test]
    fn next_delay_secs_schedule() {
        assert_eq!(next_delay_secs(0), 0);
        assert_eq!(next_delay_secs(1), 5);
        assert_eq!(next_delay_secs(2), 20);
        assert_eq!(next_delay_secs(3), 60);
        assert_eq!(next_delay_secs(4), 300);
        // High attempt count should floor at 3600, not 0
        assert_eq!(next_delay_secs(99), 3600);
    }
}

/// Sign a webhook body with HMAC-SHA256 using the subscription secret.
pub fn sign_body(secret: &str, body: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key size");
    mac.update(body.as_bytes());
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

/// Exponential backoff schedule (seconds until next attempt).
pub fn next_delay_secs(attempt: i32) -> i64 {
    match attempt {
        0 => 0,
        1 => 5,
        2 => 20,
        3 => 60,
        4 => 300,
        _ => 3600, // floor at 1h for excessive retries
    }
}

/// Send one delivery attempt.
/// Returns Ok((true, "")) on 2xx success, Ok((false, error_msg)) on failure.
pub async fn send_delivery(
    client:      &reqwest::Client,
    url:         &str,
    secret:      &str,
    payload:     &Value,
    delivery_id: i64,
    event_type:  &str,
) -> Result<(bool, String)> {
    let body = serde_json::to_string(payload)?;
    let sig  = sign_body(secret, &body);

    let resp = client
        .post(url)
        .header("Content-Type",        "application/json")
        .header("X-Atlas-Event",        event_type)
        .header("X-Atlas-Delivery-Id",  delivery_id.to_string())
        .header(SIGNATURE_HEADER,       &sig)
        .body(body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match resp {
        Ok(r) => {
            let status = r.status();
            if status.is_success() {
                Ok((true, String::new()))
            } else {
                // Capture first 256 bytes of the error body for diagnostics
                let body_preview = r.bytes().await
                    .map(|b| String::from_utf8_lossy(&b[..b.len().min(256)]).to_string())
                    .unwrap_or_else(|_| String::new());
                Ok((false, format!("HTTP {}: {}", status.as_u16(), body_preview)))
            }
        }
        Err(e) => {
            tracing::warn!("Delivery {} to {} network error: {}", delivery_id, url, e);
            Ok((false, e.to_string()))
        }
    }
}
