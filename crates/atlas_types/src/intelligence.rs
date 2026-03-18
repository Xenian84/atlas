use serde::{Deserialize, Serialize};
use serde_json::Value;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletProfile {
    pub address:          String,
    pub window:           String,           // 24h|7d|30d|all
    pub updated_at:       DateTime<Utc>,
    pub wallet_type:      WalletType,
    pub confidence:       f64,              // 0.0–1.0
    pub scores:           WalletScores,
    pub features:         WalletFeatures,
    pub top_programs:     Vec<ProgramUsage>,
    pub top_tokens:       Vec<TokenUsage>,
    pub top_counterparties: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletType {
    Human,
    Bot,
    Sniper,
    Whale,
    ExchangeLike,
    Developer,
    Unknown,
}

impl WalletType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WalletType::Human        => "human",
            WalletType::Bot          => "bot",
            WalletType::Sniper       => "sniper",
            WalletType::Whale        => "whale",
            WalletType::ExchangeLike => "exchange_like",
            WalletType::Developer    => "developer",
            WalletType::Unknown      => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WalletScores {
    pub automation: u8,   // 0–100
    pub sniper:     u8,
    pub whale:      u8,
    pub risk:       u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WalletFeatures {
    pub tx_count:              u64,
    pub active_days:           u32,
    pub burstiness:            u32,    // max tx in any 10-min bucket
    pub unique_programs:       u32,
    pub unique_tokens:         u32,
    pub unique_counterparties: u32,
    pub failure_rate:          f64,    // 0.0–1.0
    pub swap_count:            u64,
    pub transfer_count:        u64,
    pub mint_count:            u64,
    pub burn_count:            u64,
    pub avg_fee_lamports:      u64,
    pub avg_priority_fee:      Option<u64>,
    #[serde(rename = "net_xnt_delta")]
    pub net_sol_delta:         i64,    // lamports, may be negative
    pub has_deploy_actions:    bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramUsage {
    pub program_id: String,
    pub call_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub mint:       String,
    pub abs_delta:  String,    // string for large numbers
    pub symbol:     Option<String>,
}

/// Edge in wallet relationship graph (v2 clustering)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletEdge {
    pub src:        String,
    pub dst:        String,
    pub reason:     String,
    pub weight:     f64,
}
