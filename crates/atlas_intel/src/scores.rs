use atlas_types::intelligence::{WalletFeatures, WalletScores, WalletType};

pub struct ComputedScores {
    pub scores:      WalletScores,
    pub wallet_type: WalletType,
    pub confidence:  f64,
}

/// Deterministic scoring from features.
/// All rules are integer arithmetic — no floating point randomness.
pub fn compute(f: &WalletFeatures) -> ComputedScores {
    let automation = compute_automation(f);
    let sniper     = compute_sniper(f);
    let whale      = compute_whale(f);
    let risk       = compute_risk(f);

    let wallet_type = classify(automation, sniper, whale, f);
    let confidence  = compute_confidence(f);

    ComputedScores {
        scores: WalletScores { automation, sniper, whale, risk },
        wallet_type,
        confidence,
    }
}

fn compute_automation(f: &WalletFeatures) -> u8 {
    let mut score: i32 = 0;

    // High burstiness (>10 tx in 10-min bucket)
    if f.burstiness > 10 { score += 30; }
    else if f.burstiness > 5 { score += 15; }

    // Very high tx/day ratio
    let tx_per_day = if f.active_days > 0 { f.tx_count / f.active_days as u64 } else { 0 };
    if tx_per_day > 100 { score += 20; }
    else if tx_per_day > 30 { score += 10; }

    // Always uses priority fee
    if f.avg_priority_fee.unwrap_or(0) > 0 { score += 10; }

    // Very many unique programs (scripted behavior)
    if f.unique_programs > 20 { score += 20; }
    else if f.unique_programs > 10 { score += 10; }

    score.min(100) as u8
}

fn compute_sniper(f: &WalletFeatures) -> u8 {
    let mut score: i32 = 0;

    // High swap count relative to tx count
    if f.tx_count > 0 {
        let swap_ratio = f.swap_count as f64 / f.tx_count as f64;
        if swap_ratio > 0.8 { score += 40; }
        else if swap_ratio > 0.5 { score += 20; }
    }

    // Many different tokens traded
    if f.unique_tokens > 50 { score += 20; }
    else if f.unique_tokens > 20 { score += 10; }

    // High burstiness + high swaps
    if f.burstiness > 5 && f.swap_count > 10 { score += 20; }

    score.min(100) as u8
}

fn compute_whale(f: &WalletFeatures) -> u8 {
    let mut score: i32 = 0;

    // Large positive net XNT flow (received many lamports)
    let xnt = f.net_sol_delta.abs();
    if xnt > 1_000_000_000_000 { score += 60; }  // >1000 XNT
    else if xnt > 100_000_000_000 { score += 40; } // >100 XNT
    else if xnt > 10_000_000_000  { score += 20; } // >10 XNT

    // High unique tokens/programs = institutional-like
    if f.unique_programs > 15 && f.unique_tokens > 30 { score += 20; }

    score.min(100) as u8
}

fn compute_risk(f: &WalletFeatures) -> u8 {
    let mut score: i32 = 0;

    // High failure rate
    if f.failure_rate > 0.5 { score += 30; }
    else if f.failure_rate > 0.2 { score += 15; }

    // Excessive burstiness (possible spam)
    if f.burstiness > 50 { score += 20; }

    score.min(100) as u8
}

fn classify(automation: u8, sniper: u8, whale: u8, f: &WalletFeatures) -> WalletType {
    if sniper > 80 {
        WalletType::Sniper
    } else if automation > 80 {
        WalletType::Bot
    } else if whale > 80 {
        WalletType::Whale
    } else if f.has_deploy_actions {
        WalletType::Developer
    } else if f.tx_count > 0 {
        WalletType::Human
    } else {
        WalletType::Unknown
    }
}

fn compute_confidence(f: &WalletFeatures) -> f64 {
    // Confidence increases with more data points
    let base: f64 = match f.tx_count {
        0    => 0.0,
        1..=9  => 0.3,
        10..=49 => 0.6,
        50..=199 => 0.8,
        _    => 0.95,
    };
    base.min(1.0)
}
