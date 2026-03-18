/// Atlas Geyser Plugin — main GeyserPlugin implementation.
///
/// Only handles account updates. Transactions and block metadata are
/// intentionally skipped — they are handled by the Yellowstone gRPC
/// plugin that runs alongside this one.
use {
    crate::{
        config::AtlasGeyserConfig,
        token::{is_token_program, parse_token_account},
        worker::{AccountUpdate, WorkerPool},
    },
    agave_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPlugin, GeyserPluginError, ReplicaAccountInfoVersions,
        ReplicaBlockInfoVersions, ReplicaTransactionInfoVersions, Result, SlotStatus,
    },
    log::*,
    solana_sdk::pubkey::Pubkey,
    std::{fs, sync::atomic::{AtomicU64, Ordering}, sync::Arc},
};

#[derive(Default)]
pub struct AtlasGeyserPlugin {
    pool:         Option<WorkerPool>,
    log_every:    u64,
    update_count: Arc<AtomicU64>,
}

impl std::fmt::Debug for AtlasGeyserPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AtlasGeyserPlugin")
    }
}

impl GeyserPlugin for AtlasGeyserPlugin {
    fn name(&self) -> &'static str {
        "AtlasGeyserPlugin"
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    fn on_load(&mut self, config_file: &str, _is_reload: bool) -> Result<()> {
        solana_logger::setup_with_default("info");
        info!("atlas-geyser: loading from {config_file}");

        let contents = fs::read_to_string(config_file).map_err(|e| {
            GeyserPluginError::ConfigFileReadError { msg: e.to_string() }
        })?;

        let cfg: AtlasGeyserConfig = serde_json::from_str(&contents).map_err(|e| {
            GeyserPluginError::ConfigFileReadError {
                msg: format!("invalid config JSON: {e}"),
            }
        })?;

        info!(
            "atlas-geyser: threads={} batch_size={} channel_capacity={}",
            cfg.threads, cfg.batch_size, cfg.channel_capacity
        );

        self.log_every = cfg.log_every;
        self.pool      = Some(WorkerPool::new(&cfg));

        info!("atlas-geyser: plugin loaded — writing to geyser_accounts + token_owner_map");
        Ok(())
    }

    fn on_unload(&mut self) {
        info!("atlas-geyser: unloading");
        // Workers hold the receiving side of the channel; dropping the pool
        // closes the sender and allows workers to drain and exit cleanly.
        self.pool = None;
    }

    // ── Account updates ───────────────────────────────────────────────────────

    fn update_account(
        &self,
        account: ReplicaAccountInfoVersions,
        slot:       u64,
        is_startup: bool,
    ) -> Result<()> {
        let pool = match &self.pool {
            Some(p) => p,
            None    => return Ok(()),
        };

        let info = match account {
            ReplicaAccountInfoVersions::V0_0_3(a) => a,
            _ => return Ok(()), // only V0_0_3 carries the full data we need
        };

        let pubkey = bs58_encode(info.pubkey);
        let owner  = bs58_encode(info.owner);

        // Parse SPL token mint + wallet owner if this is a token account.
        let (token_mint, token_owner) = if is_token_program(info.owner) {
            match parse_token_account(info.data) {
                Some((mint, wallet)) => (Some(mint), Some(wallet)),
                None                 => (None, None),
            }
        } else {
            (None, None)
        };

        let update = AccountUpdate {
            pubkey,
            lamports:    info.lamports,
            owner,
            executable:  info.executable,
            data:        info.data.to_vec(),
            slot:        slot as i64,
            is_startup,
            token_mint,
            token_owner,
        };

        pool.send_account(update);

        // Periodic progress log (uses only validator-thread-local counter so no contention).
        let n = self.update_count.fetch_add(1, Ordering::Relaxed) + 1;
        if n % self.log_every == 0 {
            let (acc, tok, dropped) = pool.stats();
            info!(
                "atlas-geyser: {n} callbacks received | {acc} accounts written | \
                 {tok} token maps | {dropped} dropped"
            );
        }

        Ok(())
    }

    // ── Slot status ───────────────────────────────────────────────────────────

    fn update_slot_status(
        &self,
        _slot:   u64,
        _parent: Option<u64>,
        _status: &SlotStatus,
    ) -> Result<()> {
        Ok(()) // no slot table — Atlas indexer handles this via Yellowstone
    }

    fn notify_end_of_startup(&self) -> Result<()> {
        if let Some(pool) = &self.pool {
            let (acc, tok, dropped) = pool.stats();
            info!(
                "atlas-geyser: startup complete — {acc} accounts, {tok} token maps, {dropped} dropped"
            );
        }
        Ok(())
    }

    // ── Transactions / blocks — intentionally no-op ───────────────────────────
    //
    // These are handled by the Yellowstone gRPC plugin running alongside
    // this one. Returning Ok(()) is the correct and minimal implementation.

    fn notify_transaction(
        &self,
        _tx:   ReplicaTransactionInfoVersions,
        _slot: u64,
    ) -> Result<()> {
        Ok(())
    }

    fn notify_block_metadata(&self, _block: ReplicaBlockInfoVersions) -> Result<()> {
        Ok(())
    }

    // ── Feature flags ─────────────────────────────────────────────────────────

    fn account_data_notifications_enabled(&self) -> bool {
        true // we want ALL accounts
    }

    fn transaction_notifications_enabled(&self) -> bool {
        false // Yellowstone handles this
    }
}

// ── Plugin entry point ────────────────────────────────────────────────────────

impl AtlasGeyserPlugin {
    pub fn new() -> Self {
        Self {
            pool:         None,
            log_every:    100_000,
            update_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
/// # Safety
/// Called by the validator to load the plugin. Returns a raw pointer to the plugin.
pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
    let plugin: Box<dyn GeyserPlugin> = Box::new(AtlasGeyserPlugin::new());
    Box::into_raw(plugin)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

#[inline]
fn bs58_encode(bytes: &[u8]) -> String {
    if bytes.len() == 32 {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Pubkey::from(arr).to_string()
    } else {
        // Fallback for non-pubkey byte slices (rare).
        hex::encode(bytes)
    }
}

