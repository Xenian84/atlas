use std::collections::HashMap;
use serde::Deserialize;
use anyhow::Result;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProgramsConfig {
    pub core:  HashMap<String, String>,
    pub token: HashMap<String, String>,
    pub x1:    HashMap<String, String>,
    pub dex:   HashMap<String, String>,
}

impl ProgramsConfig {
    pub fn from_yaml(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let cfg: Self = serde_yaml::from_str(&content)?;
        Ok(cfg)
    }

    /// Check if a program ID belongs to a known DEX.
    pub fn is_dex(&self, program_id: &str) -> bool {
        self.dex.values().any(|id| id == program_id)
            || self.x1.get("dex").map(|id| id == program_id).unwrap_or(false)
    }

    /// Check if a program ID is the SPL Token program.
    pub fn is_token_program(&self, program_id: &str) -> bool {
        self.token.get("spl_token").map(|id| id == program_id).unwrap_or(false)
            || self.token.get("spl_token_2022").map(|id| id == program_id).unwrap_or(false)
    }

    pub fn is_system_program(&self, program_id: &str) -> bool {
        self.core.get("system").map(|id| id == program_id).unwrap_or(false)
    }

    pub fn is_stake_program(&self, program_id: &str) -> bool {
        self.core.get("stake").map(|id| id == program_id).unwrap_or(false)
    }

    pub fn is_bpf_upgradeable(&self, program_id: &str) -> bool {
        self.core.get("bpf_upgradeable").map(|id| id == program_id).unwrap_or(false)
    }
}
