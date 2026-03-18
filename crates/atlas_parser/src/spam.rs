use std::collections::HashSet;
use anyhow::Result;
use atlas_types::facts::TxFactsV1;

pub struct SpamConfig {
    pub token_denylist:   HashSet<String>,
    pub program_denylist: HashSet<String>,
}

impl SpamConfig {
    pub fn empty() -> Self {
        Self {
            token_denylist:   HashSet::new(),
            program_denylist: HashSet::new(),
        }
    }

    /// Load spam denylist from YAML.
    /// Expected format:
    ///   tokens:   ["mint1", "mint2"]
    ///   programs: ["prog1"]
    pub fn from_yaml(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let doc: serde_yaml::Value = serde_yaml::from_str(&content)?;

        let tokens: HashSet<String> = doc["tokens"].as_sequence()
            .map(|seq| seq.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let programs: HashSet<String> = doc["programs"].as_sequence()
            .map(|seq| seq.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(Self {
            token_denylist:   tokens,
            program_denylist: programs,
        })
    }
}

/// Tag tx as spam if it touches any denylisted mint or program.
pub fn apply_spam_tags(facts: &mut TxFactsV1, spam: &SpamConfig) {
    let is_spam = facts.token_deltas.iter().any(|d| spam.token_denylist.contains(&d.mint))
        || facts.programs.iter().any(|p| spam.program_denylist.contains(p));

    if is_spam && !facts.tags.iter().any(|t| t == "spam") {
        facts.tags.push("spam".into());
        facts.tags.sort();
    }
}
