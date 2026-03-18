use atlas_types::facts::{TxFactsV1, TxHistoryPage, Action, TokenDelta};
use atlas_types::intelligence::WalletProfile;
use crate::table::{ToonTable, render_list};

/// Render a full TxFactsV1 as TOON.
pub fn render_txfacts(f: &TxFactsV1) -> String {
    let mut out = String::new();

    // ── tx header ──────────────────────────────────────────────────────────────
    out.push_str("tx:\n");
    out.push_str(&format!(" sig:     {}\n", f.sig));
    out.push_str(&format!(" slot:    {}\n", f.slot));
    out.push_str(&format!(" pos:     {}\n", f.pos));
    out.push_str(&format!(" time:    {}\n", f.block_time.unwrap_or(0)));
    out.push_str(&format!(" status:  {}\n", if f.is_success() { "ok" } else { "fail" }));
    out.push_str(&format!(" fee:     {}\n", f.fee_lamports));
    out.push_str(&format!(" commit:  {}\n", f.commitment.as_str()));
    if let Some(c) = f.compute_units.consumed {
        out.push_str(&format!(" compute: {}/{}\n",
            c,
            f.compute_units.limit.unwrap_or(0)
        ));
    }
    if let Some(pf) = f.compute_units.price_micro_lamports {
        out.push_str(&format!(" pfee:    {}\n", pf));
    }
    if let Some(err) = &f.err {
        out.push_str(&format!(" err:     {}\n", err));
    }
    out.push('\n');

    // ── programs ───────────────────────────────────────────────────────────────
    out.push_str(&render_list("programs", &f.programs, 0));
    out.push('\n');

    // ── tags ───────────────────────────────────────────────────────────────────
    out.push_str(&render_list("tags", &f.tags, 0));
    out.push('\n');

    // ── actions table ──────────────────────────────────────────────────────────
    out.push_str(&render_actions_table(&f.actions, 0));
    out.push('\n');

    // ── token deltas table ─────────────────────────────────────────────────────
    out.push_str(&render_token_deltas_table(&f.token_deltas, 0));
    out.push('\n');

    // ── xnt deltas table ──────────────────────────────────────────────────────
    if !f.sol_deltas.is_empty() {
        let mut tbl = ToonTable::new("xntDeltas", vec!["owner", "pre", "post", "delta"]);
        for d in &f.sol_deltas {
            tbl.add_row(vec![
                abbrev(&d.owner),
                d.pre_lamports.to_string(),
                d.post_lamports.to_string(),
                d.delta_lamports.to_string(),
            ]);
        }
        out.push_str(&tbl.render(0));
    }

    out
}

/// Render a TxHistoryPage as TOON (two tables: tx rows + action rows).
pub fn render_tx_history(page: &TxHistoryPage) -> String {
    let mut out = String::new();

    out.push_str(&format!("address: {}\n", page.address));
    out.push_str(&format!("limit:   {}\n", page.limit));
    if let Some(c) = &page.next_cursor {
        out.push_str(&format!("cursor:  {}\n", c));
    }
    out.push('\n');

    // tx summary table
    let mut tx_tbl = ToonTable::new(
        "txs",
        vec!["sig", "slot", "time", "st", "fee", "tags"],
    );
    for tx in &page.transactions {
        tx_tbl.add_row(vec![
            abbrev(&tx.signature),
            tx.slot.to_string(),
            tx.block_time.unwrap_or(0).to_string(),
            if tx.status == atlas_types::facts::TxStatus::Success { "ok".into() } else { "fail".into() },
            tx.fee_lamports.to_string(),
            tx.tags.join("|"),
        ]);
    }
    out.push_str(&tx_tbl.render(0));
    out.push('\n');

    // flatten all actions into one table with sig prefix
    let all_actions: Vec<(&str, &Action)> = page.transactions.iter()
        .flat_map(|tx| tx.actions.iter().map(move |a| (tx.signature.as_str(), a)))
        .collect();

    if !all_actions.is_empty() {
        let mut act_tbl = ToonTable::new("actions", vec!["sig", "t", "p", "s", "x", "amt"]);
        for (sig, a) in &all_actions {
            act_tbl.add_row(vec![
                abbrev(sig),
                a.t.clone(),
                a.p.clone(),
                abbrev(&a.s),
                a.x.as_deref().map(abbrev).unwrap_or_else(|| "-".into()),
                a.amt.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
            ]);
        }
        out.push_str(&act_tbl.render(0));
    }

    out
}

/// Render a WalletProfile as TOON.
pub fn render_wallet_profile(p: &WalletProfile) -> String {
    let mut out = String::new();

    out.push_str("profile:\n");
    out.push_str(&format!(" address:    {}\n", p.address));
    out.push_str(&format!(" window:     {}\n", p.window));
    out.push_str(&format!(" type:       {}\n", p.wallet_type.as_str()));
    out.push_str(&format!(" confidence: {:.2}\n", p.confidence));
    out.push('\n');

    out.push_str("scores:\n");
    out.push_str(&format!(" automation: {}\n", p.scores.automation));
    out.push_str(&format!(" sniper:     {}\n", p.scores.sniper));
    out.push_str(&format!(" whale:      {}\n", p.scores.whale));
    out.push_str(&format!(" risk:       {}\n", p.scores.risk));
    out.push('\n');

    out.push_str("features:\n");
    out.push_str(&format!(" tx_count:    {}\n", p.features.tx_count));
    out.push_str(&format!(" active_days: {}\n", p.features.active_days));
    out.push_str(&format!(" fail_rate:   {:.2}\n", p.features.failure_rate));
    out.push_str(&format!(" swaps:       {}\n", p.features.swap_count));
    out.push_str(&format!(" transfers:   {}\n", p.features.transfer_count));
    out.push('\n');

    out
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn render_actions_table(actions: &[Action], indent: usize) -> String {
    if actions.is_empty() {
        return format!("{}actions[0]{{t,p,s,x,amt}}:\n", " ".repeat(indent));
    }
    let mut tbl = ToonTable::new("actions", vec!["t", "p", "s", "x", "amt"]);
    for a in actions {
        tbl.add_row(vec![
            a.t.clone(),
            a.p.clone(),
            abbrev(&a.s),
            a.x.as_deref().map(abbrev).unwrap_or_else(|| "-".into()),
            a.amt.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
        ]);
    }
    tbl.render(indent)
}

fn render_token_deltas_table(deltas: &[TokenDelta], indent: usize) -> String {
    if deltas.is_empty() {
        return format!("{}tokenDeltas[0]{{mint,owner,delta,dir}}:\n", " ".repeat(indent));
    }
    let mut tbl = ToonTable::new("tokenDeltas", vec!["mint", "owner", "delta", "dir"]);
    for d in deltas {
        tbl.add_row(vec![
            abbrev(&d.mint),
            abbrev(&d.owner),
            d.delta.clone(),
            format!("{:?}", d.direction).to_lowercase(),
        ]);
    }
    tbl.render(indent)
}

/// Abbreviate a pubkey/sig for display: first8...last8
fn abbrev(s: &str) -> String {
    if s.len() > 20 {
        format!("{}..{}", &s[..8], &s[s.len()-4..])
    } else {
        s.to_string()
    }
}
