use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::facts::TxSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id:          Uuid,
    pub created_at:  DateTime<Utc>,
    pub is_active:   bool,
    pub event_type:  EventType,
    pub address:     Option<String>,
    pub owner:       Option<String>,
    pub program_id:  Option<String>,
    pub url:         String,
    #[serde(skip_serializing)]
    pub secret:      String,
    pub min_conf:    String,
    pub format:      WebhookFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    AddressActivity,
    TokenBalanceChanged,
    ProgramActivity,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::AddressActivity      => "address_activity",
            EventType::TokenBalanceChanged  => "token_balance_changed",
            EventType::ProgramActivity      => "program_activity",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WebhookFormat {
    #[default]
    Json,
    Toon,
}

/// Outbound webhook payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub v:       String,  // "webhook.v1"
    pub chain:   String,  // "x1"
    pub event:   String,
    pub cursor:  String,
    pub address: Option<String>,
    pub owner:   Option<String>,
    pub tx:      TxSummary,
}

impl WebhookPayload {
    pub fn new(event: impl Into<String>, cursor: impl Into<String>, tx: TxSummary) -> Self {
        Self {
            v: "webhook.v1".to_string(),
            chain: "x1".to_string(),
            event: event.into(),
            cursor: cursor.into(),
            address: None,
            owner: None,
            tx,
        }
    }
}
