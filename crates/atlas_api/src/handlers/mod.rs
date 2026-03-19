pub mod tx;
pub mod address;
pub mod webhooks;
pub mod intel;
pub mod health;
pub mod das;
pub mod sender;
pub mod wallet;
pub mod stream;
pub mod related;
pub mod pulse;
pub mod context;
pub mod mcp;
pub mod account;
pub mod token;
pub mod block;
pub mod batch;
pub mod keys;
pub mod docs;
pub mod trace;

use axum::{Router, routing::{get, post, delete}, middleware};
use tower_http::{cors::CorsLayer, trace::TraceLayer, compression::CompressionLayer};
use crate::{state::AppState, middleware::auth_middleware, rpc::json_rpc_handler};

pub fn build_router(state: AppState) -> Router {
    let public = Router::new()
        .route("/health",       get(health::health_check))
        .route("/rpc",          post(json_rpc_handler))
        .route("/mcp",          post(mcp::mcp_handler))
        .route("/mcp/sse",      get(mcp::mcp_sse_handler))
        .route("/v1/stream",    get(stream::ws_stream_handler))
        .route("/v1/network/pulse", get(pulse::network_pulse))
        .route("/v1/network/tps",   get(pulse::network_tps))
        // OpenAPI docs — public, no auth needed
        .route("/docs",         get(docs::swagger_ui))
        .route("/openapi.json", get(docs::openapi_spec));

    let authed = Router::new()
        // ── Unified wallet overview ───────────────────────────────────────
        .route("/v1/wallet/:addr",               get(account::get_wallet))
        // ── Tx ────────────────────────────────────────────────────────────
        .route("/v1/tx/:sig",                    get(tx::get_tx))
        .route("/v1/tx/:sig/enhanced",           get(tx::get_tx_enhanced))
        .route("/v1/tx/:sig/explain",            post(tx::explain_tx))
        .route("/v1/txs/batch",                  post(batch::batch_get_txs))
        // ── Address / history ─────────────────────────────────────────────
        .route("/v1/address/:addr/txs",          get(address::get_address_txs))
        .route("/v1/address/:addr/profile",      get(intel::get_wallet_profile))
        .route("/v1/address/:addr/scores",       get(intel::get_wallet_scores))
        .route("/v1/address/:addr/related",      get(related::get_related))
        // ── Wallet sub-routes ─────────────────────────────────────────────
        .route("/v1/wallet/batch-identity",      post(wallet::batch_identity))
        .route("/v1/wallet/:addr/identity",      get(wallet::get_identity))
        .route("/v1/wallet/:addr/balances",      get(wallet::get_balances))
        .route("/v1/wallet/:addr/history",       get(wallet::get_history))
        .route("/v1/wallet/:addr/transfers",     get(wallet::get_transfers))
        .route("/v1/wallet/:addr/funded-by",     get(wallet::get_funded_by))
        .route("/v1/wallet/:addr/context",       get(context::wallet_context))
        // ── Token ─────────────────────────────────────────────────────────
        .route("/v1/token/:mint",                get(token::get_token))
        .route("/v1/token/:mint/holders",        get(token::get_token_holders))
        .route("/v1/token/:mint/transfers",      get(token::get_token_transfers))
        // ── Block ─────────────────────────────────────────────────────────
        .route("/v1/block/:slot",                get(block::get_block))
        // ── Webhooks ──────────────────────────────────────────────────────
        .route("/v1/webhooks/subscribe",         post(webhooks::create_subscription))
        .route("/v1/webhooks/subscriptions",     get(webhooks::list_subscriptions))
        .route("/v1/webhooks/subscriptions/:id", delete(webhooks::delete_subscription))
        // ── API key management (admin) ────────────────────────────────────
        .route("/v1/keys",                       post(keys::create_key).get(keys::list_keys))
        .route("/v1/keys/:id",                   delete(keys::revoke_key))
        // ── Trace / counterparty graph ────────────────────────────────────
        .route("/v1/trace/:addr",                get(trace::get_trace))
        // ── Transaction sender ────────────────────────────────────────────
        .route("/v1/tx/send",                    post(sender::send_transaction))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    Router::new()
        .merge(public)
        .merge(authed)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
