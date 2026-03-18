/// POST /v1/tx/send — submit a signed transaction with retry + priority fee.
///
/// Accepts base64-encoded signed transaction, submits to the validator RPC
/// with exponential backoff retry for higher landing rate.
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use crate::{state::AppState, error::ApiError};

#[derive(Deserialize)]
pub struct SendTxRequest {
    /// Base64-encoded signed transaction
    pub transaction:        String,
    /// Skip preflight simulation (default: false)
    #[serde(default)]
    pub skip_preflight:     bool,
    /// Max retries before giving up (default: 3, max: 5)
    #[serde(default = "default_max_retries")]
    pub max_retries:        u8,
    /// Encoding of the transaction (default: "base64")
    #[serde(default = "default_encoding")]
    pub encoding:           String,
}

fn default_max_retries() -> u8 { 3 }
fn default_encoding()    -> String { "base64".to_string() }

#[derive(Serialize)]
pub struct SendTxResponse {
    pub signature:   Option<String>,
    pub error:       Option<String>,
    pub attempts:    u8,
    pub landed:      bool,
}

pub async fn send_transaction(
    State(state): State<AppState>,
    Json(req):    Json<SendTxRequest>,
) -> Result<Json<SendTxResponse>, ApiError> {
    let max_retries = req.max_retries.min(5);

    // Validate base64
    {
        use base64::{Engine as _, engine::general_purpose};
        general_purpose::STANDARD.decode(&req.transaction)
            .map_err(|_| ApiError::BadRequest("transaction must be valid base64".into()))?;
    }

    let rpc_url = &state.cfg().validator_rpc_url;
    let http    = state.http();

    let mut attempts = 0u8;
    let mut last_error = String::new();

    // Retry schedule: immediately, +500ms, +1s, +2s, +4s
    let delays_ms = [0u64, 500, 1000, 2000, 4000];

    while attempts < max_retries + 1 {
        if attempts > 0 {
            let delay = delays_ms.get(attempts as usize).copied().unwrap_or(4000);
            sleep(Duration::from_millis(delay)).await;
            warn!("sendTransaction retry #{} after {}ms", attempts, delay);
        }

        let body = json!({
            "jsonrpc": "2.0",
            "id":      1,
            "method":  "sendTransaction",
            "params":  [
                req.transaction,
                {
                    "encoding":            req.encoding,
                    "skipPreflight":       req.skip_preflight,
                    "preflightCommitment": "processed",
                    "maxRetries":          0,   // we handle retries ourselves
                }
            ]
        });

        attempts += 1;

        let resp: Value = match http
            .post(rpc_url)
            .json(&body)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(r) => match r.json().await {
                Ok(v) => v,
                Err(e) => {
                    last_error = format!("parse error: {}", e);
                    continue;
                }
            },
            Err(e) => {
                last_error = format!("network error: {}", e);
                continue;
            }
        };

        if let Some(sig) = resp["result"].as_str() {
            info!("sendTransaction landed: {} (attempt {})", sig, attempts);
            return Ok(Json(SendTxResponse {
                signature: Some(sig.to_string()),
                error:     None,
                attempts,
                landed:    true,
            }));
        }

        // Extract RPC error
        if let Some(err) = resp.get("error") {
            let code = err["code"].as_i64().unwrap_or(-1);
            let msg  = err["message"].as_str().unwrap_or("unknown RPC error");

            // Non-retryable errors
            if code == -32002 || code == -32003 {
                // -32002: Transaction already processed
                // -32003: Transaction signature verification failure
                return Ok(Json(SendTxResponse {
                    signature: None,
                    error:     Some(msg.to_string()),
                    attempts,
                    landed:    false,
                }));
            }
            last_error = format!("RPC error {}: {}", code, msg);
        }
    }

    Ok(Json(SendTxResponse {
        signature: None,
        error:     Some(last_error),
        attempts,
        landed:    false,
    }))
}
