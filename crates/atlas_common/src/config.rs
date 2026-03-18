use serde::Deserialize;
use anyhow::Result;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    // Chain / Validator (Server A)
    pub validator_rpc_url:        String,
    pub yellowstone_grpc_endpoint: String,
    pub yellowstone_grpc_x_token: Option<String>,

    // Storage (Server B)
    pub database_url:  String,
    pub redis_url:     String,

    // API
    pub api_bind:      String,
    pub admin_api_key: String,
    pub rate_limit_rpm: u32,

    // Indexer
    pub indexer_commitment:    String,
    pub indexer_metrics_bind:  String,
    /// Enable dual-commitment stream: processed fast-path + confirmed upgrade.
    /// When true, atlas:processed stream gets events ~200-400ms earlier.
    pub indexer_dual_stream:   bool,

    // Webhooks
    pub webhook_worker_concurrency: usize,
    pub webhook_max_attempts:       u32,

    // Intelligence
    pub intel_recompute_cooldown_secs: u64,
    pub intel_windows: String,   // comma-separated: "24h,7d,30d"

    // LLM provider (optional — explain falls back to template if unset)
    // Supported: openai | anthropic | ollama
    pub llm_provider:  Option<String>,
    pub llm_base_url:  Option<String>,   // e.g. http://localhost:11434/v1
    pub llm_api_key:   Option<String>,
    pub llm_model:     Option<String>,   // e.g. llama3.2 | gpt-4o | claude-3-5-sonnet-20241022

    // Price oracle — token USD pricing for /wallet/:addr/balances
    // Set to your XDex price API once available on X1.
    // Expected: GET {price_api_url}?ids=MINT1,MINT2  → JSON {data:{MINT:{price:"1.23"}}}
    // Set to empty string to disable pricing entirely.
    pub price_api_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let cfg = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .set_default("api_bind",                     "0.0.0.0:8080")?
            .set_default("indexer_commitment",            "confirmed")?
            .set_default("indexer_metrics_bind",          "0.0.0.0:9100")?
            .set_default("indexer_dual_stream",           false)?
            .set_default("rate_limit_rpm",                300i64)?
            .set_default("webhook_worker_concurrency",    50i64)?
            .set_default("webhook_max_attempts",          5i64)?
            .set_default("intel_recompute_cooldown_secs", 60i64)?
            .set_default("intel_windows",                 "24h,7d,30d")?
            .set_default("llm_provider",                  "none")?
            .set_default("llm_base_url",                  "http://localhost:11434/v1")?
            .set_default("llm_model",                     "llama3.2")?
            // XDex token price oracle for X1 Mainnet
            // GET {price_api_url}?network=X1+Mainnet&token_address=MINT
            // → {"success":true,"data":{"price":1.23,"price_currency":"USD"}}
            .set_default("price_api_url",                 "https://api.xdex.xyz/api/token-price/price")?
            .build()?;

        Ok(cfg.try_deserialize()?)
    }

    pub fn intel_window_list(&self) -> Vec<&str> {
        self.intel_windows.split(',').map(str::trim).collect()
    }
}
