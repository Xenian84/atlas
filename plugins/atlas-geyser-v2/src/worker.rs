/// Parallel PostgreSQL writer.
///
/// Architecture
/// ───────────
/// • N worker threads, each with ONE long-lived postgres connection.
/// • A single bounded crossbeam channel feeds all workers via work-stealing.
/// • The validator thread calls `send_account` which does a non-blocking
///   `try_send`. If the channel is full the update is dropped with a warning —
///   the validator is NEVER blocked or slowed down.
/// • Each worker drains the channel in batches and flushes with a single
///   UNNEST upsert inside a transaction, so round-trips to postgres are minimal.
///
/// Atlas tables written
/// ────────────────────
///  geyser_accounts  — full account state (pubkey, lamports, owner, …)
///  token_owner_map  — SPL token → (mint, wallet owner) mapping
use {
    crate::config::AtlasGeyserConfig,
    crossbeam_channel::{bounded, Receiver, Sender, TrySendError},
    log::*,
    postgres::{Client, Config, NoTls},
    std::{
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc,
        },
        thread,
        time::Duration,
    },
};

// ── Public account-update message ────────────────────────────────────────────

#[derive(Debug)]
pub struct AccountUpdate {
    pub pubkey:      String,   // base-58
    pub lamports:    u64,
    pub owner:       String,   // base-58
    pub executable:  bool,
    pub data:        Vec<u8>,
    pub slot:        i64,
    pub is_startup:  bool,
    /// Parsed from data when owner is SPL Token / Token-2022
    pub token_mint:  Option<String>,
    pub token_owner: Option<String>,
}

// ── Handle handed to the plugin ───────────────────────────────────────────────

pub struct WorkerPool {
    sender:          Sender<AccountUpdate>,
    _threads:        Vec<thread::JoinHandle<()>>,
    total_accounts:  Arc<AtomicU64>,
    total_tokens:    Arc<AtomicU64>,
    total_dropped:   Arc<AtomicU64>,
}

impl WorkerPool {
    pub fn new(cfg: &AtlasGeyserConfig) -> Self {
        let (tx, rx) = bounded::<AccountUpdate>(cfg.channel_capacity);

        let total_accounts = Arc::new(AtomicU64::new(0));
        let total_tokens   = Arc::new(AtomicU64::new(0));
        let total_dropped  = Arc::new(AtomicU64::new(0));

        let mut threads = Vec::with_capacity(cfg.threads);
        for id in 0..cfg.threads {
            let rx_clone        = rx.clone();
            let conn_str        = cfg.connection_str.clone();
            let batch_size      = cfg.batch_size;
            let log_every       = cfg.log_every;
            let acc_counter     = total_accounts.clone();
            let tok_counter     = total_tokens.clone();

            let handle = thread::Builder::new()
                .name(format!("atlas-geyser-{id}"))
                .spawn(move || {
                    run_worker(id, conn_str, rx_clone, batch_size, log_every, acc_counter, tok_counter);
                })
                .expect("failed to spawn atlas-geyser worker");

            threads.push(handle);
        }

        WorkerPool { sender: tx, _threads: threads, total_accounts, total_tokens, total_dropped }
    }

    /// Non-blocking send. Returns false if the channel was full (update dropped).
    pub fn send_account(&self, update: AccountUpdate) -> bool {
        match self.sender.try_send(update) {
            Ok(_) => true,
            Err(TrySendError::Full(_)) => {
                let dropped = self.total_dropped.fetch_add(1, Ordering::Relaxed) + 1;
                if dropped % 10_000 == 1 {
                    warn!("atlas-geyser: channel full — dropped {} updates so far", dropped);
                }
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                error!("atlas-geyser: worker channel disconnected");
                false
            }
        }
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.total_accounts.load(Ordering::Relaxed),
            self.total_tokens.load(Ordering::Relaxed),
            self.total_dropped.load(Ordering::Relaxed),
        )
    }
}

// ── Worker thread ─────────────────────────────────────────────────────────────

fn run_worker(
    id:          usize,
    conn_str:    String,
    rx:          Receiver<AccountUpdate>,
    batch_size:  usize,
    log_every:   u64,
    acc_counter: Arc<AtomicU64>,
    tok_counter: Arc<AtomicU64>,
) {
    info!("atlas-geyser worker-{id}: starting");

    // Reconnect loop — if postgres dies we retry rather than crashing the validator.
    loop {
        match connect(&conn_str) {
            Err(e) => {
                error!("atlas-geyser worker-{id}: cannot connect to postgres: {e}. Retrying in 5s…");
                thread::sleep(Duration::from_secs(5));
            }
            Ok(mut client) => {
                info!("atlas-geyser worker-{id}: connected to postgres");
                if let Err(e) = write_loop(id, &mut client, &rx, batch_size, log_every, &acc_counter, &tok_counter) {
                    error!("atlas-geyser worker-{id}: write loop exited: {e}. Reconnecting…");
                    thread::sleep(Duration::from_secs(1));
                }
            }
        }
    }
}

fn connect(conn_str: &str) -> Result<Client, postgres::Error> {
    let cfg: Config = conn_str.parse()?;
    cfg.connect(NoTls)
}

fn write_loop(
    id:          usize,
    client:      &mut Client,
    rx:          &Receiver<AccountUpdate>,
    batch_size:  usize,
    log_every:   u64,
    acc_counter: &AtomicU64,
    tok_counter: &AtomicU64,
) -> Result<(), postgres::Error> {
    let mut batch: Vec<AccountUpdate> = Vec::with_capacity(batch_size);
    let mut local_accounts: u64 = 0;
    let mut local_tokens:   u64 = 0;

    loop {
        // Block until at least one item arrives, then drain up to batch_size.
        match rx.recv() {
            Err(_) => break, // channel closed
            Ok(first) => batch.push(first),
        }

        // Non-blocking drain for the rest of the batch.
        while batch.len() < batch_size {
            match rx.try_recv() {
                Ok(u)  => batch.push(u),
                Err(_) => break,
            }
        }

        // Flush the batch inside a single transaction.
        let n_acc = batch.len();
        let n_tok = batch.iter().filter(|u| u.token_mint.is_some()).count();

        flush_batch(client, &batch)?;
        batch.clear();

        local_accounts += n_acc as u64;
        local_tokens   += n_tok as u64;

        let total = acc_counter.fetch_add(n_acc as u64, Ordering::Relaxed) + n_acc as u64;
        tok_counter.fetch_add(n_tok as u64, Ordering::Relaxed);

        if total / log_every > (total - n_acc as u64) / log_every {
            info!("atlas-geyser worker-{id}: {total} accounts written ({local_tokens} token mappings this session)");
        }

        let _ = (local_accounts, local_tokens); // suppress unused warnings
    }

    Ok(())
}

// ── Batch flush ───────────────────────────────────────────────────────────────

/// Upserts one batch using UNNEST — a single round-trip per batch.
fn flush_batch(client: &mut Client, batch: &[AccountUpdate]) -> Result<(), postgres::Error> {
    // ── geyser_accounts ──────────────────────────────────────────────────────
    // Columns:  pubkey TEXT, lamports NUMERIC, owner TEXT, executable BOOL,
    //           data BYTEA, slot BIGINT, is_startup BOOL, written_at TIMESTAMPTZ
    let mut pubkeys:     Vec<&str>  = Vec::with_capacity(batch.len());
    let mut lamports:    Vec<String>= Vec::with_capacity(batch.len());
    let mut owners:      Vec<&str>  = Vec::with_capacity(batch.len());
    let mut executables: Vec<bool>  = Vec::with_capacity(batch.len());
    let mut datas:       Vec<&[u8]> = Vec::with_capacity(batch.len());
    let mut slots:       Vec<i64>   = Vec::with_capacity(batch.len());
    let mut startups:    Vec<bool>  = Vec::with_capacity(batch.len());

    for u in batch {
        pubkeys.push(&u.pubkey);
        lamports.push(u.lamports.to_string());
        owners.push(&u.owner);
        executables.push(u.executable);
        datas.push(&u.data);
        slots.push(u.slot);
        startups.push(u.is_startup);
    }

    let mut tx = client.transaction()?;

    // UNNEST upsert for geyser_accounts
    tx.execute(
        "INSERT INTO geyser_accounts
              (pubkey, lamports, owner, executable, data, slot, is_startup, written_at)
         SELECT t.pubkey,
                t.lamports::numeric,
                t.owner,
                t.executable,
                t.data,
                t.slot,
                t.is_startup,
                now()
         FROM UNNEST(
                $1::text[],
                $2::text[],
                $3::text[],
                $4::bool[],
                $5::bytea[],
                $6::bigint[],
                $7::bool[]
              ) AS t(pubkey, lamports, owner, executable, data, slot, is_startup)
         ON CONFLICT (pubkey) DO UPDATE SET
             lamports   = EXCLUDED.lamports,
             owner      = EXCLUDED.owner,
             executable = EXCLUDED.executable,
             data       = EXCLUDED.data,
             slot       = EXCLUDED.slot,
             is_startup = EXCLUDED.is_startup,
             written_at = now()
         WHERE geyser_accounts.slot <= EXCLUDED.slot",
        &[&pubkeys, &lamports, &owners, &executables, &datas, &slots, &startups],
    )?;

    // ── token_owner_map ──────────────────────────────────────────────────────
    // Columns:  token_account TEXT, mint TEXT, owner TEXT, updated_at TIMESTAMPTZ
    let token_updates: Vec<(&str, &str, &str)> = batch
        .iter()
        .filter_map(|u| {
            if let (Some(mint), Some(wallet)) = (&u.token_mint, &u.token_owner) {
                Some((u.pubkey.as_str(), mint.as_str(), wallet.as_str()))
            } else {
                None
            }
        })
        .collect();

    if !token_updates.is_empty() {
        let token_accounts: Vec<&str> = token_updates.iter().map(|(ta, _, _)| *ta).collect();
        let mints:          Vec<&str> = token_updates.iter().map(|(_, m, _)| *m).collect();
        let wallets:        Vec<&str> = token_updates.iter().map(|(_, _, w)| *w).collect();

        tx.execute(
            "INSERT INTO token_owner_map (token_account, mint, owner, updated_at)
             SELECT t.token_account, t.mint, t.owner, now()
             FROM UNNEST($1::text[], $2::text[], $3::text[])
                  AS t(token_account, mint, owner)
             ON CONFLICT (token_account) DO UPDATE SET
                 mint       = EXCLUDED.mint,
                 owner      = EXCLUDED.owner,
                 updated_at = now()",
            &[&token_accounts, &mints, &wallets],
        )?;
    }

    tx.commit()
}
