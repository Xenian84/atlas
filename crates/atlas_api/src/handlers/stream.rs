//! GET /v1/stream — Live WebSocket stream of indexed transactions.
//!
//! ## Connection
//!
//! ```
//! GET /v1/stream?key=YOUR_API_KEY
//! Upgrade: websocket
//! ```
//!
//! ## Subscription message (JSON, sent by client after connect)
//!
//! ```json
//! {
//!   "subscribe": {
//!     "addresses":  ["WALLET1", "WALLET2"],
//!     "programs":   ["PROGRAM_ID"],
//!     "types":      ["SWAP", "TRANSFER"],
//!     "commitment": "confirmed"
//!   }
//! }
//! ```
//!
//! All filter fields are optional. Omitting all filters streams every tx.
//!
//! ## Events
//!
//! Each event is a JSON object: `{ "type": "tx", "data": { ...TxFactsV1... } }`
//!
//! ## Ping / keep-alive
//!
//! The server sends `{ "type": "ping" }` every 30 seconds.
//! Clients should respond with `{ "type": "pong" }` (optional).

use std::collections::HashSet;
use axum::{
    extract::{State, Query, WebSocketUpgrade},
    response::Response,
    extract::ws::{WebSocket, Message},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::{interval, Duration};
use tracing::{debug, warn};

use crate::state::AppState;
use crate::middleware::check_api_key;

#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    /// API key passed as query param (since WS headers are browser-restricted).
    pub key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubscribeMsg {
    subscribe: SubscribeFilter,
}

#[derive(Debug, Deserialize, Default)]
struct SubscribeFilter {
    addresses:  Option<Vec<String>>,
    programs:   Option<Vec<String>>,
    types:      Option<Vec<String>>,
    commitment: Option<String>,
}

#[derive(Serialize)]
struct StreamEvent<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<&'a Value>,
}

pub async fn ws_stream_handler(
    ws:              WebSocketUpgrade,
    State(state):    State<AppState>,
    Query(q):        Query<StreamQuery>,
) -> Response {
    // Authenticate via ?key= query param (browsers can't set custom WS headers).
    let authed = match &q.key {
        Some(k) => check_api_key(&state, k).await,
        None    => false,
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, authed))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, authed: bool) {
    if !authed {
        let msg = serde_json::to_string(&json!({
            "type": "error",
            "message": "unauthorized: provide ?key=YOUR_API_KEY"
        })).unwrap_or_default();
        let _ = socket.send(Message::Text(msg)).await;
        return;
    }

    // Subscribe to the broadcast channel before waiting for the filter message
    // so we don't miss events during the handshake window.
    let mut rx = state.tx_broadcast().subscribe();

    // Send a welcome message.
    let welcome = serde_json::to_string(&json!({
        "type": "connected",
        "message": "send a {\"subscribe\":{...}} message to set filters, or receive all events unfiltered"
    })).unwrap_or_default();
    if socket.send(Message::Text(welcome)).await.is_err() {
        return;
    }

    // Wait up to 5 s for a subscription message. If none arrives we stream everything.
    let filter = tokio::time::timeout(
        Duration::from_secs(5),
        read_subscribe_msg(&mut socket),
    ).await.unwrap_or_default();

    debug!(?filter, "WebSocket subscription established");

    // Build lookup sets for O(1) matching.
    let addr_filter: HashSet<String>  = filter.addresses.unwrap_or_default().into_iter().collect();
    let prog_filter: HashSet<String>  = filter.programs.unwrap_or_default().into_iter().collect();
    let type_filter: HashSet<String>  = filter.types.unwrap_or_default().into_iter().collect();
    let commitment_filter: Option<String> = filter.commitment;

    let mut ping_interval = interval(Duration::from_secs(30));
    ping_interval.tick().await; // consume immediate first tick

    loop {
        tokio::select! {
            // New tx event from the broadcast channel
            event = rx.recv() => {
                match event {
                    Ok(tx) => {
                        if matches_filter(&tx, &addr_filter, &prog_filter, &type_filter, &commitment_filter) {
                            let evt = serde_json::to_string(&StreamEvent {
                                kind: "tx",
                                data: Some(&tx),
                            }).unwrap_or_default();
                            if socket.send(Message::Text(evt)).await.is_err() {
                                break; // client disconnected
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(n, "WebSocket consumer lagged, {} events dropped", n);
                    }
                    Err(_) => break, // sender dropped (shutdown)
                }
            }

            // Ping keep-alive
            _ = ping_interval.tick() => {
                let ping = serde_json::to_string(&json!({"type":"ping"})).unwrap_or_default();
                if socket.send(Message::Text(ping)).await.is_err() {
                    break;
                }
            }

            // Incoming message from client (pong or re-subscribe — ignore for now)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    debug!("WebSocket client disconnected");
}

/// Read the first text message from the socket and parse it as a SubscribeMsg.
async fn read_subscribe_msg(socket: &mut WebSocket) -> SubscribeFilter {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(t) = msg {
            if let Ok(sub) = serde_json::from_str::<SubscribeMsg>(&t) {
                return sub.subscribe;
            }
        }
    }
    SubscribeFilter::default()
}

/// Returns true if the tx event passes the caller's filter.
/// Empty filter sets mean "match all".
fn matches_filter(
    tx:      &Value,
    addrs:   &HashSet<String>,
    progs:   &HashSet<String>,
    types:   &HashSet<String>,
    commit:  &Option<String>,
) -> bool {
    // commitment filter
    if let Some(c) = commit {
        if tx.get("commitment").and_then(Value::as_str) != Some(c.as_str()) {
            return false;
        }
    }

    // address filter — match against any account in the tx
    if !addrs.is_empty() {
        let accounts = tx.get("accounts").and_then(Value::as_array);
        let hit = accounts.map(|arr| arr.iter().any(|a| {
            a.get("pubkey").and_then(Value::as_str)
                .map(|p| addrs.contains(p))
                .unwrap_or(false)
        })).unwrap_or(false);
        if !hit { return false; }
    }

    // program filter
    if !progs.is_empty() {
        let programs = tx.get("programs").and_then(Value::as_array);
        let hit = programs.map(|arr| arr.iter().any(|p| {
            p.as_str().map(|s| progs.contains(s)).unwrap_or(false)
        })).unwrap_or(false);
        if !hit { return false; }
    }

    // type filter — match against any action's "t" field
    if !types.is_empty() {
        let actions = tx.get("actions").and_then(Value::as_array);
        let hit = actions.map(|arr| arr.iter().any(|a| {
            a.get("t").and_then(Value::as_str)
                .map(|t| types.contains(t))
                .unwrap_or(false)
        })).unwrap_or(false);
        if !hit { return false; }
    }

    true
}
