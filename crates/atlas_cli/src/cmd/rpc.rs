//! atlas rpc — print the Atlas RPC + WebSocket endpoint URLs.

use anyhow::Result;

pub async fn run(api_url: &str, rpc_url: &str, json: bool) -> Result<()> {
    // Derive WebSocket URL from HTTP API URL
    let ws_url = api_url
        .replacen("https://", "wss://", 1)
        .replacen("http://", "ws://", 1);

    // Derive standard Solana WS from RPC URL
    let validator_ws = rpc_url
        .replacen("https://", "wss://", 1)
        .replacen("http://", "ws://", 1)
        .replacen(":8899", ":8900", 1);

    if json {
        println!("{}", serde_json::json!({
            "atlas": {
                "rpc":            format!("{api_url}/rpc"),
                "rest":           api_url,
                "websocket":      format!("{ws_url}/v1/stream"),
                "health":         format!("{api_url}/health"),
                "docs":           format!("{api_url}/docs"),
            },
            "validator": {
                "rpc":            rpc_url,
                "websocket":      validator_ws,
            },
            "usage": format!("{api_url}/v1/usage"),
        }));
    } else {
        println!("Atlas RPC Endpoints");
        println!("{}", "─".repeat(60));
        println!("  JSON-RPC      {api_url}/rpc");
        println!("  REST API      {api_url}");
        println!("  WebSocket     {ws_url}/v1/stream");
        println!("  Health        {api_url}/health");
        println!("  Docs          {api_url}/docs");
        println!();
        println!("  Validator RPC  {rpc_url}");
        println!("  Validator WS   {validator_ws}");
        println!("{}", "─".repeat(60));
        println!();
        println!("  Usage:  atlas rpc --json   (machine-readable)");
        println!("  Docs:   https://github.com/TachyonZK/atlas");
    }

    Ok(())
}
